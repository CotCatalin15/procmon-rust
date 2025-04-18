use std::{cell::RefCell, ops::Index, sync::Arc, thread::JoinHandle};

use flume::Sender;
use rayon::{ThreadPool, ThreadPoolBuilder};

use crate::{
    event_storage::EventStorage,
    filters::SimpleFilter,
    notifier::{NotificationBus, NotificationBusSubscriber},
};

use super::IndexData;

struct Inner {}

pub struct EventFilterService {
    subscriber: NotificationBusSubscriber,
}

thread_local! {
    static FILTERS : RefCell<Vec<SimpleFilter>> = RefCell::new(Vec::new());
}

impl EventFilterService {
    pub fn new(
        bus: NotificationBus,
        index_sender: Sender<IndexData>,
        filter_threads: usize,
        storage: Arc<EventStorage>,
        filters: Vec<SimpleFilter>,
    ) -> Self {
        let filter_pool = ThreadPoolBuilder::new()
            .thread_name(|index| format!("Filter thread_{}", index))
            .num_threads(filter_threads)
            .build()
            .unwrap();

        bus.notify();

        filter_pool.broadcast(|_context| {
            FILTERS.set(filters.clone());
        });

        let storage_clone = storage.clone();
        let mut previous_size = 0;
        Self {
            subscriber: bus.subscribe(move || {
                Self::load_balancing_routine(
                    storage_clone.clone(),
                    &mut previous_size,
                    &filter_pool,
                    &index_sender,
                );
            }),
        }
    }

    fn load_balancing_routine(
        storage: Arc<EventStorage>,
        previous_size: &mut usize,
        filter_pool: &ThreadPool,
        sender: &Sender<IndexData>,
    ) {
        const MAX_BATCH_SIZE: usize = 512;

        let new_len = storage.len();
        tracing::debug!("Received storage notification, new_len: {}", new_len);

        let mut dif = new_len - *previous_size;

        if dif == 0 {
            return;
        }

        let mut start = *previous_size;
        while dif > 0 {
            let filter_size = usize::min(MAX_BATCH_SIZE, dif);

            let storage_clone = storage.clone();
            let sender_clone = sender.clone();

            filter_pool.spawn(move || {
                FILTERS.with_borrow(|filters| {
                    Self::filter_routine(
                        sender_clone,
                        storage_clone,
                        start,
                        start + filter_size,
                        &filters,
                    );
                });
            });

            start += MAX_BATCH_SIZE;
            dif -= filter_size;
        }

        *previous_size = new_len;
    }

    fn filter_routine(
        sender: Sender<IndexData>,
        storage: Arc<EventStorage>,
        start_range: usize,
        end_range: usize,
        filters: &[SimpleFilter],
    ) {
        tracing::debug!("Filtering {} -> {}", start_range, end_range);

        for index in start_range..end_range {
            let event = storage.read_event(index);

            let mut visibile_event = true;
            for filter in filters {
                if !filter.matches(event) {
                    visibile_event = false;
                    break;
                }
            }

            if visibile_event {
                let _ = sender.send(IndexData {
                    event_timestamp: event.event.date,
                    event_index: index,
                });
            }
        }
    }
}

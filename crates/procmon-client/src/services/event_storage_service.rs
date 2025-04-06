use std::{
    sync::Arc,
    thread::{spawn, JoinHandle},
};

use flume::Receiver;
use kmum_common::KmMessage;
use rayon::ThreadBuilder;

use crate::{
    event_storage::{self, EventStorage},
    notifier::NotificationBus,
};

pub struct EventStorageService {
    storage_threads: Vec<JoinHandle<()>>,
}

impl EventStorageService {
    pub fn new(
        event_receiver: Receiver<KmMessage>,
        bus: NotificationBus,
        storage: Arc<EventStorage>,
        num_storage_threads: usize,
    ) -> Self {
        let mut workers = Vec::with_capacity(num_storage_threads);

        for _ in 0..num_storage_threads {
            let storage_clone = storage.clone();
            let bus_clone = bus.clone();
            let events_clone = event_receiver.clone();

            workers.push(spawn(move || {
                Self::storage_thread_worker(events_clone, bus_clone, storage_clone);
            }));
        }

        Self {
            storage_threads: workers,
        }
    }

    fn storage_thread_worker(
        event_receiver: Receiver<KmMessage>,
        bus: NotificationBus,
        storage: Arc<EventStorage>,
    ) {
        let mut batch = Vec::with_capacity(128);

        tracing::info!("Starting storage thread");
        while let Ok(event) = event_receiver.recv() {
            batch.push(event);

            for _ in 0..127 {
                if let Ok(event) = event_receiver.try_recv() {
                    batch.push(event);
                } else {
                    break;
                }
            }

            tracing::debug!("Storing {} new events", batch.len());
            storage.push_events(batch.drain(..));
            bus.notify();
        }

        tracing::info!("Storage thread exit");
    }
}

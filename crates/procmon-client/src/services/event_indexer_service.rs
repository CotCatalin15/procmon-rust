use std::{
    net,
    ops::Deref,
    sync::Arc,
    thread::{spawn, JoinHandle},
};

use egui::mutex::RwLock;
use flume::{bounded, Receiver, Sender};

use crate::{event_storage::EventStorage, notifier::NotificationBus};

#[derive(Copy, Clone, Debug)]
pub struct IndexData {
    pub event_timestamp: u64,
    pub event_index: usize,
}

pub struct EventIndexerService {
    stop_sender: Sender<()>,

    storage: Arc<RwLock<Vec<IndexData>>>,
    index_thread: JoinHandle<()>,
}

impl EventIndexerService {
    pub fn new(receiver: Receiver<IndexData>) -> Self {
        let (stop_sender, stop_receiver) = bounded(1);

        let storage = Arc::new(RwLock::new(Vec::with_capacity(100_000)));

        let storage_clone = storage.clone();
        let handle = spawn(move || {
            Self::indexing_routine(storage_clone.deref(), receiver, stop_receiver);
        });

        Self {
            storage,
            stop_sender,
            index_thread: handle,
        }
    }

    pub fn len(&self) -> usize {
        self.storage.read().len()
    }

    pub fn collect_indicies_into(&self, start: usize, end: usize, collection: &mut Vec<IndexData>) {
        self.storage.read()[start..end]
            .iter()
            .collect_into(collection);
    }

    fn indexing_routine(
        storage: &RwLock<Vec<IndexData>>,
        index_data_receiver: Receiver<IndexData>,
        stop_receiver: Receiver<()>,
    ) {
        const MAX_RECEIVE_SIZE: usize = 1024 * 5;
        let mut index_buffer = Vec::with_capacity(MAX_RECEIVE_SIZE);

        tracing::info!("Starting index thread");
        loop {
            let new_index = flume::Selector::new()
                .recv(&stop_receiver, |_| None)
                .recv(&index_data_receiver, |data| Some(data))
                .wait();

            if new_index.is_none() {
                break;
            }
            let new_index = match new_index.unwrap() {
                Ok(index) => index,
                Err(_) => break,
            };
            index_buffer.push(new_index);

            index_data_receiver
                .try_iter()
                .take(MAX_RECEIVE_SIZE - 1)
                .collect_into(&mut index_buffer);

            tracing::debug!("Indexing {} new events", index_buffer.len());

            let pos = {
                let ref_timestamp = index_buffer.first().unwrap().event_timestamp;
                storage
                    .read()
                    .binary_search_by(|event| event.event_timestamp.cmp(&ref_timestamp))
                    .unwrap_or_else(|error| error)
            };

            let mut guard = storage.write();

            guard.extend(index_buffer.drain(..));
            guard.as_mut_slice()[pos..]
                .sort_by(|lhs, rhs| lhs.event_timestamp.cmp(&rhs.event_timestamp));
        }

        tracing::info!("Stopping index thread");
    }
}

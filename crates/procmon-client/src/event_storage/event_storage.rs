use egui::mutex::RwLock;
use kmum_common::KmMessage;

use crate::container::ConcurrentChunkVec;

pub struct EventStorage {
    storage: ConcurrentChunkVec<KmMessage>,
}

impl EventStorage {
    pub fn new() -> Self {
        Self {
            storage: ConcurrentChunkVec::new(100_000),
        }
    }

    pub fn push_events(&self, mut events: impl ExactSizeIterator<Item = KmMessage>) {
        self.storage
            .acquire_write_inplace(events.len(), |_| events.next().unwrap());
    }

    pub fn read_event<'a>(&'a self, index: usize) -> &'a KmMessage {
        self.storage.get(index).unwrap()
    }

    pub fn len(&self) -> usize {
        self.storage.len()
    }
}

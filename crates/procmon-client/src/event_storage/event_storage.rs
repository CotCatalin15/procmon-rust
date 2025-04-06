use egui::mutex::RwLock;
use kmum_common::KmMessage;

pub struct EventStorage {
    //implement using use a Vec<Slots> that does not invalidate a reference to an item
    storage: RwLock<Vec<KmMessage>>,
}

impl EventStorage {
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(Vec::with_capacity(500_000)),
        }
    }

    //Returns the write offset
    pub fn push_events(&self, events: impl Iterator<Item = KmMessage>) {
        self.storage.write().extend(events);
    }

    pub fn read_event<F>(&self, index: usize, reader: F)
    where
        F: FnOnce(&KmMessage),
    {
        self.storage.read().get(index).inspect(|event| {
            reader(*event);
        });
    }

    pub fn len(&self) -> usize {
        self.storage.read().len()
    }
}

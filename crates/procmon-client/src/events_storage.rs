use std::sync::Arc;

use egui::mutex::Mutex;
use kmum_common::KmMessage;

#[derive(Default, Clone)]
pub struct EventStorage {
    events: Arc<Mutex<Vec<KmMessage>>>,
}

impl EventStorage {
    pub fn push_received(&self, iter: &mut impl Iterator<Item = KmMessage>) {
        let mut guard = self.events.lock();

        for event in iter {
            guard.push(event);
        }
    }

    pub fn read<F: FnOnce(&KmMessage)>(&self, index: usize, f: F) {
        let guard = self.events.lock();
        if let Some(evnt) = guard.get(index) {
            f(evnt);
        }
    }

    pub fn len(&self) -> usize {
        self.events.lock().len()
    }
}

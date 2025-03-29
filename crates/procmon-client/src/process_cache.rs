use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use egui::mutex::RwLock;
use kmum_common::{process::UniqueProcessId, serializable_ntstring::SerializableNtString};
use tokio::{
    sync::mpsc::{channel, Receiver, Sender},
    task::spawn_blocking,
};

pub struct ProcessCache {
    cache: RwLock<HashMap<UniqueProcessId, Option<SerializableNtString>>>,
    sender: Sender<UniqueProcessId>,
}

impl ProcessCache {
    pub fn new<Q>(query_cb: Q) -> Arc<Self>
    where
        Q: Fn(UniqueProcessId) -> Option<SerializableNtString> + 'static + Send,
    {
        let (sender, recv) = channel(1024);

        let cache = Arc::new(Self {
            cache: RwLock::default(),
            sender,
        });

        let cache_clone = Arc::downgrade(&cache);
        spawn_blocking(move || Self::internal_worker(cache_clone, query_cb, recv));

        cache
    }

    pub fn try_get_and<F>(&self, uid: UniqueProcessId, cb: F) -> bool
    where
        F: FnOnce(&Option<SerializableNtString>),
    {
        let guard = self.cache.read();
        if let Some(hit) = guard.get(&uid) {
            cb(hit);
            true
        } else {
            let _ = self.sender.blocking_send(uid);
            false
        }
    }

    fn internal_worker<Q>(weak_self: Weak<Self>, query: Q, mut receiver: Receiver<UniqueProcessId>)
    where
        Q: Fn(UniqueProcessId) -> Option<SerializableNtString> + 'static + Send,
    {
        let mut data = Vec::with_capacity(16);
        let mut cached_names: Vec<(u64, Option<SerializableNtString>)> = Vec::new();

        loop {
            cached_names.clear();
            data.clear();

            let size = receiver.blocking_recv_many(&mut data, 16);
            if size == 0 {
                break;
            }

            let cache = weak_self.upgrade();
            if cache.is_none() {
                break;
            }

            let cache = cache.unwrap();

            {
                let read_guard = cache.cache.read();
                cached_names.extend(data.iter().filter(|id| !read_guard.contains_key(id)).map(
                    |id| {
                        let process_info = query(*id);
                        (*id, process_info)
                    },
                ));
            }

            {
                let mut guard = cache.cache.write();
                guard.extend(cached_names.drain(..));
            }
        }
    }
}

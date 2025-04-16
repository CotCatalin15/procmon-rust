use std::{sync::Arc, thread::JoinHandle};

use dashmap::DashMap;
use egui::mutex::RwLock;
use kmum_common::{
    process::{ProcessInformation, UniqueProcessId},
    KmReplyMessage, UmSendMessage,
};
use nt_string::unicode_string::NtUnicodeString;
use tokio::sync::mpsc::{Receiver, Sender};

struct Inner {
    cache: DashMap<UniqueProcessId, Option<Arc<ProcessCacheInformation>>>,
}

pub struct ProcessManager {
    inner: Arc<Inner>,
    worker: JoinHandle<()>,
    request_sender: Sender<UniqueProcessId>,
}

pub enum ProcessCacheEntry {
    Hit(Option<Arc<ProcessCacheInformation>>),
    Miss,
}

pub struct ProcessCacheInformation {
    exe_name: NtUnicodeString,
    base_info: ProcessInformation,
}

unsafe impl Send for ProcessCacheInformation {}
unsafe impl Sync for ProcessCacheInformation {}

impl ProcessManager {
    pub fn new<Q>(query_cb: Q) -> Self
    where
        Q: Fn(UniqueProcessId) -> Option<ProcessInformation> + Send + 'static,
    {
        let (sender, receiver) = tokio::sync::mpsc::channel(128);
        let inner = Arc::new(Inner {
            cache: DashMap::default(),
        });

        let inner_clone = inner.clone();
        let worker = std::thread::spawn(move || {
            Self::worker_routine(query_cb, inner_clone, receiver);
        });

        Self {
            request_sender: sender,
            inner: inner,
            worker,
        }
    }

    pub fn try_get_async(&self, uid: UniqueProcessId) -> ProcessCacheEntry {
        match self.inner.cache.get(&uid) {
            Some(info) => ProcessCacheEntry::Hit(info.value().clone()),
            None => {
                self.request_sender.blocking_send(uid);
                ProcessCacheEntry::Miss
            }
        }
    }

    fn worker_routine<Q>(query_cb: Q, inner: Arc<Inner>, mut receiver: Receiver<UniqueProcessId>)
    where
        Q: Fn(UniqueProcessId) -> Option<ProcessInformation> + Send + 'static,
    {
        let mut uid_buffer = Vec::default();

        tracing::info!("Starting process cache routing");
        loop {
            let size = receiver.blocking_recv_many(&mut uid_buffer, 128);
            if size == 0 {
                tracing::info!("Stopping process cache routing");
                return;
            }

            for uid in uid_buffer.drain(..size) {
                if inner.cache.contains_key(&uid) {
                    continue;
                }

                let info = query_cb(uid);
                inner.cache.insert(
                    uid,
                    info.map(|value| Arc::new(ProcessCacheInformation::new(value))),
                );
            }
        }
    }
}

impl ProcessCacheInformation {
    fn new(info: ProcessInformation) -> Self {
        let nt_string = info.path.0.as_u16str();

        let mut pos = None;
        for (i, c) in nt_string.char_indices_lossy() {
            if c == '\\' || c == '/' {
                pos = Some(i);
            }
        }

        let path = if let Some(pos) = pos {
            nt_string.split_at(pos + 1).1
        } else {
            nt_string
        };

        let process_name =
            NtUnicodeString::try_from(path).unwrap_or_else(|_| NtUnicodeString::new());

        Self {
            base_info: info,
            exe_name: process_name,
        }
    }

    pub fn get_info(&self) -> &ProcessInformation {
        &self.base_info
    }

    pub fn get_process_name(&self) -> &NtUnicodeString {
        &self.exe_name
    }
}

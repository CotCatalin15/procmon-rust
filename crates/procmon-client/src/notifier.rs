use std::{
    ops::DerefMut,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use egui::mutex::Mutex;

struct InternalSub {
    id: usize,
    cb: Box<dyn FnMut() + Send + 'static>,
}

#[derive(Default)]
struct InternalSubData {
    last_id: usize,
    subscribers: Vec<InternalSub>,
}

struct Inner {
    binary_semaphore: AtomicBool,
    notification_pool: rayon::ThreadPool,
    data: Mutex<InternalSubData>,
}

#[derive(Clone)]
pub struct NotificationBus {
    inner: Arc<Inner>,
}

pub struct NotificationBusSubscriber {
    id: usize,
    inner: Arc<Inner>,
}

impl NotificationBus {
    pub fn new() -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        Self {
            inner: Arc::new(Inner {
                notification_pool: pool,
                binary_semaphore: AtomicBool::new(true),
                data: Mutex::default(),
            }),
        }
    }

    pub fn subscribe<F>(&self, callback: F) -> NotificationBusSubscriber
    where
        F: FnMut() + Send + 'static,
    {
        let mut guard = self.inner.data.lock();
        let id = guard.last_id;
        guard.last_id += 1;

        guard.subscribers.push(InternalSub {
            id: id,
            cb: Box::new(callback),
        });

        NotificationBusSubscriber {
            id,
            inner: self.inner.clone(),
        }
    }

    pub fn notify(&self) {
        let permit = self.inner.binary_semaphore.compare_exchange(
            true,
            false,
            Ordering::Acquire,
            Ordering::Relaxed,
        );

        if permit.is_err() {
            return;
        }

        let inner_clone = self.inner.clone();
        self.inner.notification_pool.spawn(move || {
            inner_clone.binary_semaphore.store(true, Ordering::Release);

            inner_clone
                .data
                .lock()
                .subscribers
                .iter_mut()
                .for_each(|sub| {
                    (sub.cb)();
                });
        });
    }
}

impl Drop for NotificationBusSubscriber {
    fn drop(&mut self) {
        let mut guard = self.inner.data.lock();

        let sub_position = guard
            .subscribers
            .iter()
            .position(|sub| sub.id == self.id)
            .unwrap();

        guard.subscribers.swap_remove(sub_position);
    }
}

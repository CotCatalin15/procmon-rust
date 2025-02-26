use std::{
    sync::Arc,
    thread::{spawn, JoinHandle},
};

use kmum_common::{
    get_communication_port_name, MAX_KM_MESSAGE_RECEIVE_SIZE, MAX_UM_REPLY_MESSAGE_SIZE,
};
use windows_sys::Win32::{
    Foundation::{STATUS_SUCCESS, STATUS_UNSUCCESSFUL, WAIT_OBJECT_0},
    System::{
        Threading::{WaitForMultipleObjects, INFINITE},
        IO::GetOverlappedResult,
    },
};

use crate::win::{constatns::WAIT_OBJECT_1, event::Event, overlapped::Overlapped};

use super::{
    parsed::{FilterMessageBuffer, FilterReplyBuffer},
    raw_communication::RawCommunication,
    CommunicationError,
};

pub trait FilterBufferHandler: Send + Sync + 'static {
    fn handle_buffer(
        &self,
        receive_buffer: &[u8],
        reply_buffer: &mut [u8],
    ) -> anyhow::Result<(), CommunicationError>;
}

#[allow(dead_code)]
pub struct Dispatcher<Handler: FilterBufferHandler> {
    inner: Arc<DispatcherInner<Handler>>,
    workers: Vec<Worker>,
}

impl<Handler: FilterBufferHandler> Dispatcher<Handler> {
    pub fn new(num_threads: u32, handler: Handler) -> Self {
        let inner = Arc::new(DispatcherInner::new(handler));
        let mut workers = Vec::new();

        for _ in 0..num_threads {
            workers.push(Worker::new(inner.clone()));
        }

        Self { workers, inner }
    }

    pub fn send_message(
        &self,
        buffer: &[u8],
        output: Option<&mut [u8]>,
    ) -> anyhow::Result<u32, CommunicationError> {
        self.inner.raw_communication.send_buffer(buffer, output)
    }
}

struct DispatcherInner<Handler: FilterBufferHandler> {
    raw_communication: RawCommunication,
    stop_event: Event,
    handler: Handler,
}

impl<Handler: FilterBufferHandler> DispatcherInner<Handler> {
    fn new(handler: Handler) -> Self {
        Self {
            handler,
            raw_communication: RawCommunication::new(get_communication_port_name().as_slice())
                .unwrap(),
            stop_event: Event::new().unwrap(),
        }
    }
}

struct Worker {
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new<Handler: FilterBufferHandler>(inner: Arc<DispatcherInner<Handler>>) -> Self {
        Self {
            handle: Some(spawn(move || Self::worker_routine(inner))),
        }
    }

    fn worker_routine<Handler: FilterBufferHandler>(inner: Arc<DispatcherInner<Handler>>) {
        let mut overlapped = Box::pin(Overlapped::new().unwrap());
        let handles = [inner.stop_event.handle(), overlapped.ov().hEvent];

        let mut send_buffer = FilterMessageBuffer::new(MAX_KM_MESSAGE_RECEIVE_SIZE);
        let mut reply_buffer = FilterReplyBuffer::new(MAX_UM_REPLY_MESSAGE_SIZE);

        loop {
            let status = unsafe {
                inner
                    .raw_communication
                    .get_message_overlapped_raw(send_buffer.mut_buffer(), overlapped.mut_ov())
            };

            if status.is_err() {
                panic!(
                    "Failed to receive km message status: {:#?}",
                    status.unwrap_err()
                );
            }

            let status =
                unsafe { WaitForMultipleObjects(2, handles.as_ptr(), false as _, INFINITE) };
            match status {
                WAIT_OBJECT_0 => return,
                WAIT_OBJECT_1 => {}
                _ => panic!(
                    "Unknown waiting result from WaitForMultipleObjects: {:x}",
                    status
                ),
            }

            let message_size = unsafe {
                let mut transfered: u32 = 0;
                if 0 == GetOverlappedResult(
                    inner.raw_communication.handle(),
                    overlapped.ov(),
                    &mut transfered,
                    false as _,
                ) {
                    panic!("GetOverlappedResult returned false");
                }

                transfered
            } as usize;

            {
                let send_parsed = send_buffer.as_parsed(message_size);
                let mut reply_parsed = reply_buffer.as_parsed();

                let result = inner
                    .handler
                    .handle_buffer(send_parsed.buffer, reply_parsed.buffer);

                match result {
                    Ok(_) => reply_parsed.construct_reply(&send_parsed, STATUS_SUCCESS),
                    Err(_) => reply_parsed.construct_reply(&send_parsed, STATUS_UNSUCCESSFUL),
                };

                let _ = unsafe {
                    inner
                        .raw_communication
                        .reply_message_raw(reply_buffer.as_buffer())
                }
                .inspect_err(|e| match e {
                    CommunicationError::NoWaiterPresent => {}
                    _ => panic!("Failed to send raw message: {:#?}", e),
                });
            }
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let worker = self.handle.take();
        let _ = worker.unwrap().join();
    }
}

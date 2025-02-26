use core::{ops::DerefMut, time::Duration};

use kmum_common::{KmMessage, KmReplyMessage, UmSendMessage, MAX_KM_MESSAGE_RECEIVE_SIZE};
use nt_string::unicode_string::NtUnicodeStr;
use wdrf::minifilter::communication::client_communication::{
    FltClientCommunication, FltCommunicationCallback,
};
use wdrf_std::{
    boxed::{Box, BoxExt},
    collections::vec_deq::{VecDeque, VecDequeCreate, VecDequeExt},
    constants::PoolFlags,
    kmalloc::{GlobalKernelAllocator, MemoryTag, TaggedObject},
    slice::tracked_slice::SeekFrom,
    sync::{
        arc::{Arc, ArcExt},
        InStackLockHandle, StackSpinMutex,
    },
    sys::{
        event::{EventType, KeEvent},
        WaitResponse, WaitableObject,
    },
    thread::{spawn, JoinHandle},
    time::Timeout,
    traits::DispatchSafe,
    vec::{Vec, VecCreate, VecExt},
};

use super::CommunicationError;

pub trait MessagingCallback {
    fn on_client(&self) -> anyhow::Result<()>;
    fn on_message(
        &self,
        message: &UmSendMessage,
    ) -> anyhow::Result<Option<KmReplyMessage>, CommunicationError>;
    fn on_disconnect(&self);
}

pub struct AsyncMessaging {
    num_workers: usize,
    workers: Vec<Worker>,
}
unsafe impl Send for AsyncMessaging {}
unsafe impl Sync for AsyncMessaging {}

impl AsyncMessaging {
    pub fn try_create<CB: MessagingCallback + TaggedObject + 'static>(
        num_workers: usize,
        callback: CB,
        name: NtUnicodeStr,
    ) -> anyhow::Result<Self, CommunicationError> {
        let callback = MessagingPortCallback::try_new(callback)?;
        let port_communication = FltClientCommunication::new(callback, name)
            .inspect_err(|e| {
                maple::error!("Failed to create flt client communication: {:#?}", e);
            })
            .map_err(|_| CommunicationError::PortError)?;

        let communication = Arc::try_create_in(
            port_communication,
            GlobalKernelAllocator::new(
                MemoryTag::new_from_bytes(b"asco"),
                PoolFlags::POOL_FLAG_NON_PAGED,
            ),
        )
        .map_err(|_| CommunicationError::NotEnoughMemory)?;

        let mut workers = Vec::create();
        workers
            .try_reserve(num_workers)
            .map_err(|_| CommunicationError::NotEnoughMemory)?;

        for _ in 0..num_workers {
            let worker = Worker::try_create(communication.clone())?;

            workers.push(worker);
        }

        Ok(Self {
            num_workers,
            workers,
        })
    }

    pub fn try_emplace_event(&self, message: KmMessage) -> anyhow::Result<(), CommunicationError> {
        let worker_id = (message.common.thread_id as usize) % self.num_workers;
        let worker: &Worker = self.workers.get(worker_id).unwrap();

        worker.try_push_event(message)
    }
}

struct MessagingPortCallback {
    callback: Box<dyn MessagingCallback>,
}

unsafe impl Send for MessagingPortCallback {}
unsafe impl Sync for MessagingPortCallback {}

impl MessagingPortCallback {
    fn try_new<CB: Sized + MessagingCallback + TaggedObject + 'static>(
        callback: CB,
    ) -> anyhow::Result<Self, CommunicationError> {
        let callback =
            Box::try_create(callback).map_err(|_| CommunicationError::NotEnoughMemory)?;

        Ok(Self { callback })
    }
}

impl FltCommunicationCallback for MessagingPortCallback {
    fn connect(&self, _buffer: Option<&[u8]>) -> anyhow::Result<()> {
        self.callback.on_client()
    }

    fn disconnect(&self) {
        self.callback.on_disconnect();
    }

    fn message(
        &self,
        input: &[u8],
        output: Option<&mut wdrf_std::slice::tracked_slice::TrackedSlice>,
    ) -> anyhow::Result<()> {
        let result = &postcard::from_bytes(input);

        match result {
            Ok(receive) => {
                let reply = self
                    .callback
                    .on_message(receive)
                    .map_err(|_| anyhow::Error::msg("On message failed"))?;

                if output.is_none() || reply.is_none() {
                    Ok(())
                } else {
                    let reply = reply.unwrap();
                    let output = output.unwrap();

                    let len = postcard::to_slice(&reply, output.as_slice_mut())
                        .map_err(|_| anyhow::Error::msg("Failed to trasform output to slice"))
                        .map(|buf| buf.len())?;

                    output.seek(SeekFrom::Start(len));
                    Ok(())
                }
            }
            Err(_) => Err(anyhow::Error::msg("Failed to parse input")),
        }
    }
}

pub struct DummyKmMessage(KmMessage);
unsafe impl DispatchSafe for DummyKmMessage {}
impl TaggedObject for DummyKmMessage {}

struct WorkerInternal {
    //Todo: Maybe try make it a Box for faster performance
    items: StackSpinMutex<VecDeque<DummyKmMessage>>,
    stop_event: KeEvent,
}
unsafe impl Send for WorkerInternal {}
unsafe impl Sync for WorkerInternal {}

impl TaggedObject for WorkerInternal {}

struct Worker {
    #[allow(unused)]
    handle: JoinHandle<()>,
    internal: Arc<WorkerInternal>,
}

impl TaggedObject for Worker {}

impl Worker {
    pub fn try_create(
        communication: Arc<FltClientCommunication<MessagingPortCallback>>,
    ) -> anyhow::Result<Self, CommunicationError> {
        let internal = Arc::try_create(WorkerInternal {
            items: StackSpinMutex::new(VecDeque::create()),
            stop_event: unsafe { KeEvent::new() },
        })
        .map_err(|_| CommunicationError::NotEnoughMemory)?;

        internal.stop_event.init(EventType::Notification, false);

        let internal_clone = internal.clone();
        let handle = spawn(move || {
            Worker::worker_routine(communication, internal_clone);
        })
        .map_err(|_| CommunicationError::NotEnoughMemory)?;

        Ok(Self { handle, internal })
    }

    fn worker_routine(
        communication: Arc<FltClientCommunication<MessagingPortCallback>>,
        internal: Arc<WorkerInternal>,
    ) {
        let mut buffer: Vec<u8> = Vec::create();
        let communication = communication.as_ref();
        let internal = internal.as_ref();

        buffer
            .try_resize(MAX_KM_MESSAGE_RECEIVE_SIZE, 0)
            .expect("Failed to resize worker buffer");

        //Send only every 30ms or when the buffer is 75% full

        let mut items: VecDeque<DummyKmMessage> = VecDeque::create();
        loop {
            let result = internal.stop_event.wait_for(Duration::from_millis(15));
            match result {
                WaitResponse::Object(_) | WaitResponse::Success => {
                    return;
                }
                WaitResponse::Timeout => {}
                _ => panic!("Unknown wait response"),
            }

            {
                let handle = InStackLockHandle::new();
                let mut guard = internal.items.lock(&handle);

                core::mem::swap(guard.deref_mut(), &mut items);
            }

            let mut offset = 0;
            let mut serliazed_items_count = 0;
            for item in items.drain(..).map(|item| item.0) {
                match postcard::to_slice(&item, &mut buffer[offset..]) {
                    Ok(serialize_slice) => {
                        offset += serialize_slice.len();
                        serliazed_items_count += 1;
                    }
                    Err(_) => {
                        maple::info!(
                            "Flushing {serliazed_items_count} serialized items to usermode"
                        );
                        let _ = communication.send_message(&buffer[..offset], Timeout::infinite());
                        offset = 0;

                        if let Ok(serialized_slice) = postcard::to_slice(&item, &mut buffer) {
                            offset += serialized_slice.len();
                        } else {
                            maple::error!("Double serialization error for item");
                        }
                    }
                }
            }

            if offset != 0 {
                maple::info!("Send {serliazed_items_count} remaining serialized items to usermode");
                communication.send_message(&buffer[..offset], Timeout::infinite());
            }
        }
    }

    #[inline]
    fn try_push_event(&self, message: KmMessage) -> anyhow::Result<(), CommunicationError> {
        let handle = InStackLockHandle::new();
        let result = self
            .internal
            .items
            .lock(&handle)
            .try_push_back(DummyKmMessage(message));

        result.map_err(|_| CommunicationError::NotEnoughMemory)
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.internal.stop_event.signal();

        let handle = InStackLockHandle::new();
        self.internal.items.lock(&handle).clear();
    }
}

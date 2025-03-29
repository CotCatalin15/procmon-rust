use core::{
    sync::atomic::{AtomicU64, Ordering},
    u64,
};

use async_messaging::{AsyncMessaging, MessagingCallback};
use kmum_common::{
    get_communication_port_name, serializable_ntstring::SerializableNtString, ClientConnectMessage,
    KmMessage, KmReplyMessage,
};
use maple::{error, info};
use nt_string::unicode_string::{NtUnicodeStr, NtUnicodeString};
use wdrf_std::kmalloc::TaggedObject;

use crate::global::DRIVER_CONTEXT;

pub mod async_messaging;
mod message_buffer;
pub mod port;

#[derive(Debug)]
pub enum CommunicationError {
    NotEnoughMemory,
    Timeout,
    ParseError,
    PortError,
}

pub struct Communication {
    messaging: AsyncMessaging,
    filter_test_pid: AtomicU64,
}

struct CommunicationCallback {}

impl Communication {
    pub fn try_create() -> anyhow::Result<Self, CommunicationError> {
        let name = get_communication_port_name();

        let name = NtUnicodeStr::try_from_u16(name.as_slice())
            .map_err(|_| CommunicationError::ParseError)?;
        let messaging =
            AsyncMessaging::try_create(4, CommunicationCallback {}, name).inspect_err(|e| {
                error!("Failed to create messaging: {:#?}", e);
            })?;

        Ok(Self {
            messaging,
            filter_test_pid: AtomicU64::new(u64::MAX),
        })
    }

    pub fn try_send_event(&self, message: KmMessage) -> anyhow::Result<(), CommunicationError> {
        let filter_pid = self.filter_test_pid.load(Ordering::Acquire);
        if filter_pid != 0 && filter_pid != message.process.pid {
            Ok(())
        } else {
            self.messaging.try_emplace_event(message)
        }
    }

    pub fn stop(&self) {
        self.messaging.stop();
    }
}

impl MessagingCallback for CommunicationCallback {
    fn on_client(&self, data: Option<ClientConnectMessage>) -> anyhow::Result<()> {
        info!("Client connected {:?}", data);

        if let Some(data) = data {
            match data {
                ClientConnectMessage::Testing { filter_pid } => {
                    DRIVER_CONTEXT
                        .get()
                        .communication
                        .filter_test_pid
                        .store(filter_pid, Ordering::Release);
                }
                ClientConnectMessage::Any => DRIVER_CONTEXT
                    .get()
                    .communication
                    .filter_test_pid
                    .store(0, Ordering::Release),
            }
        }

        Ok(())
    }

    fn on_message(
        &self,
        message: &kmum_common::UmSendMessage,
    ) -> anyhow::Result<Option<kmum_common::KmReplyMessage>, CommunicationError> {
        info!("OnMessage receceied: {:#?}", message);

        match message {
            kmum_common::UmSendMessage::GetProcessInfo(unique_id) => {
                let process_info = DRIVER_CONTEXT
                    .get()
                    .process_cache
                    .get_process_info_from_uid(*unique_id);

                Ok(process_info.map(|info| KmReplyMessage::ProcessInfo(info)))
            }
            kmum_common::UmSendMessage::GetExeName(unique_id) => {
                let process_info = DRIVER_CONTEXT
                    .get()
                    .process_cache
                    .get_process_info_from_uid(*unique_id);

                maple::info!("GetExeName for uid: {unique_id} -> {:?}", process_info);

                if process_info.is_none() {
                    Ok(None)
                } else {
                    let process_info = process_info.unwrap();
                    let nt_string = process_info.path.0.as_u16str();

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

                    let process_name = NtUnicodeString::try_from(path);

                    match process_name {
                        Ok(name) => Ok(Some(KmReplyMessage::ExeName(SerializableNtString::new(
                            name,
                        )))),
                        Err(_) => Ok(None),
                    }
                }
            }
        }
    }

    fn on_disconnect(&self) {
        info!("Client disconnected");
        DRIVER_CONTEXT
            .get()
            .communication
            .filter_test_pid
            .store(u64::MAX, Ordering::Release);
    }
}

impl TaggedObject for CommunicationCallback {
    fn tag() -> wdrf_std::kmalloc::MemoryTag {
        wdrf_std::kmalloc::MemoryTag::new_from_bytes(b"cocb")
    }

    fn flags() -> wdrf_std::constants::PoolFlags {
        wdrf_std::constants::PoolFlags::POOL_FLAG_NON_PAGED
    }
}

use async_messaging::{AsyncMessaging, MessagingCallback};
use kmum_common::{get_communication_port_name, KmMessage, KmReplyMessage};
use maple::{error, info};
use nt_string::unicode_string::NtUnicodeStr;
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

        Ok(Self { messaging })
    }

    pub fn try_send_event(&self, message: KmMessage) -> anyhow::Result<(), CommunicationError> {
        self.messaging.try_emplace_event(message)
    }
}

impl MessagingCallback for CommunicationCallback {
    fn on_client(&self) -> anyhow::Result<()> {
        info!("Client connected");

        Ok(())
    }

    fn on_message(
        &self,
        message: &kmum_common::UmSendMessage,
    ) -> anyhow::Result<Option<kmum_common::KmReplyMessage>, CommunicationError> {
        info!("OnMessage receceied: {:#?}", message);

        match message {
            kmum_common::UmSendMessage::GetPidInfo(pid) => {
                let unique_id = DRIVER_CONTEXT.get().process_cache.pid_to_unique_id(*pid);

                let process_info = if let Some(unique_id) = unique_id {
                    DRIVER_CONTEXT
                        .get()
                        .process_cache
                        .get_process_info_from_uid(unique_id)
                } else {
                    None
                };

                Ok(process_info.map(|info| KmReplyMessage::AboutPid(info)))
            }
        }
    }

    fn on_disconnect(&self) {
        info!("Client disconnected");
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

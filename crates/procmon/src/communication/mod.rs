use kmum_common::{get_communication_port_name, KmMessage, KmReplyMessage, UmReplyMessage};
use maple::{error, info};
use messaging::{Messaging, MessagingCallback};
use nt_string::unicode_string::NtUnicodeStr;
use wdrf::minifilter::FltFilter;
use wdrf_std::{kmalloc::TaggedObject, time::Timeout};

use crate::global::{Test, DRIVER_CONTEXT};

mod message_buffer;
pub mod messaging;
pub mod port;

#[derive(Debug)]
pub enum CommunicationError {
    NotEnoughMemory,
    Timeout,
    ParseError,
    PortError,
}

pub struct Communication {
    messaging: Messaging,
}

struct CommunicationCallback {}

impl Communication {
    pub fn try_create(filter: FltFilter) -> anyhow::Result<Self, CommunicationError> {
        let name = get_communication_port_name();

        let name = NtUnicodeStr::try_from_u16(name.as_slice())
            .map_err(|_| CommunicationError::ParseError)?;
        let messaging =
            Messaging::try_create(CommunicationCallback {}, filter, name).inspect_err(|e| {
                error!("Failed to create messaging: {:#?}", e);
            })?;

        Ok(Self { messaging })
    }

    pub fn send_with_reply(
        &self,
        message: &KmMessage,
        timeout: Timeout,
    ) -> anyhow::Result<Option<UmReplyMessage>, CommunicationError> {
        self.messaging.send_with_reply(message, timeout)
    }
}

impl MessagingCallback for CommunicationCallback {
    fn on_client(&self) -> anyhow::Result<()> {
        info!("Client connected");

        unsafe {
            DRIVER_CONTEXT.get_mut().test = Some(Test::new());
        }

        Ok(())
    }

    fn on_message(
        &self,
        message: &kmum_common::UmSendMessage,
    ) -> anyhow::Result<Option<kmum_common::KmReplyMessage>, CommunicationError> {
        info!("OnMessage receceied: {:#?}", message);
        Ok(Some(KmReplyMessage::Reply(true)))
    }

    fn on_disconnect(&self) {
        info!("Client disconnected");

        unsafe {
            DRIVER_CONTEXT.get_mut().test = None;
        }
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

use kmum_common::{KmMessage, MAX_KM_MESSAGE_RECEIVE_SIZE, MAX_UM_REPLY_MESSAGE_SIZE};
use wdrf_std::{
    kmalloc::{MemoryTag, TaggedObject},
    traits::DispatchSafe,
    vec::{Vec, VecCreate, VecExt},
};

use super::CommunicationError;

pub struct MessageBuffer {
    pub(super) send_buffer: Vec<u8>,
    pub(super) reply_buffer: Vec<u8>,
}

unsafe impl DispatchSafe for MessageBuffer {}
unsafe impl Send for MessageBuffer {}

impl TaggedObject for MessageBuffer {
    fn tag() -> MemoryTag {
        MemoryTag::new_from_bytes(b"msbf")
    }
}

impl MessageBuffer {
    pub fn try_create(can_receive_reply: bool) -> anyhow::Result<Self> {
        let mut send_buffer = Vec::create();
        send_buffer.try_resize(MAX_KM_MESSAGE_RECEIVE_SIZE, 0)?;

        let mut reply_buffer = Vec::create();

        if can_receive_reply {
            reply_buffer.try_resize(MAX_UM_REPLY_MESSAGE_SIZE, 0)?;
        }
        Ok(Self {
            send_buffer: send_buffer,
            reply_buffer: reply_buffer,
        })
    }

    pub fn fill_buffer<F>(
        &mut self,
        f: F,
        message: &KmMessage,
    ) -> anyhow::Result<(), CommunicationError>
    where
        F: FnOnce(&[u8], &mut [u8]) -> anyhow::Result<(), CommunicationError>,
    {
        let buffer = postcard::to_slice(message, self.send_buffer.as_mut_slice())
            .map_err(|_| CommunicationError::ParseError)?;

        f(buffer, &mut self.reply_buffer)
    }
}

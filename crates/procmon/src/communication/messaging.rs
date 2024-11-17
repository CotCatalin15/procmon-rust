use kmum_common::{KmMessage, KmReplyMessage, UmReplyMessage, UmSendMessage};
use maple::error;
use nt_string::unicode_string::NtUnicodeStr;
use wdrf::minifilter::{
    communication::client_communication::{FltClientCommunication, FltCommunicationCallback},
    FltFilter,
};
use wdrf_std::{
    boxed::{Box, BoxExt},
    kmalloc::TaggedObject,
    slice::tracked_slice::SeekFrom,
    sync::{InStackLockHandle, StackSpinMutex},
    time::Timeout,
    vec::{Vec, VecCreate, VecExt},
};

use super::{message_buffer::MessageBuffer, CommunicationError};

pub trait MessagingCallback {
    fn on_client(&self) -> anyhow::Result<()>;
    fn on_message(
        &self,
        message: &UmSendMessage,
    ) -> anyhow::Result<Option<KmReplyMessage>, CommunicationError>;
    fn on_disconnect(&self);
}

pub struct Messaging {
    communication: FltClientCommunication<MessagingPortCallback>,
    free_buffered: StackSpinMutex<Vec<MessageBuffer>>,
}

impl Messaging {
    pub fn try_create<CB: MessagingCallback + TaggedObject + 'static>(
        callback: CB,
        filter: FltFilter,
        name: NtUnicodeStr,
    ) -> anyhow::Result<Self, CommunicationError> {
        let callback = MessagingPortCallback::try_new(callback)?;
        let port_communication = FltClientCommunication::new(callback, filter, name)
            .inspect_err(|e| {
                error!("Failed to create flt client communication: {:#?}", e);
            })
            .map_err(|_| CommunicationError::PortError)?;

        Ok(Self {
            communication: port_communication,
            free_buffered: StackSpinMutex::new(Vec::create()),
        })
    }

    pub fn send_with_reply(
        &self,
        message: &KmMessage,
        timeout: Timeout,
    ) -> anyhow::Result<Option<UmReplyMessage>, CommunicationError> {
        let mut buf = {
            let handle = InStackLockHandle::new();
            let mut guard = self.free_buffered.lock(&handle);

            if let Some(buf) = guard.pop() {
                buf
            } else {
                if let Ok(buf) = MessageBuffer::try_create(true) {
                    buf
                } else {
                    return Err(CommunicationError::NotEnoughMemory);
                }
            }
        };

        let send_slice = postcard::to_slice(message, &mut buf.send_buffer)
            .map_err(|_| CommunicationError::ParseError)?;

        let reply_slice = self
            .communication
            .send_message_with_reply(&send_slice, &mut buf.reply_buffer, timeout)
            .inspect_err(|e| error!("Failed to send message with reply: {:#?}", e))
            .map_err(|_| CommunicationError::PortError)?;

        if reply_slice.is_empty() {
            Ok(None)
        } else {
            let reply =
                postcard::from_bytes(reply_slice).map_err(|_| CommunicationError::ParseError)?;

            let handle = InStackLockHandle::new();
            let _ = self.free_buffered.lock(&handle).try_push(buf);

            Ok(Some(reply))
        }
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
            Err(_) => Err(anyhow::Error::msg("Fai;ed tp parse input")),
        }
    }
}

use std::marker::PhantomData;

use kmum_common::{KmMessage, KmReplyMessage, UmSendMessage, MAX_UM_SEND_MESSAGE_BUFFER_SIZE};

use super::{
    dispatcher::{Dispatcher, FilterBufferHandler},
    CommunicationError, CommunicationInterface, EventProcessor,
};

pub struct DriverCommunication {
    dispatcher: Dispatcher,
}

impl DriverCommunication {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for DriverCommunication {
    fn default() -> Self {
        Self {
            dispatcher: Dispatcher::new(),
        }
    }
}

impl CommunicationInterface for DriverCommunication {
    fn send_message_blocking(
        &self,
        message: &UmSendMessage,
    ) -> anyhow::Result<Option<KmReplyMessage>, CommunicationError> {
        let mut send_buffer = vec![0u8; MAX_UM_SEND_MESSAGE_BUFFER_SIZE];
        let mut reply_buffer = vec![0u8; MAX_UM_SEND_MESSAGE_BUFFER_SIZE];

        let send_slice = postcard::to_slice(message, &mut send_buffer)
            .map_err(|_| CommunicationError::Parsing)?;

        let reply_size = self
            .dispatcher
            .send_message(&send_slice, Some(&mut reply_buffer))? as usize;

        if reply_size > 0 {
            postcard::from_bytes(&reply_buffer[..reply_size])
                .map_err(|_| CommunicationError::Parsing)
                .map(|reply| Some(reply))
        } else {
            Ok(None)
        }
    }

    fn process_blocking<P: EventProcessor>(&self, processor: P) {
        self.dispatcher
            .process_blocking(CommunicationProcessorCallback { processor });
    }
}

struct CommunicationProcessorCallback<P: EventProcessor> {
    processor: P,
}

impl<P> FilterBufferHandler for CommunicationProcessorCallback<P>
where
    P: EventProcessor,
{
    fn handle_buffer(
        &self,
        receive_buffer: &[u8],
        _reply_buffer: &mut [u8],
    ) -> anyhow::Result<(), CommunicationError> {
        let mut km_iter = KmMessageIterator {
            buffer: receive_buffer,
        };

        let _ = self.processor.process(&mut km_iter);

        Ok(())
    }
}

pub struct KmMessageIterator<'a> {
    buffer: &'a [u8],
}

impl<'a> Iterator for KmMessageIterator<'a> {
    type Item = KmMessage;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok((message, remaining)) = postcard::take_from_bytes::<KmMessage>(&self.buffer) {
            self.buffer = remaining;
            Some(message)
        } else {
            None
        }
    }
}

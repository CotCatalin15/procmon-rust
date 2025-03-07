use kmum_common::{KmMessage, KmReplyMessage, UmSendMessage, MAX_UM_SEND_MESSAGE_BUFFER_SIZE};

use super::{
    dispatcher::{Dispatcher, FilterBufferHandler},
    CommunicationError,
};

pub trait MessageProcessor: Send + Sync + 'static {
    fn process(&self, message: &mut KmMessageIterator) -> anyhow::Result<(), CommunicationError>;
}

pub struct Communication {
    dispatcher: Dispatcher,
}

impl Communication {
    pub fn new() -> Self {
        Self {
            dispatcher: Dispatcher::new(),
        }
    }

    pub fn send_message_blocking(
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

    pub fn process_blocking<P>(&self, processor: P)
    where
        P: Fn(&mut KmMessageIterator) -> anyhow::Result<(), CommunicationError>,
    {
        self.dispatcher
            .process_blocking(CommunicationProcessorCallback { processor });
    }
}

struct CommunicationProcessorCallback<
    P: Fn(&mut KmMessageIterator) -> anyhow::Result<(), CommunicationError>,
> {
    processor: P,
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

impl<P: Fn(&mut KmMessageIterator) -> anyhow::Result<(), CommunicationError>> FilterBufferHandler
    for CommunicationProcessorCallback<P>
{
    fn handle_buffer(
        &self,
        receive_buffer: &[u8],
        _reply_buffer: &mut [u8],
    ) -> anyhow::Result<(), CommunicationError> {
        let mut km_iter = KmMessageIterator {
            buffer: receive_buffer,
        };

        let _ = (self.processor)(&mut km_iter);

        Ok(())
    }
}

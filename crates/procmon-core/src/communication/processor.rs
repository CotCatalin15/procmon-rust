use kmum_common::{
    KmMessage, KmReplyMessage, UmReplyMessage, UmSendMessage, MAX_UM_SEND_MESSAGE_BUFFER_SIZE,
};

use super::{
    dispatcher::{Dispatcher, FilterBufferHandler},
    CommunicationError,
};

pub trait MessageProcessor: Send + Sync + 'static {
    fn process(&self, message: &mut KmMessageIterator) -> anyhow::Result<(), CommunicationError>;
}

pub struct CommunicationProcessor {
    dispatcher: Box<dyn DispatcherHolder>,
}

impl CommunicationProcessor {
    pub fn new<P: MessageProcessor>(num_threads: u32, processor: P) -> Self {
        Self {
            dispatcher: Box::new(ProcessorDispatcherHolder::new(num_threads, processor)),
        }
    }

    pub fn send_message(
        &self,
        message: &UmSendMessage,
    ) -> anyhow::Result<Option<KmReplyMessage>, CommunicationError> {
        self.dispatcher.send_message(message)
    }
}

trait DispatcherHolder {
    fn send_message(
        &self,
        message: &UmSendMessage,
    ) -> anyhow::Result<Option<KmReplyMessage>, CommunicationError>;
}

struct ProcessorDispatcherHolder<P: MessageProcessor> {
    dispatcher: Dispatcher<CommunicationProcessorCallback<P>>,
}

impl<P: MessageProcessor> DispatcherHolder for ProcessorDispatcherHolder<P> {
    fn send_message(
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
}

impl<P: MessageProcessor> ProcessorDispatcherHolder<P> {
    fn new(num_threads: u32, processor: P) -> Self {
        Self {
            dispatcher: Dispatcher::new(num_threads, CommunicationProcessorCallback { processor }),
        }
    }
}

struct CommunicationProcessorCallback<P: MessageProcessor> {
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

impl<P: MessageProcessor> FilterBufferHandler for CommunicationProcessorCallback<P> {
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

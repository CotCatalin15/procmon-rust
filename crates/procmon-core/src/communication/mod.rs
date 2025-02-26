use kmum_common::{KmMessage, KmReplyMessage, UmReplyMessage, UmSendMessage};
use nt_string::unicode_string::NtUnicodeString;
use processor::{CommunicationProcessor, KmMessageIterator, MessageProcessor};
use tracing::info;

mod dispatcher;
mod message_handler;
mod parsed;
mod processor;
mod raw_communication;

#[derive(Debug)]
pub enum CommunicationError {
    Parsing,
    NoMemory,
    Port,
    NoWaiterPresent,
}

#[allow(dead_code)]
pub struct Communication {
    processor: CommunicationProcessor,
}

impl Communication {
    pub fn new() -> Self {
        let processor = CommunicationProcessor::new(1, CommunicationMessageHandler {});

        Self { processor }
    }

    pub fn send_message(
        &self,
        message: UmSendMessage,
    ) -> Result<Option<KmReplyMessage>, CommunicationError> {
        self.processor.send_message(&message)
    }
}

struct CommunicationMessageHandler {}

impl MessageProcessor for CommunicationMessageHandler {
    fn process(&self, iter: &mut KmMessageIterator) -> anyhow::Result<(), CommunicationError> {
        for msg in iter {
            info!("Received message from kernel: {:#?}", msg);
        }

        Ok(())
    }
}

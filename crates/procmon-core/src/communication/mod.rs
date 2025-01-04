use kmum_common::{KmMessage, UmReplyMessage, UmSendMessage};
use nt_string::unicode_string::NtUnicodeString;
use processor::{CommunicationProcessor, MessageProcessor};
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

        let reply = processor
            .send_message(&UmSendMessage::Redirect(
                NtUnicodeString::try_from("RATATATAT").unwrap().into(),
            ))
            .unwrap();

        info!("Received reply from km: {:#?}", reply);

        Self { processor }
    }
}

struct CommunicationMessageHandler {}

impl MessageProcessor for CommunicationMessageHandler {
    fn process(
        &self,
        message: &KmMessage,
    ) -> anyhow::Result<Option<UmReplyMessage>, CommunicationError> {
        info!("Received message from kernel: {:#?}", message);

        Ok(None)
    }
}

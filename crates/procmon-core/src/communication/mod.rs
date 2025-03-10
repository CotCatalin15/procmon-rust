use kmum_common::{KmMessage, KmReplyMessage, UmSendMessage};

mod dispatcher;
mod message_handler;
mod parsed;
mod raw_communication;

pub mod driver_communication;

#[derive(Debug)]
pub enum CommunicationError {
    Parsing,
    NoMemory,
    Port,
    NoWaiterPresent,
    TokioSender,
}

pub trait EventProcessor {
    fn process<I>(&self, iter: &mut I) -> anyhow::Result<(), CommunicationError>
    where
        I: Iterator<Item = KmMessage>;
}

pub trait CommunicationInterface: Sync + Send + 'static {
    fn send_message_blocking(
        &self,
        message: &UmSendMessage,
    ) -> anyhow::Result<Option<KmReplyMessage>, CommunicationError>;

    fn process_blocking<P: EventProcessor>(&self, processor: P);
}

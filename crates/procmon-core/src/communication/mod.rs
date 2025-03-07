mod dispatcher;
mod message_handler;
mod parsed;
mod raw_communication;

pub mod communication;

#[derive(Debug)]
pub enum CommunicationError {
    Parsing,
    NoMemory,
    Port,
    NoWaiterPresent,
    TokioSender,
}

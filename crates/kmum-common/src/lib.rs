#![no_std]

use nt_string::widestring::U16CStr;
use serde::{Deserialize, Serialize};
use serializable_ntstring::SerializableNtString;

pub mod serializable_ntstring;

pub fn get_communication_port_name() -> &'static U16CStr {
    nt_string::widestring::u16cstr!("\\PROCMONPORT")
}

pub const MAX_KM_MESSAGE_RECEIVE_SIZE: usize = 32 * 1024;
pub const MAX_UM_REPLY_MESSAGE_SIZE: usize = 32 * 1024;

pub const MAX_UM_SEND_MESSAGE_BUFFER_SIZE: usize = 32 * 1024;

//Km -> Um
#[derive(Debug, Serialize, Deserialize)]
pub enum KmMessage {
    CreateFile(SerializableNtString),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum UmReplyMessage {
    Reply(bool),
    Redirect(SerializableNtString),
}

//Um -> Km
#[derive(Debug, Serialize, Deserialize)]
pub enum UmSendMessage {
    Reply(bool),
    Redirect(SerializableNtString),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KmReplyMessage {
    Reply(bool),
}

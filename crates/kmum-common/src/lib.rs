#![no_std]

use event::{EventCompoent, EventStack, SimpleProcessDetails};
use nt_string::{unicode_string::NtUnicodeString, widestring::U16CStr};
use process::{ProcessInformation, UniqueProcessId};
use serde::{Deserialize, Serialize};
use serializable_ntstring::SerializableNtString;

pub mod event;
pub mod process;
pub mod serializable_ntstring;

pub fn get_communication_port_name() -> &'static U16CStr {
    nt_string::widestring::u16cstr!("\\PROCMONPORT")
}

pub const MAX_KM_MESSAGE_RECEIVE_SIZE: usize = 32 * 1024;
pub const MAX_UM_REPLY_MESSAGE_SIZE: usize = 32 * 1024;

pub const MAX_UM_SEND_MESSAGE_BUFFER_SIZE: usize = 32 * 1024;

//Km -> Um
#[derive(Debug, Serialize, Deserialize)]
pub struct KmMessage {
    pub event: EventCompoent,
    pub process: SimpleProcessDetails,
    pub stack: EventStack,
}

unsafe impl Sync for KmMessage {}
unsafe impl Send for KmMessage {}

#[derive(Debug, Serialize, Deserialize)]
pub enum UmReplyMessage {}

//Um -> Km
#[derive(Debug, Serialize, Deserialize)]
pub enum UmSendMessage {
    GetProcessInfo(UniqueProcessId),
    GetExeName(UniqueProcessId),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KmReplyMessage {
    ProcessInfo(ProcessInformation),
    ExeName(SerializableNtString),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientConnectMessage {
    Any,
    Testing { filter_pid: u64 },
}

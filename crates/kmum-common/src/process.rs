use serde::{Deserialize, Serialize};

use crate::serializable_ntstring::SerializableNtString;

pub type UniqueProcessId = u64;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessInformation {
    pub path: SerializableNtString,
    pub cmd: Option<SerializableNtString>,
    pub pid: u64,
    pub parent_pid: u64,
    pub start_time: u64,
    pub end_time: Option<u64>,

    pub unique_id: UniqueProcessId,
}

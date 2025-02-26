use serde::{Deserialize, Serialize};

use crate::serializable_ntstring::SerializableNtString;

#[derive(Debug, Serialize, Deserialize)]
pub enum KmMessageOperationType {
    ProcessCreate,
    ProcessDestroy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KmMessageCommonHeader {
    pub operation: KmMessageOperationType,
    pub timestamp: u64, //Timestamp in 100ns
    pub pid: u64,
    pub thread_id: u64,
    pub class: u64,
    pub result: i32, //ntstatus
    pub path: SerializableNtString,
    pub duration: u64,

    //pids can be recycled by the system, this one is globaly unique
    pub unique_pid: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum KmMessageEventKind {
    DummyEvent(),
    ProcessCreate(ProcessCreateEvent),
    ProcessDestroy(ProcessDestroyEvent),
    FileSystem(FileSystemEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessCreateEvent {
    pub pid: u64,
    pub cmd: Option<SerializableNtString>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessDestroyEvent {
    pub pid: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FileSystemEvent {
    Create { attributes: u64 },
    Read { offset: u64, length: u64 },
}

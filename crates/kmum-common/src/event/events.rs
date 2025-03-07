use super::{EventFileSystemOperation, EventProcessOperation, EventRegistryOperation};
use crate::serializable_ntstring::SerializableNtString;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum EventClass {
    Process(EventProcessOperation),
    FileSystem(EventFileSystemOperation),
    Registry(EventRegistryOperation),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimpleProcessDetails {
    pub pid: u64,
    pub unique_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EventCompoent {
    pub date: u64,
    pub thread: u64,
    pub operation: EventClass,
    pub result: i32,
    pub path: SerializableNtString,
    pub duration: u64,
}

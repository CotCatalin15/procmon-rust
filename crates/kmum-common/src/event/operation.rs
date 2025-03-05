use serde::{Deserialize, Serialize};

use crate::serializable_ntstring::SerializableNtString;

#[derive(Debug, Serialize, Deserialize)]
pub enum EventProcessOperation {
    ProcessCreate {
        pid: u64,
        cmd: Option<SerializableNtString>,
    },
    ProcessDestroy {
        pid: u64,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EventFileSystemOperation {
    Create { attribute: u64 },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EventRegistryOperation {}

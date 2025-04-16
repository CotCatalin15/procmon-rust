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
    Create {
        attribute: u16,
    },
    Read {
        length: u64,
        offset: i64,
    },
    Write {
        length: u64,
        offset: i64,
    },
    Cleanup {},
    Close {},
    QueryFileInfo {
        info_class: u32,
        buffer_len: u32,
    },
    SetFileInfo {
        info_class: u32,
        length: u32,
    },
    AcquireForSectionSync {
        sync_type: u32,
        page_protection: u32,
        flags: u32,
        allocation_attributes: u32,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum EventRegistryOperation {
    Open(),
}

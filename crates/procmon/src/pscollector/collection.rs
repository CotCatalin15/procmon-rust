use core::sync::atomic::{AtomicU64, Ordering};

use kmum_common::{
    process::{ProcessInformation, UniqueProcessId},
    serializable_ntstring::SerializableNtString,
};
use nt_string::{unicode_string::NtUnicodeString, widestring::u16cstr};
use wdrf::process::ps_lookup_by_process_id;
use wdrf_std::{
    constants::PoolFlags,
    hashbrown::{HashMap, HashMapExt},
    kmalloc::{GlobalKernelAllocator, MemoryTag, TaggedObject},
    object::ArcKernelObj,
    structs::PKPROCESS,
    sync::ExSpinMutex,
    time::SystemTime,
    traits::DispatchSafe,
};

use super::proc_info_factory::ProcessInformationFactory;

#[derive(Clone, Debug)]
struct DispatchSafeProcessInfo(ProcessInformation);

unsafe impl DispatchSafe for DispatchSafeProcessInfo {}

pub struct PsInfoContainer {
    last_unique_id: AtomicU64,

    pid_to_unique_id: ExSpinMutex<HashMap<u64, UniqueProcessId>>,
    process_information_map: ExSpinMutex<HashMap<UniqueProcessId, Option<DispatchSafeProcessInfo>>>,
}

impl PsInfoContainer {
    pub fn create() -> Self {
        let container = Self {
            last_unique_id: AtomicU64::new(4),
            pid_to_unique_id: ExSpinMutex::new(HashMap::create_in(GlobalKernelAllocator::new(
                MemoryTag::new_from_bytes(b"piui"),
                PoolFlags::POOL_FLAG_NON_PAGED,
            ))),
            process_information_map: ExSpinMutex::new(HashMap::create_in(
                GlobalKernelAllocator::new(
                    MemoryTag::new_from_bytes(b"psin"),
                    PoolFlags::POOL_FLAG_NON_PAGED,
                ),
            )),
        };

        let system_path = NtUnicodeString::try_from(u16cstr!("system")).unwrap();

        container.pid_to_unique_id.write().insert(4, 1); //system
        container.process_information_map.write().insert(
            1, //uid
            Some(DispatchSafeProcessInfo(ProcessInformation {
                path: SerializableNtString::new(system_path),
                cmd: None,
                pid: 4,
                parent_pid: 0,
                start_time: 0,
                end_time: None,
                unique_id: 1,
            })),
        );

        container
    }

    pub fn get_uid(&self, pid: u64) -> Option<UniqueProcessId> {
        self.pid_to_unique_id.read().get(&pid).copied()
    }

    pub fn get_info(&self, uid: UniqueProcessId) -> Option<ProcessInformation> {
        self.process_information_map
            .read()
            .get(&uid)
            .cloned()?
            .map(|d| d.0)
    }

    pub fn map_pid(&self, pid: u64, info: Option<ProcessInformation>) -> UniqueProcessId {
        let next_uid = self.last_unique_id.fetch_add(1, Ordering::SeqCst);

        maple::info!("Mapping PID: {pid} to {next_uid}");

        let mut uid_guard = self.pid_to_unique_id.write();
        let mut process_guard = self.process_information_map.write();

        match uid_guard.try_insert(pid, next_uid) {
            Ok(_) => {
                process_guard.insert(next_uid, info.map(|value| DispatchSafeProcessInfo(value)));

                next_uid
            }
            Err(occ) => occ.value,
        }
    }

    pub fn unmap_pid(&self, pid: u64) -> Option<UniqueProcessId> {
        let end_time = SystemTime::new().raw_time();

        let mut uid_guard = self.pid_to_unique_id.write();
        let mut process_guard = self.process_information_map.write();

        let uid = *uid_guard.get(&pid)?;

        let process_info = process_guard.get_mut(&uid)?;
        if let Some(proc) = process_info {
            proc.0.end_time = Some(end_time);
        }

        Some(uid)
    }
}

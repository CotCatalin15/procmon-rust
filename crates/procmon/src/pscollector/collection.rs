use core::sync::atomic::{AtomicU64, Ordering};

use kmum_common::{
    process::{ProcessInformation, UniqueProcessId},
    serializable_ntstring::SerializableNtString,
};
use nt_string::{unicode_string::NtUnicodeString, widestring::u16cstr};
use wdrf::process::ps_lookup_by_process_id;
use wdrf_std::{
    hashbrown::{HashMap, HashMapExt},
    kmalloc::{GlobalKernelAllocator, TaggedObject},
    object::ArcKernelObj,
    structs::PKPROCESS,
    sync::ExSpinMutex,
    time::SystemTime,
    traits::DispatchSafe,
};

use super::proc_info_factory::ProcessInformationFactory;

#[derive(Clone)]
enum ProcessMapping {
    Partial {
        uid: UniqueProcessId,
        pid: u64,
        eprocess: ArcKernelObj<PKPROCESS>,
    },
    Full(ProcessInformation),
    Failed,
}

unsafe impl DispatchSafe for ProcessMapping {}
impl TaggedObject for ProcessMapping {}

struct ProcessInformationInternal {}

pub struct PsInfoContainer {
    last_unique_id: AtomicU64,

    pid_to_unique_id: ExSpinMutex<HashMap<u64, UniqueProcessId>>,
    process_information_map: ExSpinMutex<HashMap<UniqueProcessId, ProcessMapping>>,
}

impl PsInfoContainer {
    pub fn create() -> Self {
        let container = Self {
            last_unique_id: AtomicU64::new(4),
            pid_to_unique_id: ExSpinMutex::new(HashMap::create_in(
                GlobalKernelAllocator::new_for_tagged::<ProcessMapping>(),
            )),
            process_information_map: ExSpinMutex::new(HashMap::create_in(
                GlobalKernelAllocator::new_for_tagged::<ProcessMapping>(),
            )),
        };

        let system_path = NtUnicodeString::try_from(u16cstr!("system")).unwrap();

        container.pid_to_unique_id.write().insert(4, 1); //system
        container.process_information_map.write().insert(
            1, //uid
            ProcessMapping::Full(ProcessInformation {
                path: SerializableNtString::new(system_path),
                cmd: None,
                pid: 4,
                parent_pid: 0,
                start_time: 0,
                end_time: None,
                unique_id: 1,
            }),
        );

        container
    }

    pub fn get_uid(&self, pid: u64) -> Option<UniqueProcessId> {
        if pid < 4 {
            return None;
        }

        let uid = self.pid_to_unique_id.read().get(&pid).cloned();
        if uid.is_some() {
            return uid;
        }

        let next_uid = self.last_unique_id.fetch_add(1, Ordering::SeqCst);
        let eprocess = ps_lookup_by_process_id(pid as _)?;
        let mut uid_guard = self.pid_to_unique_id.write();

        match uid_guard.try_insert(pid, next_uid) {
            Ok(_) => {
                let mut proc_guard = self.process_information_map.write();
                proc_guard.insert(
                    next_uid,
                    ProcessMapping::Partial {
                        uid: next_uid,
                        pid,
                        eprocess,
                    },
                );

                Some(next_uid)
            }
            Err(occ) => Some(*occ.entry.key()),
        }
    }

    pub fn get_info(&self, uid: UniqueProcessId) -> Option<ProcessInformation> {
        let proc_guard = self.process_information_map.read();
        let mapping = proc_guard.get(&uid)?;
        if !mapping.needs_upgrade() {
            return mapping.process_info().cloned();
        };
        let mut mapping = mapping.clone();
        drop(proc_guard);

        mapping.upgrade()?;

        let mut proc_guard = self.process_information_map.write();
        let info = mapping.process_info().cloned();
        proc_guard.insert(uid, mapping);

        info
    }

    pub fn unmap_pid(&self, pid: u64) -> Option<UniqueProcessId> {
        self.pid_to_unique_id.write().remove(&pid).clone()
    }
}

impl ProcessMapping {
    fn needs_upgrade(&self) -> bool {
        match self {
            ProcessMapping::Partial { uid, pid, eprocess } => true,
            _ => false,
        }
    }

    fn upgrade(&mut self) -> Option<u64> {
        match self {
            ProcessMapping::Partial { uid, pid, eprocess } => {
                if let Some(mut process_info) =
                    ProcessInformationFactory::try_create_from_eprocess(&*eprocess, *pid)
                {
                    process_info.unique_id = *uid;
                    Some(*uid)
                } else {
                    None
                }
            }
            ProcessMapping::Full(process_info) => Some(process_info.unique_id),
            _ => None,
        }
    }

    fn process_info(&self) -> Option<&ProcessInformation> {
        match self {
            ProcessMapping::Full(process_information) => Some(process_information),
            _ => None,
        }
    }

    fn mark_terminate_time(&mut self) {
        match self {
            ProcessMapping::Full(process_information) => {
                process_information.end_time = Some(SystemTime::new().raw_time());
            }
            _ => {}
        }
    }
}

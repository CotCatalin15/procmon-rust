use core::sync::atomic::{AtomicU64, Ordering};

use kmum_common::process::{ProcessInformation, UniqueProcessId};
use wdrf::process::{ps_lookup_by_process_id, PsCreateNotifyInfo};
use wdrf_std::{
    hashbrown::{HashMap, HashMapExt},
    kmalloc::{GlobalKernelAllocator, TaggedObject},
    object::ArcKernelObj,
    structs::PKPROCESS,
    sync::{ExSpinMutex, InStackLockHandle},
    time::SystemTime,
    traits::DispatchSafe,
};
use windows_sys::Win32::Foundation::HANDLE;

use super::proc_info_factory::ProcessInformationFactory;

pub struct DispatchSafeProcessInformation(ProcessInformation);

unsafe impl DispatchSafe for DispatchSafeProcessInformation {}
impl TaggedObject for DispatchSafeProcessInformation {}

pub struct PsInfoContainer {
    last_unique_id: AtomicU64,

    pid_to_unique_id: ExSpinMutex<HashMap<u64, UniqueProcessId>>,
    process_information_map: ExSpinMutex<HashMap<UniqueProcessId, DispatchSafeProcessInformation>>,
}

impl PsInfoContainer {
    pub fn create() -> Self {
        Self {
            last_unique_id: AtomicU64::new(1),
            pid_to_unique_id: ExSpinMutex::new(HashMap::create_in(
                GlobalKernelAllocator::new_for_tagged::<DispatchSafeProcessInformation>(),
            )),
            process_information_map: ExSpinMutex::new(HashMap::create_in(
                GlobalKernelAllocator::new_for_tagged::<DispatchSafeProcessInformation>(),
            )),
        }
    }

    pub fn get_process_info_from_uid(
        &self,
        unique_id: UniqueProcessId,
    ) -> Option<ProcessInformation> {
        let guard = self.process_information_map.read();

        guard.get(&unique_id).map(|info| (&info.0).clone())
    }

    pub fn pid_to_unique_id(&self, pid: u64) -> Option<UniqueProcessId> {
        {
            let guard = self.pid_to_unique_id.read();

            if let Some(uid) = guard.get(&pid) {
                return Some(*uid);
            }
        }

        let eprocess = ps_lookup_by_process_id(pid as _)?;

        //lazy map the process
        let mut process_info = ProcessInformationFactory::try_create_from_eprocess(&eprocess, pid)?;

        Some(self.create_mapping(process_info))
    }

    pub fn register_from_process_create(
        &self,
        eprocess: &ArcKernelObj<PKPROCESS>,
        pid: HANDLE,
        process_info: &PsCreateNotifyInfo,
    ) -> Option<UniqueProcessId> {
        let process_info =
            ProcessInformationFactory::try_create_from_process_create(eprocess, pid, process_info)?;

        Some(self.create_mapping(process_info))
    }

    pub fn register_from_process_destroy(&self, pid: u64) -> Option<UniqueProcessId> {
        let exit_time = SystemTime::new().raw_time();

        let unique_guard = self.pid_to_unique_id.read();

        if let Some(uid) = unique_guard.get(&pid) {
            let mut process_guard = self.process_information_map.write();

            process_guard.get_mut(uid).map(|process_info| {
                process_info.0.end_time = Some(exit_time);
                process_info.0.unique_id
            })
        } else {
            None
        }
    }

    fn create_mapping(&self, mut process_information: ProcessInformation) -> UniqueProcessId {
        let pid = process_information.pid;
        let next_uid = self.last_unique_id.fetch_add(1, Ordering::SeqCst);

        let mut unique_guard = self.pid_to_unique_id.write();
        let mut process_guard = self.process_information_map.write();

        if let Some(uid) = unique_guard.get(&pid) {
            *uid
        } else {
            process_information.unique_id = next_uid;

            unique_guard.insert(pid, next_uid);
            process_guard.insert(
                next_uid,
                DispatchSafeProcessInformation(process_information),
            );

            next_uid
        }
    }
}

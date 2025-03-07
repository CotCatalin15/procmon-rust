use core::sync::atomic::{AtomicU64, Ordering};

use kmum_common::{
    process::{ProcessInformation, UniqueProcessId},
    serializable_ntstring::SerializableNtString,
};
use nt_string::unicode_string::NtUnicodeString;
use wdrf::process::{ke_stack_attach_process, ps_lookup_by_process_id};
use wdrf_std::{
    hashbrown::{HashMap, HashMapExt},
    kmalloc::{GlobalKernelAllocator, TaggedObject},
    nt_success,
    object::{handle::Handle, ArcKernelObj, KernelObjectType},
    structs::PKPROCESS,
    sync::{InStackLockHandle, StackSpinMutex},
    traits::DispatchSafe,
};
use windows_sys::{
    Wdk::{
        Storage::FileSystem::{ObOpenObjectByPointer, SeLocateProcessImageName},
        System::SystemServices::{ExFreePool, KernelMode, PsGetProcessCreateTimeQuadPart},
    },
    Win32::{
        Foundation::GENERIC_READ,
        System::{Kernel::OBJ_KERNEL_HANDLE, Threading::PROCESS_BASIC_INFORMATION},
    },
};

use crate::imports::{ProcesInformationClass, DYN_IMPORTS};

#[derive(Debug, Clone)]
struct DispatchSafeProcessInfo(ProcessInformation);
unsafe impl DispatchSafe for DispatchSafeProcessInfo {}

impl TaggedObject for DispatchSafeProcessInfo {}

pub struct ProcessCollectorCache {
    last_unique_id: AtomicU64,
    pid_to_unique_id: StackSpinMutex<HashMap<u64, UniqueProcessId>>,

    process_map: StackSpinMutex<HashMap<UniqueProcessId, DispatchSafeProcessInfo>>,
}

unsafe impl Sync for ProcessCollectorCache {}
unsafe impl Send for ProcessCollectorCache {}

impl ProcessCollectorCache {
    pub fn create() -> Self {
        Self {
            last_unique_id: AtomicU64::new(1),
            pid_to_unique_id: StackSpinMutex::new(HashMap::create_in(
                GlobalKernelAllocator::new_for_tagged::<DispatchSafeProcessInfo>(),
            )),
            process_map: StackSpinMutex::new(HashMap::create_in(
                GlobalKernelAllocator::new_for_tagged::<DispatchSafeProcessInfo>(),
            )),
        }
    }

    pub fn get_unique_from_pid(&self, pid: u64) -> Option<UniqueProcessId> {
        {
            let handle = InStackLockHandle::new();
            let guard = self.pid_to_unique_id.lock(&handle);

            if guard.contains_key(&pid) {
                return guard.get(&pid).map(|value| *value);
            }
        }

        self.lazy_map(pid)
    }

    pub fn get_process_info_from_unique_id(
        &self,
        unique_id: UniqueProcessId,
    ) -> Option<ProcessInformation> {
        let process_info = {
            let handle = InStackLockHandle::new();
            let guard = self.process_map.lock(&handle);
            guard.get(&unique_id).cloned()
        };

        process_info.map(|ds| ds.0)
    }

    fn lazy_map(&self, pid: u64) -> Option<UniqueProcessId> {
        let eprocess = ps_lookup_by_process_id(pid as _)?;

        let process_info = unsafe { populate_process_info_from_eprocess(eprocess)? };

        let next_id = self.last_unique_id.fetch_add(1, Ordering::SeqCst);

        let process_info = ProcessInformation {
            pid: pid,
            start_time: 0,
            unique_id: next_id,
            ..process_info
        };

        {
            let handle_pid = InStackLockHandle::new();
            let proc_handle = InStackLockHandle::new();

            let mut pid_to_uid = self.pid_to_unique_id.lock(&handle_pid);
            let mut proc_info = self.process_map.lock(&proc_handle);

            let _ = pid_to_uid.insert(pid, next_id);
            let _ = proc_info.insert(next_id, DispatchSafeProcessInfo(process_info));
        }

        Some(next_id)
    }
}

unsafe fn populate_process_info_from_eprocess(
    eprocess: ArcKernelObj<PKPROCESS>,
) -> Option<ProcessInformation> {
    let mut process_handle = 0;
    let status = ObOpenObjectByPointer(
        eprocess.as_raw_obj() as _,
        OBJ_KERNEL_HANDLE as _,
        core::ptr::null(),
        GENERIC_READ,
        0,
        KernelMode as _,
        &mut process_handle,
    );
    if !nt_success(status) {
        maple::error!("ObOpenObjectByPointer failed with status: {status}");
        return None;
    }

    let process_handle = Handle::new(KernelObjectType::Process, process_handle);
    let mut basic_information: PROCESS_BASIC_INFORMATION = core::mem::zeroed();

    let mut length: u32 = 0;
    let basic_information_ptr: *mut PROCESS_BASIC_INFORMATION = &mut basic_information;
    let status = DYN_IMPORTS.get().zw_query_information_process(
        process_handle.raw_handle(),
        ProcesInformationClass::ProcessBasicInformation,
        basic_information_ptr as _,
        core::mem::size_of::<PROCESS_BASIC_INFORMATION>() as _,
        &mut length,
    );
    if !nt_success(status) || length != core::mem::size_of::<PROCESS_BASIC_INFORMATION>() as _ {
        maple::error!("zw_query_information_process failed with status: {status}");
        return None;
    }

    let cmd_line = ke_stack_attach_process(&eprocess, || {
        if basic_information.PebBaseAddress.is_null()
            || (*basic_information.PebBaseAddress)
                .ProcessParameters
                .is_null()
            || (*(*basic_information.PebBaseAddress).ProcessParameters)
                .CommandLine
                .Buffer
                .is_null()
            || (*(*basic_information.PebBaseAddress).ProcessParameters)
                .CommandLine
                .Length
                == 0
        {
            None
        } else {
            let cmd_line_raw = (*(*basic_information.PebBaseAddress).ProcessParameters).CommandLine;

            let buffer: &[u16] = core::slice::from_raw_parts(
                cmd_line_raw.Buffer,
                (cmd_line_raw.Length as usize) / core::mem::size_of::<u16>(),
            );
            let mut cmd_line = NtUnicodeString::new();
            cmd_line.try_push_u16(buffer).ok()?;

            Some(cmd_line)
        }
    })
    .ok()?
    .map(|cmd_line| SerializableNtString::new(cmd_line));

    let time = PsGetProcessCreateTimeQuadPart(eprocess.as_raw_obj() as _);

    let mut image_name_raw = core::ptr::null_mut();
    let status = SeLocateProcessImageName(eprocess.as_raw_obj() as _, &mut image_name_raw);
    let path = if nt_success(status) {
        let buffer: &[u16] = core::slice::from_raw_parts(
            (*image_name_raw).Buffer,
            ((*image_name_raw).Length as usize) / core::mem::size_of::<u16>(),
        );
        let mut path = NtUnicodeString::new();
        path.try_push_u16(buffer).ok()?;

        ExFreePool(image_name_raw as _);

        SerializableNtString::new(path)
    } else {
        SerializableNtString::new(NtUnicodeString::new())
    };

    Some(ProcessInformation {
        path,
        cmd: cmd_line.unwrap_or_else(|| SerializableNtString::new(NtUnicodeString::new())),
        pid: 0,
        parent_pid: 0,
        start_time: time as u64,
        end_time: None,
        unique_id: 0,
    })
}

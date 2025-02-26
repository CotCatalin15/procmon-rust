use kmum_common::{process::ProcessInformation, serializable_ntstring::SerializableNtString};
use nt_string::unicode_string::NtUnicodeString;
use wdrf::process::{ke_stack_attach_process, PsCreateNotifyInfo};
use wdrf_std::{
    nt_success,
    object::{handle::Handle, ArcKernelObj, KernelObjectType},
    slice::slice_from_raw_parts_or_empty,
    structs::PKPROCESS,
};
use windows_sys::{
    Wdk::{
        Storage::FileSystem::{ObOpenObjectByPointer, SeLocateProcessImageName},
        System::SystemServices::{ExFreePool, KernelMode, PsGetProcessCreateTimeQuadPart},
    },
    Win32::{
        Foundation::{GENERIC_READ, HANDLE},
        System::{Kernel::OBJ_KERNEL_HANDLE, Threading::PROCESS_BASIC_INFORMATION},
    },
};

use crate::imports::{ProcesInformationClass, DYN_IMPORTS};

pub struct ProcessInformationFactory {}

impl ProcessInformationFactory {
    pub fn try_create_from_eprocess(
        eprocess: &ArcKernelObj<PKPROCESS>,
        pid: u64,
    ) -> Option<ProcessInformation> {
        let path = unsafe { se_get_process_image_path(eprocess) }?;
        let cmd_line = unsafe { get_cmd_line_from_eprocess(eprocess) };
        let parent_process = unsafe {
            DYN_IMPORTS
                .get()
                .ps_get_process_inherited_from_unique_process_id(eprocess.as_raw_obj())
        };

        Some(ProcessInformation {
            path: SerializableNtString::new(path),
            cmd: cmd_line.map(SerializableNtString::new),
            pid: pid,
            parent_pid: parent_process as _,
            start_time: Self::get_eprocess_start_time(eprocess),
            end_time: None,
            unique_id: 0,
        })
    }

    pub fn try_create_from_process_create(
        eprocess: &ArcKernelObj<PKPROCESS>,
        pid: HANDLE,
        process_info: &PsCreateNotifyInfo,
    ) -> Option<ProcessInformation> {
        let path = unsafe { se_get_process_image_path(eprocess) }?;
        let parent_process = process_info.client_id.UniqueProcess as u64;

        let cmd_line = if let Some(cmd) = process_info.command_line {
            Some(NtUnicodeString::from(&cmd))
        } else {
            unsafe { get_cmd_line_from_eprocess(eprocess) }
        };

        Some(ProcessInformation {
            path: SerializableNtString::new(path),
            cmd: cmd_line.map(SerializableNtString::new),
            pid: pid as _,
            parent_pid: parent_process as _,
            start_time: Self::get_eprocess_start_time(eprocess),
            end_time: None,
            unique_id: 0,
        })
    }

    fn get_eprocess_start_time(eprocess: &ArcKernelObj<PKPROCESS>) -> u64 {
        unsafe { PsGetProcessCreateTimeQuadPart(eprocess.as_raw_obj() as _) as _ }
    }
}

unsafe fn se_get_process_image_path(eprocess: &ArcKernelObj<PKPROCESS>) -> Option<NtUnicodeString> {
    let mut image_unicode = core::ptr::null_mut();
    let status = SeLocateProcessImageName(eprocess.as_raw_obj() as _, &mut image_unicode);

    if nt_success(status) {
        let buffer: &[u16] = slice_from_raw_parts_or_empty(
            (*image_unicode).Buffer,
            ((*image_unicode).Length as usize) / core::mem::size_of::<u16>(),
        );

        let mut path = NtUnicodeString::new();

        if !buffer.is_empty() {
            path.try_push_u16(buffer).ok()?;
        }

        ExFreePool(image_unicode as _);

        Some(path)
    } else {
        None
    }
}

unsafe fn get_cmd_line_from_eprocess(
    eprocess: &ArcKernelObj<PKPROCESS>,
) -> Option<NtUnicodeString> {
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

    ke_stack_attach_process(&eprocess, || {
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
}

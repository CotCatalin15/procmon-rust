use core::ffi::c_void;

use nt_string::widestring::{self, U16CStr};
use wdrf::context::{Context, ContextRegistry};
use wdrf_std::structs::PKPROCESS;
use windows_sys::{
    Wdk::System::SystemServices::MmGetSystemRoutineAddress,
    Win32::{
        Foundation::{HANDLE, NTSTATUS, UNICODE_STRING},
        System::Threading::PROCESS_INFORMATION_CLASS,
    },
};

type ZwQueryInformationProcessFn = unsafe extern "system" fn(
    HANDLE,
    PROCESS_INFORMATION_CLASS,
    *mut core::ffi::c_void,
    u32,
    *mut u32,
) -> NTSTATUS;

type PsGetProcessInheritedFromUniqueProcessIdFn = unsafe extern "system" fn(PKPROCESS) -> HANDLE;

pub struct DynFncImports {
    fn_zw_query_information_process: ZwQueryInformationProcessFn,
    fn_ps_get_process_inherited_from_unique_process_id: PsGetProcessInheritedFromUniqueProcessIdFn,
}

#[repr(i32)]
pub enum ProcesInformationClass {
    ProcessBasicInformation = 0,
    ProcessDebugPort = 7,
    ProcessWow64Information = 26,
    ProcessImageFileName = 27,
    ProcessBreakOnTermination = 29,
    ProcessProtectionInformation = 61,
}

pub static DYN_IMPORTS: Context<DynFncImports> = Context::uninit();

impl DynFncImports {
    fn load_fnc(name: &U16CStr) -> Option<*mut c_void> {
        unsafe {
            // Convert the U16CStr to a UNICODE_STRING
            let unicode_struct = UNICODE_STRING {
                Length: (name.len() * core::mem::size_of::<u16>()) as u16,
                MaximumLength: (name.len() * core::mem::size_of::<u16>()) as u16,
                Buffer: name.as_ptr() as _,
            };

            // Call MmGetSystemRoutineAddress to load the function pointer
            let func_ptr = MmGetSystemRoutineAddress(&unicode_struct);

            // Check if the function pointer is null
            if func_ptr.is_null() {
                None
            } else {
                // Cast the raw pointer to the target function type T
                Some(func_ptr)
            }
        }
    }

    pub fn try_load<R: ContextRegistry>(registry: &'static R) -> anyhow::Result<()> {
        let zw_query_info = {
            let fnc_ptr = Self::load_fnc(widestring::u16cstr!("ZwQueryInformationProcess"));

            // Check if the function pointer was successfully loaded
            if let Some(ptr) = fnc_ptr {
                // Cast the function pointer to the correct function type
                unsafe { core::mem::transmute::<*mut c_void, ZwQueryInformationProcessFn>(ptr) }
            } else {
                // Return an error if the function could not be loaded
                return Err(anyhow::anyhow!("Failed to load ZwQueryInformationProcess")).into();
            }
        };

        let ps_inherited_process_id = {
            let fnc_ptr = Self::load_fnc(widestring::u16cstr!(
                "PsGetProcessInheritedFromUniqueProcessId"
            ));

            // Check if the function pointer was successfully loaded
            if let Some(ptr) = fnc_ptr {
                // Cast the function pointer to the correct function type
                unsafe {
                    core::mem::transmute::<*mut c_void, PsGetProcessInheritedFromUniqueProcessIdFn>(
                        ptr,
                    )
                }
            } else {
                // Return an error if the function could not be loaded
                return Err(anyhow::anyhow!(
                    "Failed to load PsGetProcessInheritedFromUniqueProcessId"
                ))
                .into();
            }
        };

        DYN_IMPORTS.init(registry, move || DynFncImports {
            fn_zw_query_information_process: zw_query_info,
            fn_ps_get_process_inherited_from_unique_process_id: ps_inherited_process_id,
        })
    }

    pub unsafe fn zw_query_information_process(
        &self,
        proce_handle: HANDLE,
        process_information_class: ProcesInformationClass,
        process_information: *mut core::ffi::c_void,
        process_information_length: u32,
        return_length: *mut u32,
    ) -> NTSTATUS {
        (self.fn_zw_query_information_process)(
            proce_handle,
            process_information_class as i32,
            process_information,
            process_information_length,
            return_length,
        )
    }

    pub unsafe fn ps_get_process_inherited_from_unique_process_id(
        &self,
        eprocess: PKPROCESS,
    ) -> HANDLE {
        (self.fn_ps_get_process_inherited_from_unique_process_id)(eprocess)
    }
}

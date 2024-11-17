#![no_std]

use communication::Communication;
use global::{DriverContext, CONTEXT_REGISTRY, DBGPRINT_LOGGER, DRIVER_CONTEXT};
use nt_string::{unicode_string::NtUnicodeStr, widestring::U16Str};
use wdrf::{
    context::ContextRegistry,
    logger::DbgPrintLogger,
    minifilter::{
        filter::{framework::MinifilterFramework, MinifilterFrameworkBuilder},
        structs::IRP_MJ_OPERATION_END,
    },
};
use wdrf_std::dbg_break;
use windows_sys::{
    Wdk::{
        Foundation::DRIVER_OBJECT,
        Storage::FileSystem::{
            Minifilters::{
                FltGetRequestorProcessId, FLT_CALLBACK_DATA, FLT_OPERATION_REGISTRATION,
                FLT_PREOP_CALLBACK_STATUS, FLT_PREOP_COMPLETE, FLT_PREOP_SUCCESS_NO_CALLBACK,
                FLT_RELATED_OBJECTS,
            },
            SyncTypeCreateSection,
        },
        System::SystemServices::PAGE_EXECUTE,
    },
    Win32::{
        Foundation::{
            NTSTATUS, STATUS_ACCESS_DENIED, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, UNICODE_STRING,
        },
        System::Diagnostics::Debug::CONTEXT,
    },
};

use maple::info;

pub mod communication;
pub mod global;
pub mod panic;

fn driver_main(driver: &mut DRIVER_OBJECT, _registry_path: &UNICODE_STRING) -> anyhow::Result<()> {
    dbg_break();
    init_logging()?;

    info!(name = "DriverMain", "Init driver something idk :(");

    let filter = setup_filter(driver)?;
    let communication = create_communication(filter.clone())?;

    DRIVER_CONTEXT.init(&CONTEXT_REGISTRY, || DriverContext {
        filter: filter.clone(),
        communication,
        test: None,
    })?;

    unsafe {
        info!("Starting the minifilter");
        filter.start_filtering()?;
    }

    Ok(())
}

fn init_logging() -> anyhow::Result<()> {
    //Init logging
    let logger = DbgPrintLogger::new()?;
    DBGPRINT_LOGGER.init(&CONTEXT_REGISTRY, move || logger)?;

    maple::consumer::set_global_consumer(DBGPRINT_LOGGER.get());

    Ok(())
}

const IRP_MJ_ACQUIRE_FOR_SECTION_SYNCHRONIZATION: u8 = 255;

unsafe extern "system" fn process_map_precreate(
    data: *mut FLT_CALLBACK_DATA,
    fltobjects: *const FLT_RELATED_OBJECTS,
    _completioncontext: *mut *mut ::core::ffi::c_void,
) -> FLT_PREOP_CALLBACK_STATUS {
    let param_block = &*(*data).Iopb;
    let section = &param_block.Parameters.AcquireForSectionSynchronization;

    if section.SyncType == SyncTypeCreateSection
        && (section.PageProtection & PAGE_EXECUTE) == PAGE_EXECUTE
    {
        let file_object = &*(*fltobjects).FileObject;
        let name = &file_object.FileName;
        let name = NtUnicodeStr::from_raw_parts(name.Buffer, name.Length, name.MaximumLength);

        let requestor_id = FltGetRequestorProcessId(data);

        let edge: &'static U16Str = nt_string::widestring::u16str!("edge");

        let is_edge = name
            .as_slice()
            .windows(edge.len())
            .any(|w| w == edge.as_slice());

        if is_edge {
            info!("## DENY AcquireForSectionSync, process: {name}, panrent {requestor_id}");
            let block = &mut (*data).IoStatus;
            block.Anonymous.Status = STATUS_ACCESS_DENIED;

            return FLT_PREOP_COMPLETE;
        } else {
            info!("AcquireForSectionSync, process: {name}, panrent {requestor_id}");
        }
    }

    return FLT_PREOP_SUCCESS_NO_CALLBACK;
}

fn setup_filter(driver: &mut DRIVER_OBJECT) -> anyhow::Result<()> {
    info!("Initializing the minifilter");

    MinifilterFrameworkBuilder::new(pre)
        .build_and_register(&CONTEXT_REGISTRY, driver)
        .unwrap();

    Ok(())
}

fn create_communication(filter: FltFilter) -> anyhow::Result<Communication> {
    info!("Creating communication ");

    Communication::try_create(filter).map_err(|e| {
        maple::error!("Failed to create communication: {:#?}", e);
        anyhow::Error::msg("Failed to create communication")
    })
}

///# Safety
///
/// Driver entry point
///
///
#[export_name = "DriverEntry"]
pub unsafe extern "system" fn driver_entry(
    driver: &mut DRIVER_OBJECT,
    registry_path: &UNICODE_STRING,
) -> NTSTATUS {
    match driver_main(driver, registry_path) {
        Ok(_) => STATUS_SUCCESS,
        Err(_) => {
            CONTEXT_REGISTRY.drop_self();
            STATUS_UNSUCCESSFUL
        }
    }
}

pub unsafe extern "system" fn minifilter_unload(_flags: u32) -> NTSTATUS {
    info!(name = "Unload", "Unloading callback called");

    maple::consumer::get_global_registry().disable_consumer();
    CONTEXT_REGISTRY.drop_self();
    STATUS_SUCCESS
}

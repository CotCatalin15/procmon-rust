#![no_std]

use communication::Communication;
use global::{DriverContext, CONTEXT_REGISTRY, DBGPRINT_LOGGER, DRIVER_CONTEXT};
use minifilter::ProcmonMinifilterPreOp;
use wdrf::{
    context::ContextRegistry,
    logger::DbgPrintLogger,
    minifilter::filter::{
        framework::MinifilterFramework,
        registration::{FltOperationEntry, FltOperationType},
        EmptyFltOperationsVisitor, FilterOperationVisitor, MinifilterFrameworkBuilder,
        UnloadStatus,
    },
};
use wdrf_std::{dbg_break, kmalloc::TaggedObject};
use windows_sys::{
    Wdk::Foundation::DRIVER_OBJECT,
    Win32::Foundation::{NTSTATUS, STATUS_SUCCESS, STATUS_UNSUCCESSFUL, UNICODE_STRING},
};

use maple::info;

pub mod communication;
pub mod global;
pub mod minifilter;
pub mod panic;

fn driver_main(driver: &mut DRIVER_OBJECT, _registry_path: &UNICODE_STRING) -> anyhow::Result<()> {
    dbg_break();
    init_logging()?;

    info!(name = "DriverMain", "Init driver something idk :(");

    setup_filter(driver)?;
    let communication = create_communication()?;

    DRIVER_CONTEXT.init(&CONTEXT_REGISTRY, || DriverContext { communication })?;

    unsafe {
        info!("Starting the minifilter");
        MinifilterFramework::start_filtering().unwrap();
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

fn setup_filter(driver: &mut DRIVER_OBJECT) -> anyhow::Result<()> {
    info!("Initializing the minifilter");

    let flt_operations = [FltOperationEntry::new(FltOperationType::Create, 0, false)];

    MinifilterFrameworkBuilder::new(ProcmonMinifilterPreOp {})
        .operations(&flt_operations)
        .filter(MinifilterUnloadStruct {}, true)
        .post(EmptyFltOperationsVisitor {})
        .build_and_register(&CONTEXT_REGISTRY, driver)
        .unwrap();

    Ok(())
}

fn create_communication() -> anyhow::Result<Communication> {
    info!("Creating communication ");

    Communication::try_create().map_err(|e| {
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

struct MinifilterUnloadStruct {}

impl FilterOperationVisitor for MinifilterUnloadStruct {
    fn unload(&self, mandatory: bool) -> UnloadStatus {
        info!(name = "Unload", "Unloading callback called");

        maple::consumer::get_global_registry().disable_consumer();
        CONTEXT_REGISTRY.drop_self();

        UnloadStatus::Unload
    }
}
impl TaggedObject for MinifilterUnloadStruct {}

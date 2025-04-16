#![no_std]
#![feature(let_chains)]
#![allow(unused_attributes)]

use communication::Communication;
use global::{DriverContext, CONTEXT_REGISTRY, DBGPRINT_LOGGER, DRIVER_CONTEXT};
use imports::DynFncImports;
use minifilter::{ProcmonMinifilterCallback, ProcmonMinifilterContext};
use pscollector::ProcessCollectorCache;
use wdrf::{
    context::ContextRegistry,
    logger::DbgPrintLogger,
    minifilter::filter::{
        builder::{MinifilterFrameworkBuilder, MinifilterOperationBuilder},
        framework::MinifilterFramework,
        registration::{FltOperationEntry, FltOperationType},
        FilterUnload, UnloadStatus,
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
pub mod imports;
pub mod minifilter;
pub mod panic;
pub mod pscollector;

fn driver_main(driver: &mut DRIVER_OBJECT, _registry_path: &UNICODE_STRING) -> anyhow::Result<()> {
    dbg_break();
    init_logging()?;

    info!(name = "DriverMain", "Init driver something idk :(");

    DynFncImports::try_load(&CONTEXT_REGISTRY)?;

    setup_filter(driver)?;
    let communication = create_communication()?;

    let cache = ProcessCollectorCache::try_create().unwrap();
    DRIVER_CONTEXT.init(&CONTEXT_REGISTRY, || DriverContext {
        communication,
        process_cache: cache,
    })?;

    DRIVER_CONTEXT.get().process_cache.try_start()?;

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

    let flt_operations = [
        FltOperationEntry::new(FltOperationType::Create, 0),
        FltOperationEntry::new(FltOperationType::Write, 0),
        FltOperationEntry::new(FltOperationType::Read, 0),
        FltOperationEntry::new(FltOperationType::Cleanup, 0),
        FltOperationEntry::new(FltOperationType::Close, 0),
        FltOperationEntry::new(FltOperationType::QueryFileInfo, 0),
        FltOperationEntry::new(FltOperationType::SetFileInfo, 0),
        FltOperationEntry::new(FltOperationType::AcquireForSectionSync, 0),
    ];

    MinifilterFrameworkBuilder::new_with_context(
        || {
            MinifilterOperationBuilder::new()
                .operation_with_postop(ProcmonMinifilterCallback, &flt_operations)?
                .build()
        },
        ProcmonMinifilterContext,
    )
    .map_err(|e| {
        maple::error!("Minifilter framework error: {e}");
        anyhow::Error::msg("Failed to create minifilter framework")
    })?
    .unload(MinifilterUnloadStruct)
    .build_and_register(&CONTEXT_REGISTRY, driver)
    .inspect_err(|e| maple::error!("Failed to create minifilter instance: {:?}", e))?;

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

struct MinifilterUnloadStruct;

impl FilterUnload for MinifilterUnloadStruct {
    type MinifilterContext = ProcmonMinifilterContext;

    fn call(minifilter_context: &'static Self::MinifilterContext, mandatory: bool) -> UnloadStatus {
        info!(name = "Unload", "Unloading callback called");

        DRIVER_CONTEXT.get().communication.stop();
        MinifilterFramework::unregister();

        if let Err(_) = DRIVER_CONTEXT.get().process_cache.try_stop() {
            maple::error!("Failed to stop process cache :(");
        }

        maple::consumer::get_global_registry().disable_consumer();
        CONTEXT_REGISTRY.drop_self();

        UnloadStatus::Unload
    }
}
impl TaggedObject for MinifilterUnloadStruct {}

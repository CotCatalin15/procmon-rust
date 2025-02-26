#![no_std]

use core::time::Duration;

use communication::Communication;
use global::{DriverContext, CONTEXT_REGISTRY, DBGPRINT_LOGGER, DRIVER_CONTEXT};
use imports::DynFncImports;
use kmum_common::{
    krnmsg::{KmMessageCommonHeader, KmMessageEventKind},
    serializable_ntstring::SerializableNtString,
    KmMessage,
};
use minifilter::ProcmonMinifilterPreOp;
use nt_string::unicode_string::NtUnicodeString;
use pscollector::ProcessCollectorCache;
use wdrf::{
    context::ContextRegistry,
    logger::DbgPrintLogger,
    minifilter::filter::{
        framework::MinifilterFramework, EmptyFltOperationsVisitor, FilterOperationVisitor,
        MinifilterFrameworkBuilder, UnloadStatus,
    },
};
use wdrf_std::{
    dbg_break,
    kmalloc::TaggedObject,
    sync::{arc::Arc, event::Event},
    sys::{event::EventType, WaitResponse, WaitableObject},
    thread::spawn,
};
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

fn test(stop_event: Arc<Event>) {
    return;
    loop {
        if let WaitResponse::Success = stop_event.wait_for(Duration::from_secs(5)) {
            return;
        }

        (0..200).for_each(|_| {
            let _ = &DRIVER_CONTEXT
                .get()
                .communication
                .try_send_event(KmMessage {
                    common: KmMessageCommonHeader {
                        operation: kmum_common::krnmsg::KmMessageOperationType::ProcessCreate,
                        timestamp: 0,
                        pid: 1234,
                        thread_id: 100,
                        class: 10,
                        result: STATUS_SUCCESS,
                        path: SerializableNtString::new(NtUnicodeString::try_from("Test").unwrap()),
                        duration: 0,
                        unique_pid: 0,
                    },
                    event: KmMessageEventKind::DummyEvent(),
                });
        });
    }
}

fn driver_main(driver: &mut DRIVER_OBJECT, _registry_path: &UNICODE_STRING) -> anyhow::Result<()> {
    dbg_break();
    init_logging()?;

    info!(name = "DriverMain", "Init driver something idk :(");

    DynFncImports::try_load(&CONTEXT_REGISTRY)?;

    setup_filter(driver)?;
    let communication = create_communication()?;

    /*
        let collector =
            ProcessCollector::try_create_with_registry(&CONTEXT_REGISTRY, ProcMonProcessFactory {})
                .map_err(|_| anyhow::Error::msg("Failed to create process collector"))?;

        collector
            .start()
            .map_err(|_| anyhow::Error::msg("Failed to start proc collector"))?;
    */

    let stop_event = Event::try_create_arc(EventType::Notification, false).unwrap();

    let cache = ProcessCollectorCache::try_create().unwrap();

    let stop_event_clone = stop_event.clone();
    DRIVER_CONTEXT.init(&CONTEXT_REGISTRY, || DriverContext {
        communication,
        stop_event,
        test_thread: Some(spawn(move || test(stop_event_clone)).unwrap()),
        process_cache: cache,
    })?;

    DRIVER_CONTEXT.get().process_cache.try_start().unwrap();

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

    let flt_operations = [/*FltOperationEntry::new(FltOperationType::Create, 0, false)*/];

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
    fn unload(&self, _mandatory: bool) -> UnloadStatus {
        info!(name = "Unload", "Unloading callback called");

        DRIVER_CONTEXT.get().stop_event.signal();
        let th = unsafe { DRIVER_CONTEXT.get_mut() }
            .test_thread
            .take()
            .unwrap();
        th.join();

        maple::consumer::get_global_registry().disable_consumer();
        CONTEXT_REGISTRY.drop_self();

        UnloadStatus::Unload
    }
}
impl TaggedObject for MinifilterUnloadStruct {}

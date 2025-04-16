#![allow(internal_features)]
#![feature(core_intrinsics)]
#![feature(iter_collect_into)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

mod app_ui;
mod communication;
mod event_storage;
mod filters;
mod notifier;
mod process;
mod process_cache;
mod services;

use app_ui::ProcmonUi;
use clap::Parser;
use clap::ValueEnum;
use communication::EventCommunication;
use communication::FakeCommunication;
use eframe::NativeOptions;
use egui::ViewportBuilder;
use event_storage::EventStorage;
use filters::SimpleFilter;
use flume::bounded;
use flume::unbounded;
use flume::Sender;
use kmum_common::KmMessage;
use notifier::NotificationBus;
use process::ProcessManager;
use procmon_core::communication::driver_communication::DriverCommunication;
use procmon_core::communication::CommunicationInterface;
use rayon::ThreadPoolBuilder;
use services::EventStorageService;
use services::IndexerController;
use std::num::NonZeroU32;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::info;
use tracing::Level;

#[derive(Debug, Clone, ValueEnum)]
enum CommunicationType {
    Driver,
    Fake,
    DriverTest,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct ProcmonArgs {
    #[arg(short, long, default_value = "fake")]
    communication: CommunicationType,

    #[arg(short, long, default_value = "4")]
    num_threads: NonZeroU32,
}

fn main() {
    let args = ProcmonArgs::parse();
    println!("Args: {:#?}", args);

    if args.num_threads.get() > 32 {
        print!("Max threads is 32");
        return;
    }

    let sub = tracing_subscriber::fmt()
        .with_ansi(false) // Disable ANSI color codes
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(sub).expect("Failed to sent global tracing subscriber");

    info!("Creating global rayon thread pool");

    let event_storage = Arc::new(EventStorage::new());

    let (event_sender, event_receiver) = unbounded();

    let event_storage_bus = NotificationBus::new();

    let event_storage_service = EventStorageService::new(
        event_receiver,
        event_storage_bus.clone(),
        event_storage.clone(),
        args.num_threads.get() as _,
    );

    let communication = create_event_communication(event_sender, &args);

    eframe::run_native(
        "Procmon in Rust",
        NativeOptions {
            viewport: ViewportBuilder {
                min_inner_size: Some([1000.0; 2].into()),
                ..ViewportBuilder::default()
            },
            ..NativeOptions::default()
        },
        Box::new(|_cc| {
            Ok(Box::new(ProcmonUi::new(
                event_storage.clone(),
                IndexerController::new(event_storage_bus, event_storage, 6),
            )))
        }),
    )
    .unwrap();
}

fn create_event_communication(
    event_sender: Sender<KmMessage>,
    args: &ProcmonArgs,
) -> EventCommunication {
    let num_threads = args.num_threads.get() as usize;
    match args.communication {
        CommunicationType::Driver => EventCommunication::new(
            event_sender,
            Arc::new(DriverCommunication::new()),
            num_threads,
        ),
        CommunicationType::Fake => EventCommunication::new(
            event_sender,
            Arc::new(FakeCommunication::new()),
            num_threads,
        ),
        CommunicationType::DriverTest => {
            let child = Command::new("procmon-tester.exe").spawn().unwrap();
            EventCommunication::new(
                event_sender,
                Arc::new(DriverCommunication::new_test(child.id() as _)),
                num_threads,
            )
        }
    }
}

/*
fn create_communication_and_process_manager(
    sender: Sender<KmMessage>,
    args: ProcmonArgs,
) -> (Arc<ProcessManager>, EventCommunication) {
    impl_create_communication_and_process_manager(FakeCommunication::new(), sender, args)
}

fn impl_create_communication_and_process_manager<C: CommunicationInterface>(
    communication: C,
    sender: Sender<KmMessage>,
    args: ProcmonArgs,
) -> (Arc<ProcessManager>, EventCommunication) {
    let communication = Arc::new(communication);

    let event_communication =
        EventCommunication::new(sender, communication.clone(), args.num_threads.get() as _);
    let process_manager = ProcessManager::new(move |message| {
        let result = communication.send_message_blocking(&message);

        match result {
            Ok(reply) => reply,
            Err(_) => None,
        }
    });

    (Arc::new(process_manager), event_communication)
}
*/

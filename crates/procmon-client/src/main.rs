#![allow(internal_features)]
#![feature(core_intrinsics)]

mod app;
mod client_runtime;
mod event_reader;
mod events_storage;
mod fake_communication;
mod process_cache;

use app::ProcmonApp;
use clap::Parser;
use clap::ValueEnum;
use client_runtime::ClientRuntime;
use eframe::NativeOptions;
use events_storage::EventStorage;
use kmum_common::KmMessage;
use std::num::NonZeroU32;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedReceiver;
use tracing::info;

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

    #[arg(short, long, default_value = "1")]
    num_threads: NonZeroU32,
}

fn main() {
    std::panic::set_hook(Box::new(|_| {
        std::intrinsics::breakpoint();
    }));

    let args = ProcmonArgs::parse();
    println!("Args: {:#?}", args);

    let sub = tracing_subscriber::fmt()
        .with_ansi(false) // Disable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(sub).expect("Failed to sent global tracing subscriber");

    info!("Starting client");

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .build()
        .unwrap();

    let _guard = rt.enter();

    let storage = EventStorage::default();

    let runtime = ClientRuntime::from_args(storage.clone(), &args);
    runtime.start();

    eframe::run_native(
        "Procmon in Rust",
        NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(ProcmonApp::new(runtime, storage)))),
    )
    .unwrap();
}

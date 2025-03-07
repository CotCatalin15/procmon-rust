#![allow(internal_features)]
#![feature(core_intrinsics)]

use tracing::info;

fn main() {
    std::panic::set_hook(Box::new(|_info| core::intrinsics::breakpoint()));

    let sub = tracing_subscriber::fmt()
        .with_ansi(false) // Disable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(sub).expect("Failed to sent global tracing subscriber");

    info!("Starting client");

    procmon_core::test();
}

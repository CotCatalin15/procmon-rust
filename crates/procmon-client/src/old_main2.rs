use eframe::{egui, Frame};
use egui_extras::{Column, TableBuilder};
use event_reader::{Event, EventReader};
use std::time::Instant;

#[derive(Default)]
struct ProcMonApp {}

mod event_reader;

impl eframe::App for ProcMonApp {}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 800.0])
            .with_title("Process Monitor"),
        ..Default::default()
    };

    eframe::run_native(
        "Process Monitor",
        options,
        Box::new(|_cc| Ok(Box::<ProcMonApp>::default())),
    )
}

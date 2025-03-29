use std::{sync::Arc, time::Duration};

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use eframe::Frame;
use egui_extras::{Column, TableBuilder};
use kmum_common::event::{
    EventClass, EventFileSystemOperation, EventProcessOperation, EventRegistryOperation,
};

use crate::{
    client_runtime::ClientRuntime, events_storage::EventStorage, process_cache::ProcessCache,
};

pub struct ProcmonApp {
    runtime: ClientRuntime,
    storage: EventStorage,
}

impl Drop for ProcmonApp {
    fn drop(&mut self) {
        self.runtime.stop();
    }
}

impl ProcmonApp {
    pub fn new(runtime: ClientRuntime, storage: EventStorage) -> Self {
        Self { runtime, storage }
    }
}

impl eframe::App for ProcmonApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(Column::auto().at_least(50.0).resizable(true)) //id
                .column(Column::auto().at_least(100.0).resizable(true)) //timestamp
                .column(Column::auto().at_least(100.0).resizable(true)) //operation
                .column(Column::auto().at_least(100.0).resizable(true)) //process
                .column(Column::auto().at_least(100.0).resizable(true)) //pid
                .column(Column::remainder()) //path
                .header(25.0, |mut header| {
                    for name in ["ID", "TIMESTAMP", "OPERATION", "PROCESS", "PID", "PATH"] {
                        header.col(|ui| {
                            ui.label(name);
                        });
                    }
                })
                .body(|body| {
                    body.rows(25.0, self.storage.len(), |mut row| {
                        let index = row.index();

                        self.storage.read(index, |event| {
                            //id
                            row.col(|ui| {
                                ui.label(format!("{}", index));
                            });

                            //timepstamp
                            row.col(|ui| {
                                ui.label(format!(
                                    "{}",
                                    Self::filetime_to_datetime(event.event.date)
                                ));
                            });

                            //operations
                            row.col(|ui| {
                                ui.label(Self::event_operation_to_str(&event.event.operation));
                            });

                            //process
                            row.col(|ui| {
                                let hit = self.runtime.cache().try_get_and(
                                    event.process.unique_id,
                                    |info| match info {
                                        Some(info) => {
                                            ui.label(format!("{}", info));
                                        }
                                        None => {
                                            ui.label("Unknown");
                                        }
                                    },
                                );
                                if !hit {
                                    ui.label("Loading...");
                                }
                            });

                            //pid
                            row.col(|ui| {
                                ui.label(format!("{}", event.process.pid));
                            });

                            //path
                            row.col(|ui| {
                                ui.label(format!("{}", event.event.path));
                            });
                        });
                    });
                });
        });
    }
}

impl ProcmonApp {
    fn event_operation_to_str(operation: &EventClass) -> &'static str {
        match operation {
            EventClass::Process(event_process_operation) => {
                Self::process_op_to_str(event_process_operation)
            }
            EventClass::FileSystem(event_file_system_operation) => {
                Self::file_op_to_str(event_file_system_operation)
            }
            EventClass::Registry(event_registry_operation) => {
                Self::registry_op_to_str(event_registry_operation)
            }
        }
    }

    fn process_op_to_str(operation: &EventProcessOperation) -> &'static str {
        match operation {
            EventProcessOperation::ProcessCreate { .. } => "Process create",
            EventProcessOperation::ProcessDestroy { .. } => "Process destroy",
        }
    }

    fn file_op_to_str(operation: &EventFileSystemOperation) -> &'static str {
        match operation {
            EventFileSystemOperation::Create { .. } => "Create",
            EventFileSystemOperation::Read { .. } => "Read",
            EventFileSystemOperation::Write { .. } => "Write",
            EventFileSystemOperation::Close {} => "Close",
        }
    }

    fn registry_op_to_str(operation: &EventRegistryOperation) -> &'static str {
        match operation {
            EventRegistryOperation::Open() => "RegOpen",
        }
    }

    fn filetime_to_datetime(filetime: u64) -> DateTime<Utc> {
        // Windows FILETIME is 100-ns intervals since 1601-01-01
        // Unix timestamp is seconds since 1970-01-01
        // The difference between these dates is 11644473600 seconds

        const EPOCH_DIFFERENCE: u64 = 11_644_473_600;
        const HUNDRED_NS_PER_SEC: u64 = 10_000_000;

        // Convert 100-ns intervals to seconds since 1601
        let total_secs = filetime / HUNDRED_NS_PER_SEC;
        // Subtract epoch difference to get Unix timestamp
        let unix_secs = (total_secs - EPOCH_DIFFERENCE) as i64;
        // The remaining 100-ns intervals after seconds conversion
        let subsec_nanos = ((filetime % HUNDRED_NS_PER_SEC) * 100) as u32;

        // Create a NaiveDateTime from these values
        let naive =
            NaiveDateTime::from_timestamp_opt(unix_secs, subsec_nanos).expect("Invalid timestamp");

        // Convert to UTC DateTime
        Utc.from_utc_datetime(&naive)
    }
}

use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use eframe::Frame;
use egui_extras::{Column, TableBuilder};
use kmum_common::{
    event::{EventClass, EventFileSystemOperation, EventProcessOperation, EventRegistryOperation},
    KmMessage,
};

use crate::{
    event_storage::EventStorage,
    filters::SimpleFilter,
    process::ProcessManager,
    services::{IndexData, IndexerController},
};

struct IndexStorageView {
    view: Vec<IndexData>,
    start_view: usize,
    end_view: usize,
}

pub struct ProcmonUi {
    storage: Arc<EventStorage>,
    proc_manager: Arc<ProcessManager>,
    controller: IndexerController,
}

impl ProcmonUi {
    pub fn new(
        storage: Arc<EventStorage>,
        proc_manager: Arc<ProcessManager>,
        controller: IndexerController,
    ) -> Self {
        Self {
            storage,
            proc_manager,
            controller,
        }
    }
}

impl eframe::App for ProcmonUi {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        ctx.request_repaint_after_secs(1.0);

        let mut current_view = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            TableBuilder::new(ui)
                .stick_to_bottom(true)
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
                    body.rows(25.0, self.controller.num_events(), |mut row| {
                        let index = row.index();

                        if current_view.is_none() {
                            let end = (index + 50).min(self.controller.num_events());
                            let mut collection = Vec::default();

                            self.controller
                                .collect_indicies_into(index, end, &mut collection);

                            current_view = Some(IndexStorageView {
                                view: collection,
                                start_view: index,
                                end_view: end,
                            });
                        }

                        current_view
                            .as_ref()
                            .unwrap()
                            .read_event(index, &self.storage, |event| {
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
                                    let hit = false;
                                    /*self.runtime.cache().try_get_and(
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
                                    */

                                    let entry =
                                        self.proc_manager.try_get_async(event.process.unique_id);

                                    match entry {
                                        crate::process::ProcessCacheEntry::Hit(
                                            process_cache_information,
                                        ) => match process_cache_information {
                                            Some(info) => {
                                                ui.label(format!("{}", info.get_process_name()))
                                            }
                                            None => ui.label("Unknown"),
                                        },
                                        crate::process::ProcessCacheEntry::Miss => {
                                            ui.label("Loading...")
                                        }
                                    };
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

impl ProcmonUi {
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
            EventFileSystemOperation::Cleanup {} => "Cleanup",
            EventFileSystemOperation::Close {} => "Close",
            EventFileSystemOperation::QueryFileInfo { .. } => "QueryFileInfo",
            EventFileSystemOperation::SetFileInfo { .. } => "SetFileInfo",
            EventFileSystemOperation::AcquireForSectionSync { .. } => "AcquireForSectionSync",
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
        #[allow(deprecated)]
        let naive =
            NaiveDateTime::from_timestamp_opt(unix_secs, subsec_nanos).expect("Invalid timestamp");

        // Convert to UTC DateTime
        Utc.from_utc_datetime(&naive)
    }
}

impl IndexStorageView {
    pub fn read_event<R: FnOnce(&KmMessage)>(
        &self,
        index: usize,
        storage: &EventStorage,
        reader: R,
    ) {
        let view = self.view[index - self.start_view];
        storage.read_event(view.event_index, reader);
    }
}

use std::{sync::Arc, time::Duration};

use eframe::Frame;
use egui_extras::{Column, TableBuilder};

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
                .column(Column::auto().at_least(100.0).resizable(true)) //id
                .column(Column::auto().at_least(100.0).resizable(true)) //timestamp
                .column(Column::auto().at_least(100.0).resizable(true)) //process
                .column(Column::auto().at_least(100.0).resizable(true)) //pid
                .column(Column::remainder()) //path
                .header(25.0, |mut header| {
                    for name in ["ID", "TIMESTAMP", "PROCESS", "PID", "PATH"] {
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
                                ui.label(format!("{}", event.event.date));
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

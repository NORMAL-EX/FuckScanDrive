use crate::config::{BlockedDrives, Config, ProcessRule};
use crate::process_monitor::ProcessInfo;
use chrono::Local;
use eframe::egui;
use parking_lot::Mutex;
use std::sync::Arc;

#[derive(Clone)]
pub struct BlockedEvent {
    pub timestamp: String,
    pub process_name: String,
    pub pid: u32,
    pub action: String,
}

pub struct AppState {
    pub config: Arc<Mutex<Option<Config>>>,
    pub is_running: Arc<Mutex<bool>>,
    pub blocked_processes: Arc<Mutex<Vec<ProcessInfo>>>,
    pub blocked_events: Arc<Mutex<Vec<BlockedEvent>>>,
    pub status_message: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            config: Arc::new(Mutex::new(None)),
            is_running: Arc::new(Mutex::new(false)),
            blocked_processes: Arc::new(Mutex::new(Vec::new())),
            blocked_events: Arc::new(Mutex::new(Vec::new())),
            status_message: Arc::new(Mutex::new(String::from("Idle"))),
        }
    }

    pub fn add_blocked_event(&self, process_name: String, pid: u32, action: String) {
        let event = BlockedEvent {
            timestamp: Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            process_name,
            pid,
            action,
        };

        let mut events = self.blocked_events.lock();
        events.push(event);

        if events.len() > 1000 {
            events.drain(0..500);
        }
    }

    pub fn set_status(&self, message: String) {
        let mut status = self.status_message.lock();
        *status = message;
    }
}

pub struct FuckScanDriveApp {
    pub state: Arc<AppState>,
}

impl FuckScanDriveApp {
    pub fn new(_cc: &eframe::CreationContext<'_>, state: Arc<AppState>) -> Self {
        FuckScanDriveApp { state }
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.heading("FuckScanDrive");
        ui.separator();

        ui.horizontal(|ui| {
            let is_running = *self.state.is_running.lock();
            let status_color = if is_running {
                egui::Color32::GREEN
            } else {
                egui::Color32::RED
            };

            ui.colored_label(
                status_color,
                if is_running {
                    "● ACTIVE"
                } else {
                    "● STOPPED"
                },
            );

            ui.separator();

            let status = self.state.status_message.lock().clone();
            ui.label(format!("Status: {}", status));
        });

        ui.separator();
    }

    fn render_rules(&self, ui: &mut egui::Ui) {
        ui.heading("Protection Rules");

        let config_guard = self.state.config.lock();
        if let Some(config) = config_guard.as_ref() {
            egui::Grid::new("rules_grid")
                .striped(true)
                .min_col_width(150.0)
                .show(ui, |ui| {
                    ui.label("Process Name");
                    ui.label("Blocked Drives");
                    ui.end_row();

                    for rule in &config.rules {
                        ui.label(&rule.process_name);

                        let drives_str = match &rule.blocked_drives {
                            BlockedDrives::All => "All Drives".to_string(),
                            BlockedDrives::Specific(drives) => {
                                let mut sorted: Vec<_> = drives.iter().collect();
                                sorted.sort();
                                sorted
                                    .iter()
                                    .map(|d| format!("{}:", d))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            }
                        };

                        ui.label(drives_str);
                        ui.end_row();
                    }
                });
        } else {
            ui.label("No configuration loaded");
        }
    }

    fn render_blocked_processes(&self, ui: &mut egui::Ui) {
        ui.heading("Currently Blocked Processes");

        let blocked = self.state.blocked_processes.lock();

        if blocked.is_empty() {
            ui.label("No processes are currently blocked");
        } else {
            egui::Grid::new("blocked_processes_grid")
                .striped(true)
                .min_col_width(100.0)
                .show(ui, |ui| {
                    ui.label("PID");
                    ui.label("Process Name");
                    ui.label("Full Path");
                    ui.end_row();

                    for process in blocked.iter() {
                        ui.label(process.pid.to_string());
                        ui.label(&process.name);
                        ui.label(
                            process
                                .full_path
                                .as_ref()
                                .unwrap_or(&"Unknown".to_string()),
                        );
                        ui.end_row();
                    }
                });
        }
    }

    fn render_event_log(&self, ui: &mut egui::Ui) {
        ui.heading("Block Events");

        let events = self.state.blocked_events.lock();

        if events.is_empty() {
            ui.label("No events recorded");
        } else {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    egui::Grid::new("events_grid")
                        .striped(true)
                        .min_col_width(80.0)
                        .show(ui, |ui| {
                            ui.label("Time");
                            ui.label("PID");
                            ui.label("Process");
                            ui.label("Action");
                            ui.end_row();

                            for event in events.iter().rev().take(100) {
                                ui.label(&event.timestamp);
                                ui.label(event.pid.to_string());
                                ui.label(&event.process_name);
                                ui.label(&event.action);
                                ui.end_row();
                            }
                        });
                });
        }
    }

    fn render_statistics(&self, ui: &mut egui::Ui) {
        ui.heading("Statistics");

        ui.horizontal(|ui| {
            let config_guard = self.state.config.lock();
            let rule_count = config_guard
                .as_ref()
                .map(|c| c.rules.len())
                .unwrap_or(0);

            ui.label(format!("Rules: {}", rule_count));
            ui.separator();

            let blocked_count = self.state.blocked_processes.lock().len();
            ui.label(format!("Blocked Processes: {}", blocked_count));
            ui.separator();

            let event_count = self.state.blocked_events.lock().len();
            ui.label(format!("Total Events: {}", event_count));
        });
    }
}

impl eframe::App for FuckScanDriveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_header(ui);

                ui.add_space(10.0);
                self.render_statistics(ui);

                ui.add_space(10.0);
                ui.separator();
                self.render_rules(ui);

                ui.add_space(10.0);
                ui.separator();
                self.render_blocked_processes(ui);

                ui.add_space(10.0);
                ui.separator();
                self.render_event_log(ui);

                ui.add_space(20.0);
            });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

pub fn run_gui(state: Arc<AppState>) -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&[])
                    .unwrap_or_default()
            ),
        ..Default::default()
    };

    eframe::run_native(
        "FuckScanDrive - Anti Malicious Disk Scanner",
        options,
        Box::new(|cc| Ok(Box::new(FuckScanDriveApp::new(cc, state)))),
    )
}

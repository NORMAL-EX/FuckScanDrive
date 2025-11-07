use crate::app_config::AppConfig;
use crate::config::{BlockedDrives, Config};
use crate::process_monitor::ProcessInfo;
use eframe::egui;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem},
    TrayIcon, TrayIconBuilder,
};

#[derive(Debug, Clone)]
pub struct BlockStatistics {
    pub process_name: String,
    pub block_count: u64,
}

pub struct AppState {
    pub config: Arc<Mutex<Option<Config>>>,
    pub app_config: Arc<Mutex<AppConfig>>,
    pub is_running: Arc<Mutex<bool>>,
    pub blocked_processes: Arc<Mutex<Vec<ProcessInfo>>>,
    pub block_statistics: Arc<Mutex<HashMap<String, u64>>>,
    pub status_message: Arc<Mutex<String>>,
}

impl AppState {
    pub fn new() -> Self {
        let app_config = AppConfig::load();

        AppState {
            config: Arc::new(Mutex::new(None)),
            app_config: Arc::new(Mutex::new(app_config)),
            is_running: Arc::new(Mutex::new(false)),
            blocked_processes: Arc::new(Mutex::new(Vec::new())),
            block_statistics: Arc::new(Mutex::new(HashMap::new())),
            status_message: Arc::new(Mutex::new(String::from("Idle"))),
        }
    }

    pub fn increment_block_count(&self, process_name: String) {
        let logging_enabled = self.app_config.lock().logging_enabled;

        if logging_enabled {
            let mut stats = self.block_statistics.lock();
            *stats.entry(process_name.to_lowercase()).or_insert(0) += 1;
        }
    }

    pub fn get_statistics(&self) -> Vec<BlockStatistics> {
        let stats = self.block_statistics.lock();
        let mut result: Vec<BlockStatistics> = stats
            .iter()
            .map(|(name, count)| BlockStatistics {
                process_name: name.clone(),
                block_count: *count,
            })
            .collect();

        result.sort_by(|a, b| b.block_count.cmp(&a.block_count));
        result
    }

    pub fn clear_statistics(&self) {
        let mut stats = self.block_statistics.lock();
        stats.clear();
    }

    pub fn set_status(&self, message: String) {
        let mut status = self.status_message.lock();
        *status = message;
    }

    pub fn toggle_logging(&self) -> bool {
        let mut app_config = self.app_config.lock();
        app_config.logging_enabled = !app_config.logging_enabled;
        let new_state = app_config.logging_enabled;

        if let Err(e) = app_config.save() {
            eprintln!("Failed to save config: {}", e);
        }

        if !new_state {
            drop(app_config);
            self.clear_statistics();
        }

        new_state
    }

    pub fn is_logging_enabled(&self) -> bool {
        self.app_config.lock().logging_enabled
    }
}

pub struct FuckScanDriveApp {
    pub state: Arc<AppState>,
    pub show_window: Arc<Mutex<bool>>,
    tray_icon: Option<TrayIcon>,
    show_menu_id: String,
    exit_menu_id: String,
}

impl FuckScanDriveApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        state: Arc<AppState>,
        show_window: Arc<Mutex<bool>>,
    ) -> Self {
        let show_menu_id = "show_window".to_string();
        let exit_menu_id = "exit_app".to_string();

        let tray_icon = Self::create_tray_icon(&show_menu_id, &exit_menu_id).ok();

        FuckScanDriveApp {
            state,
            show_window,
            tray_icon,
            show_menu_id,
            exit_menu_id,
        }
    }

    fn create_tray_icon(show_id: &str, exit_id: &str) -> Result<TrayIcon, Box<dyn std::error::Error>> {
        let tray_menu = Menu::new();

        let show_item = MenuItem::with_id(show_id, "Show Main Window", true, None);
        let exit_item = MenuItem::with_id(exit_id, "Exit Protection", true, None);

        tray_menu.append(&show_item)?;
        tray_menu.append(&exit_item)?;

        let icon_rgba = vec![255u8; 32 * 32 * 4];
        let icon = tray_icon::Icon::from_rgba(icon_rgba, 32, 32)?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(tray_menu))
            .with_tooltip("FuckScanDrive - Protection Active")
            .with_icon(icon)
            .build()?;

        Ok(tray)
    }

    fn handle_tray_events(&mut self, ctx: &egui::Context) {
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id.0 == self.show_menu_id {
                *self.show_window.lock() = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            } else if event.id.0 == self.exit_menu_id {
                *self.state.is_running.lock() = false;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn render_header(&self, ui: &mut egui::Ui) {
        ui.heading("FuckScanDrive - Anti Malicious Disk Scanner");
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

    fn render_controls(&mut self, ui: &mut egui::Ui) {
        ui.heading("Controls");

        ui.horizontal(|ui| {
            let logging_enabled = self.state.is_logging_enabled();

            let button_text = if logging_enabled {
                "🟢 Logging: ON"
            } else {
                "🔴 Logging: OFF"
            };

            if ui.button(button_text).clicked() {
                let new_state = self.state.toggle_logging();
                self.state.set_status(format!(
                    "Logging {}",
                    if new_state { "enabled" } else { "disabled" }
                ));
            }

            ui.separator();

            if ui.button("Clear Statistics").clicked() {
                self.state.clear_statistics();
                self.state.set_status("Statistics cleared".to_string());
            }

            ui.separator();

            if ui.button("Hide to Tray").clicked() {
                *self.show_window.lock() = false;
            }
        });

        if !self.state.is_logging_enabled() {
            ui.colored_label(
                egui::Color32::YELLOW,
                "⚠ Logging is disabled. Enable it to track block statistics.",
            );
        }
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

    fn render_statistics(&self, ui: &mut egui::Ui) {
        ui.heading("Block Statistics");

        let stats = self.state.get_statistics();

        if stats.is_empty() {
            if self.state.is_logging_enabled() {
                ui.label("No blocks recorded yet");
            } else {
                ui.colored_label(
                    egui::Color32::GRAY,
                    "Logging disabled. No statistics available.",
                );
            }
        } else {
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    egui::Grid::new("stats_grid")
                        .striped(true)
                        .min_col_width(200.0)
                        .show(ui, |ui| {
                            ui.label("Process Name");
                            ui.label("Block Count");
                            ui.end_row();

                            for stat in stats.iter() {
                                ui.label(&stat.process_name);
                                ui.label(stat.block_count.to_string());
                                ui.end_row();
                            }
                        });
                });
        }
    }

    fn render_info_summary(&self, ui: &mut egui::Ui) {
        ui.heading("Summary");

        ui.horizontal(|ui| {
            let config_guard = self.state.config.lock();
            let rule_count = config_guard
                .as_ref()
                .map(|c| c.rules.len())
                .unwrap_or(0);

            ui.label(format!("Rules: {}", rule_count));
            ui.separator();

            let blocked_count = self.state.blocked_processes.lock().len();
            ui.label(format!("Active Blocks: {}", blocked_count));
            ui.separator();

            let total_blocks: u64 = self.state.block_statistics.lock().values().sum();
            ui.label(format!("Total Blocked: {}", total_blocks));
        });
    }
}

impl eframe::App for FuckScanDriveApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_tray_events(ctx);

        if ctx.input(|i| i.viewport().close_requested()) {
            *self.show_window.lock() = false;
        }

        let should_show = *self.show_window.lock();

        if !should_show {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
            return;
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_header(ui);

                ui.add_space(10.0);
                self.render_controls(ui);

                ui.add_space(10.0);
                ui.separator();
                self.render_info_summary(ui);

                ui.add_space(10.0);
                ui.separator();
                self.render_rules(ui);

                ui.add_space(10.0);
                ui.separator();
                self.render_blocked_processes(ui);

                ui.add_space(10.0);
                ui.separator();
                self.render_statistics(ui);

                ui.add_space(20.0);
            });
        });

        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        *self.state.is_running.lock() = false;
    }
}

pub fn run_gui(state: Arc<AppState>) -> Result<(), eframe::Error> {
    let show_window = Arc::new(Mutex::new(true));
    let show_window_clone = show_window.clone();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_min_inner_size([800.0, 600.0])
            .with_icon(eframe::icon_data::from_png_bytes(&[]).unwrap_or_default()),
        ..Default::default()
    };

    eframe::run_native(
        "FuckScanDrive - Anti Malicious Disk Scanner",
        options,
        Box::new(move |cc| {
            Ok(Box::new(FuckScanDriveApp::new(
                cc,
                state,
                show_window_clone,
            )))
        }),
    )
}

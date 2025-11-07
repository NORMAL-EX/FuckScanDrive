mod app_config;
mod config;
mod gui;
mod injector;
mod process_monitor;

use config::{BlockedDrives, Config};
use gui::{AppState, run_gui};
use injector::{get_dll_path, Injector};
use process_monitor::{ProcessInfo, ProcessMonitor, ProcessWatcher};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_WRITE};
use windows::core::PCWSTR;

const CONFIG_FILE: &str = "fuck.ini";

type SetBlockedDrivesFunc = extern "C" fn(*const u8, usize);

struct ProtectionEngine {
    config: Config,
    watcher: ProcessWatcher,
    dll_path: String,
    state: Arc<AppState>,
    injected_processes: HashSet<u32>,
}

impl ProtectionEngine {
    fn new(config: Config, state: Arc<AppState>) -> Result<Self, String> {
        let target_processes: Vec<String> = config
            .rules
            .iter()
            .map(|r| r.process_name.clone())
            .collect();

        let watcher = ProcessWatcher::new(target_processes);
        let dll_path = get_dll_path()?;

        Ok(ProtectionEngine {
            config,
            watcher,
            dll_path,
            state,
            injected_processes: HashSet::new(),
        })
    }

    fn configure_dll_for_process(&self, pid: u32, rule: &config::ProcessRule) -> Result<(), String> {
        let drives_to_block: Vec<char> = match &rule.blocked_drives {
            BlockedDrives::All => {
                let mut all_drives = Vec::new();
                for c in b'A'..=b'Z' {
                    all_drives.push(c as char);
                }
                all_drives
            }
            BlockedDrives::Specific(drives) => {
                drives.iter().flat_map(|s| s.chars()).collect()
            }
        };

        unsafe {
            let process_handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_OPERATION | PROCESS_VM_WRITE,
                false,
                pid,
            )
            .map_err(|e| format!("Failed to open process {}: {}", pid, e))?;

            if process_handle.is_invalid() {
                return Err(format!("Invalid process handle for PID {}", pid));
            }

            let dll_name: Vec<u16> = "fuck_scan_hook.dll"
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();

            let dll_handle = GetModuleHandleW(PCWSTR(dll_name.as_ptr()));

            if dll_handle.is_err() || dll_handle.as_ref().unwrap().is_invalid() {
                return Err("Hook DLL not loaded in target process".to_string());
            }

            let dll_handle = dll_handle.unwrap();

            let func_name = std::ffi::CString::new("set_blocked_drives")
                .map_err(|_| "Invalid function name")?;

            let func_addr = GetProcAddress(dll_handle, windows::core::PCSTR(func_name.as_ptr() as *const u8));

            if let Some(func_addr) = func_addr {
                let set_blocked_drives: SetBlockedDrivesFunc = std::mem::transmute(func_addr);

                let drive_bytes: Vec<u8> = drives_to_block.iter().map(|&c| c as u8).collect();
                set_blocked_drives(drive_bytes.as_ptr(), drive_bytes.len());
            } else {
                return Err("Failed to get set_blocked_drives function address".to_string());
            }
        }

        Ok(())
    }

    fn handle_target_process(&mut self, process: &ProcessInfo) {
        if self.injected_processes.contains(&process.pid) {
            return;
        }

        if let Some(rule) = self.config.find_rule(&process.name) {
            self.state.set_status(format!(
                "Detected target process: {} (PID: {})",
                process.name, process.pid
            ));

            match Injector::inject_dll(process.pid, &self.dll_path) {
                Ok(_) => {
                    thread::sleep(Duration::from_millis(200));

                    if let Err(e) = self.configure_dll_for_process(process.pid, rule) {
                        self.state.set_status(format!(
                            "Failed to configure DLL for {} (PID: {}): {}. Process continues without protection.",
                            process.name, process.pid, e
                        ));

                        self.injected_processes.insert(process.pid);
                    } else {
                        self.injected_processes.insert(process.pid);

                        let mut blocked = self.state.blocked_processes.lock();
                        blocked.push(process.clone());

                        self.state.increment_block_count(process.name.clone());

                        self.state.set_status(format!(
                            "Successfully injected and protecting {} (PID: {})",
                            process.name, process.pid
                        ));
                    }
                }
                Err(e) => {
                    self.state.set_status(format!(
                        "Injection failed for {} (PID: {}): {}. Process continues without protection.",
                        process.name, process.pid, e
                    ));

                    self.injected_processes.insert(process.pid);
                }
            }
        }
    }

    fn cleanup_dead_processes(&mut self) {
        self.injected_processes
            .retain(|&pid| ProcessMonitor::is_process_running(pid));

        let mut blocked = self.state.blocked_processes.lock();
        blocked.retain(|p| ProcessMonitor::is_process_running(p.pid));
    }

    fn run(&mut self) {
        *self.state.is_running.lock() = true;
        self.state.set_status("Protection engine started".to_string());

        loop {
            if !*self.state.is_running.lock() {
                break;
            }

            match self.watcher.check_and_get_targets() {
                Ok(targets) => {
                    for process in targets {
                        self.handle_target_process(&process);
                    }
                }
                Err(e) => {
                    self.state.set_status(format!("Error scanning processes: {}", e));
                }
            }

            self.cleanup_dead_processes();

            thread::sleep(Duration::from_millis(500));
        }

        self.state.set_status("Protection engine stopped".to_string());
    }
}

fn initialize_and_run() -> Result<(), String> {
    let config_path = Path::new(CONFIG_FILE);

    if !config_path.exists() {
        return Err(format!(
            "Configuration file '{}' not found. Please create it first.",
            CONFIG_FILE
        ));
    }

    let config = Config::load(config_path)?;

    let state = Arc::new(AppState::new());
    *state.config.lock() = Some(config.clone());

    let engine_state = state.clone();
    thread::spawn(move || {
        match ProtectionEngine::new(config, engine_state.clone()) {
            Ok(mut engine) => {
                engine.run();
            }
            Err(e) => {
                engine_state.set_status(format!("Failed to start protection engine: {}", e));
            }
        }
    });

    run_gui(state).map_err(|e| format!("GUI error: {}", e))?;

    Ok(())
}

fn main() {
    if let Err(e) = initialize_and_run() {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}

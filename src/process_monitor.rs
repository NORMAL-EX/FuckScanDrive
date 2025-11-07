use std::collections::HashSet;
use std::mem;
use windows::Win32::Foundation::{CloseHandle, MAX_PATH};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_QUERY_INFORMATION, PROCESS_TERMINATE,
    PROCESS_VM_READ,
};
use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub full_path: Option<String>,
}

pub struct ProcessMonitor {
    known_pids: HashSet<u32>,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        ProcessMonitor {
            known_pids: HashSet::new(),
        }
    }

    pub fn enumerate_processes(&self) -> Result<Vec<ProcessInfo>, String> {
        let mut processes = Vec::new();

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
                .map_err(|e| format!("Failed to create process snapshot: {}", e))?;

            if snapshot.is_invalid() {
                return Err("Invalid snapshot handle".to_string());
            }

            let mut entry: PROCESSENTRY32W = mem::zeroed();
            entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;

            if Process32FirstW(snapshot, &mut entry).is_ok() {
                loop {
                    let name = String::from_utf16_lossy(
                        &entry.szExeFile[..entry
                            .szExeFile
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(entry.szExeFile.len())],
                    );

                    let full_path = Self::get_process_full_path(entry.th32ProcessID);

                    processes.push(ProcessInfo {
                        pid: entry.th32ProcessID,
                        name: name.to_lowercase(),
                        full_path,
                    });

                    if Process32NextW(snapshot, &mut entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        Ok(processes)
    }

    fn get_process_full_path(pid: u32) -> Option<String> {
        unsafe {
            let handle = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                pid,
            );

            if let Ok(handle) = handle {
                if !handle.is_invalid() {
                    let mut buffer = vec![0u16; MAX_PATH as usize];
                    let len = GetModuleFileNameExW(handle, None, &mut buffer);

                    let _ = CloseHandle(handle);

                    if len > 0 {
                        let path = String::from_utf16_lossy(&buffer[..len as usize]);
                        return Some(path);
                    }
                }
            }
        }

        None
    }

    pub fn scan_for_new_processes(&mut self) -> Result<Vec<ProcessInfo>, String> {
        let all_processes = self.enumerate_processes()?;
        let current_pids: HashSet<u32> = all_processes.iter().map(|p| p.pid).collect();

        let new_processes: Vec<ProcessInfo> = all_processes
            .into_iter()
            .filter(|p| !self.known_pids.contains(&p.pid))
            .collect();

        self.known_pids = current_pids;

        Ok(new_processes)
    }

    pub fn terminate_process(pid: u32) -> Result<(), String> {
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, false, pid)
                .map_err(|e| format!("Failed to open process {}: {}", pid, e))?;

            if handle.is_invalid() {
                return Err(format!("Invalid handle for process {}", pid));
            }

            let result = TerminateProcess(handle, 1);

            let _ = CloseHandle(handle);

            result.map_err(|e| format!("Failed to terminate process {}: {}", pid, e))
        }
    }

    pub fn is_process_running(pid: u32) -> bool {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_INFORMATION, false, pid);

            if let Ok(handle) = handle {
                if !handle.is_invalid() {
                    let _ = CloseHandle(handle);
                    return true;
                }
            }
        }

        false
    }

    pub fn find_process_by_name(&self, name: &str) -> Result<Vec<ProcessInfo>, String> {
        let all_processes = self.enumerate_processes()?;
        let normalized_name = name.to_lowercase();

        Ok(all_processes
            .into_iter()
            .filter(|p| p.name == normalized_name)
            .collect())
    }

    pub fn reset_known_pids(&mut self) {
        self.known_pids.clear();
    }
}

pub struct ProcessWatcher {
    monitor: ProcessMonitor,
    target_processes: HashSet<String>,
}

impl ProcessWatcher {
    pub fn new(target_processes: Vec<String>) -> Self {
        let target_set: HashSet<String> = target_processes
            .into_iter()
            .map(|s| s.to_lowercase())
            .collect();

        ProcessWatcher {
            monitor: ProcessMonitor::new(),
            target_processes: target_set,
        }
    }

    pub fn update_targets(&mut self, target_processes: Vec<String>) {
        self.target_processes = target_processes
            .into_iter()
            .map(|s| s.to_lowercase())
            .collect();
    }

    pub fn check_and_get_targets(&mut self) -> Result<Vec<ProcessInfo>, String> {
        let new_processes = self.monitor.scan_for_new_processes()?;

        let targets: Vec<ProcessInfo> = new_processes
            .into_iter()
            .filter(|p| self.target_processes.contains(&p.name))
            .collect();

        Ok(targets)
    }

    pub fn get_all_running_targets(&self) -> Result<Vec<ProcessInfo>, String> {
        let all_processes = self.monitor.enumerate_processes()?;

        Ok(all_processes
            .into_iter()
            .filter(|p| self.target_processes.contains(&p.name))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_processes() {
        let monitor = ProcessMonitor::new();
        let processes = monitor.enumerate_processes().unwrap();
        assert!(!processes.is_empty());
    }

    #[test]
    fn test_find_process_by_name() {
        let monitor = ProcessMonitor::new();
        let result = monitor.find_process_by_name("nonexistent.exe");
        assert!(result.is_ok());
    }
}

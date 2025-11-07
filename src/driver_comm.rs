use std::ffi::OsStr;
use std::iter::once;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;

const FUCKSCAN_PORT_NAME: &str = "\\FuckScanPort";
const MAX_PROCESS_NAME_LEN: usize = 256;

#[repr(C)]
#[derive(Debug, Clone)]
pub struct BlockedProcessRule {
    pub process_name: [u16; MAX_PROCESS_NAME_LEN],
    pub block_all_drives: u32,
    pub blocked_drives: [u16; 26],
    pub blocked_drive_count: u32,
}

pub struct DriverCommunicator {
    port_handle: HANDLE,
}

impl DriverCommunicator {
    pub fn new() -> Result<Self, String> {
        let port_name: Vec<u16> = OsStr::new(FUCKSCAN_PORT_NAME)
            .encode_wide()
            .chain(once(0))
            .collect();

        unsafe {
            let mut port_handle: HANDLE = HANDLE::default();

            let result = FilterConnectCommunicationPort(
                PCWSTR(port_name.as_ptr()),
                0,
                ptr::null(),
                0,
                ptr::null_mut(),
                &mut port_handle,
            );

            if result != 0 {
                return Err(format!("Failed to connect to driver port: 0x{:X}", result));
            }

            Ok(DriverCommunicator { port_handle })
        }
    }

    pub fn send_rule(&self, process_name: &str, block_all: bool, blocked_drives: &[char]) -> Result<(), String> {
        let mut rule = BlockedProcessRule {
            process_name: [0u16; MAX_PROCESS_NAME_LEN],
            block_all_drives: if block_all { 1 } else { 0 },
            blocked_drives: [0u16; 26],
            blocked_drive_count: 0,
        };

        let process_wide: Vec<u16> = OsStr::new(process_name).encode_wide().collect();
        let copy_len = process_wide.len().min(MAX_PROCESS_NAME_LEN - 1);
        rule.process_name[..copy_len].copy_from_slice(&process_wide[..copy_len]);

        for (i, &drive) in blocked_drives.iter().enumerate().take(26) {
            rule.blocked_drives[i] = drive as u16;
            rule.blocked_drive_count += 1;
        }

        unsafe {
            let mut bytes_returned: u32 = 0;

            let result = FilterSendMessage(
                self.port_handle,
                &rule as *const _ as *const _,
                mem::size_of::<BlockedProcessRule>() as u32,
                ptr::null_mut(),
                0,
                &mut bytes_returned,
            );

            if result != 0 {
                return Err(format!("Failed to send rule to driver: 0x{:X}", result));
            }
        }

        Ok(())
    }

    pub fn is_driver_loaded() -> bool {
        Self::new().is_ok()
    }
}

impl Drop for DriverCommunicator {
    fn drop(&mut self) {
        unsafe {
            if !self.port_handle.is_invalid() {
                let _ = CloseHandle(self.port_handle);
            }
        }
    }
}

extern "system" {
    fn FilterConnectCommunicationPort(
        lpPortName: PCWSTR,
        dwOptions: u32,
        lpContext: *const std::ffi::c_void,
        wSizeOfContext: u16,
        lpSecurityAttributes: *mut std::ffi::c_void,
        hPort: *mut HANDLE,
    ) -> i32;

    fn FilterSendMessage(
        hPort: HANDLE,
        lpInBuffer: *const std::ffi::c_void,
        dwInBufferSize: u32,
        lpOutBuffer: *mut std::ffi::c_void,
        dwOutBufferSize: u32,
        lpBytesReturned: *mut u32,
    ) -> i32;
}

pub fn install_driver() -> Result<(), String> {
    use std::process::Command;

    let inf_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe directory")?
        .join("driver")
        .join("FuckScanFilter.inf");

    if !inf_path.exists() {
        return Err(format!("Driver INF not found at: {:?}", inf_path));
    }

    let output = Command::new("rundll32.exe")
        .args(&[
            "setupapi.dll,InstallHinfSection",
            "DefaultInstall",
            "132",
            inf_path.to_str().unwrap(),
        ])
        .output()
        .map_err(|e| format!("Failed to execute rundll32: {}", e))?;

    if !output.status.success() {
        return Err("Driver installation failed".to_string());
    }

    let output = Command::new("sc.exe")
        .args(&["start", "FuckScanFilter"])
        .output()
        .map_err(|e| format!("Failed to start driver: {}", e))?;

    if !output.status.success() {
        return Err("Failed to start driver service".to_string());
    }

    Ok(())
}

pub fn uninstall_driver() -> Result<(), String> {
    use std::process::Command;

    let _ = Command::new("sc.exe")
        .args(&["stop", "FuckScanFilter"])
        .output();

    let inf_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get exe path: {}", e))?
        .parent()
        .ok_or("Failed to get exe directory")?
        .join("driver")
        .join("FuckScanFilter.inf");

    if inf_path.exists() {
        let _ = Command::new("rundll32.exe")
            .args(&[
                "setupapi.dll,InstallHinfSection",
                "DefaultUninstall",
                "132",
                inf_path.to_str().unwrap(),
            ])
            .output();
    }

    Ok(())
}

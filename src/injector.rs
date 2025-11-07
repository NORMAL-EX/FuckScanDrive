use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::Debug::WriteProcessMemory;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{
    VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
};
use windows::Win32::System::Threading::{
    CreateRemoteThread, OpenProcess, WaitForSingleObject, PROCESS_CREATE_THREAD,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_OPERATION, PROCESS_VM_READ, PROCESS_VM_WRITE,
};

pub struct Injector;

impl Injector {
    pub fn inject_dll<P: AsRef<Path>>(pid: u32, dll_path: P) -> Result<(), String> {
        let dll_path = dll_path.as_ref();

        if !dll_path.exists() {
            return Err(format!("DLL file not found: {:?}", dll_path));
        }

        let dll_path_wide: Vec<u16> = OsStr::new(dll_path.as_os_str())
            .encode_wide()
            .chain(once(0))
            .collect();

        let dll_path_size = dll_path_wide.len() * std::mem::size_of::<u16>();

        unsafe {
            let process_handle = OpenProcess(
                PROCESS_CREATE_THREAD
                    | PROCESS_QUERY_INFORMATION
                    | PROCESS_VM_OPERATION
                    | PROCESS_VM_WRITE
                    | PROCESS_VM_READ,
                false,
                pid,
            )
            .map_err(|e| format!("Failed to open process {}: {}", pid, e))?;

            if process_handle.is_invalid() {
                return Err(format!("Invalid process handle for PID {}", pid));
            }

            let result = Self::inject_internal(process_handle, &dll_path_wide, dll_path_size);

            let _ = CloseHandle(process_handle);

            result
        }
    }

    unsafe fn inject_internal(
        process_handle: HANDLE,
        dll_path_wide: &[u16],
        dll_path_size: usize,
    ) -> Result<(), String> {
        let remote_buffer = VirtualAllocEx(
            process_handle,
            None,
            dll_path_size,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );

        if remote_buffer.is_null() {
            return Err("Failed to allocate memory in target process".to_string());
        }

        let write_result = WriteProcessMemory(
            process_handle,
            remote_buffer,
            dll_path_wide.as_ptr() as *const _,
            dll_path_size,
            None,
        );

        if write_result.is_err() {
            VirtualFreeEx(process_handle, remote_buffer, 0, MEM_RELEASE);
            return Err("Failed to write DLL path to target process".to_string());
        }

        let kernel32_name: Vec<u16> = OsStr::new("kernel32.dll")
            .encode_wide()
            .chain(once(0))
            .collect();

        let kernel32_handle = GetModuleHandleW(PCWSTR(kernel32_name.as_ptr()))
            .map_err(|e| format!("Failed to get kernel32.dll handle: {}", e))?;

        if kernel32_handle.is_invalid() {
            VirtualFreeEx(process_handle, remote_buffer, 0, MEM_RELEASE);
            return Err("Invalid kernel32.dll handle".to_string());
        }

        let load_library_name = b"LoadLibraryW\0";
        let load_library_addr = GetProcAddress(kernel32_handle, windows::core::PCSTR(load_library_name.as_ptr()));

        if load_library_addr.is_none() {
            VirtualFreeEx(process_handle, remote_buffer, 0, MEM_RELEASE);
            return Err("Failed to get LoadLibraryW address".to_string());
        }

        let load_library_addr = load_library_addr.unwrap();

        let thread_handle = CreateRemoteThread(
            process_handle,
            None,
            0,
            Some(std::mem::transmute(load_library_addr)),
            Some(remote_buffer),
            0,
            None,
        );

        if let Ok(thread_handle) = thread_handle {
            if !thread_handle.is_invalid() {
                WaitForSingleObject(thread_handle, 5000);
                let _ = CloseHandle(thread_handle);
            }
        }

        VirtualFreeEx(process_handle, remote_buffer, 0, MEM_RELEASE);

        if thread_handle.is_err() {
            return Err("Failed to create remote thread".to_string());
        }

        Ok(())
    }

    pub fn is_dll_injected(pid: u32, dll_name: &str) -> bool {
        use windows::Win32::Foundation::MAX_PATH;
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Module32FirstW, Module32NextW, MODULEENTRY32W,
            TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32,
        };

        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32, pid);

            if snapshot.is_err() || snapshot.as_ref().unwrap().is_invalid() {
                return false;
            }

            let snapshot = snapshot.unwrap();

            let mut module_entry: MODULEENTRY32W = std::mem::zeroed();
            module_entry.dwSize = std::mem::size_of::<MODULEENTRY32W>() as u32;

            let dll_name_lower = dll_name.to_lowercase();

            if Module32FirstW(snapshot, &mut module_entry).is_ok() {
                loop {
                    let module_name = String::from_utf16_lossy(
                        &module_entry.szModule[..module_entry
                            .szModule
                            .iter()
                            .position(|&c| c == 0)
                            .unwrap_or(MAX_PATH as usize)],
                    )
                    .to_lowercase();

                    if module_name == dll_name_lower {
                        let _ = CloseHandle(snapshot);
                        return true;
                    }

                    if Module32NextW(snapshot, &mut module_entry).is_err() {
                        break;
                    }
                }
            }

            let _ = CloseHandle(snapshot);
        }

        false
    }
}

pub fn get_dll_path() -> Result<String, String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?;

    let exe_dir = exe_path
        .parent()
        .ok_or("Failed to get executable directory")?;

    let dll_path = exe_dir.join("fuck_scan_hook.dll");

    if !dll_path.exists() {
        return Err(format!(
            "Hook DLL not found at: {}",
            dll_path.display()
        ));
    }

    Ok(dll_path
        .to_str()
        .ok_or("Invalid DLL path")?
        .to_string())
}

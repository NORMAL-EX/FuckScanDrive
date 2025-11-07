use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashSet;
use std::ffi::OsString;
use std::mem;
use std::os::windows::ffi::OsStringExt;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HANDLE, NTSTATUS, STATUS_ACCESS_DENIED, UNICODE_STRING};
use windows::Win32::Storage::FileSystem::{FILE_ACCESS_FLAGS, FILE_SHARE_MODE};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Memory::{VirtualProtect, PAGE_EXECUTE_READWRITE, PAGE_PROTECTION_FLAGS};

static BLOCKED_DRIVES: Lazy<Mutex<HashSet<char>>> = Lazy::new(|| Mutex::new(HashSet::new()));
static HOOK_ENABLED: AtomicBool = AtomicBool::new(false);

#[repr(C)]
struct OBJECT_ATTRIBUTES {
    length: u32,
    root_directory: HANDLE,
    object_name: *mut UNICODE_STRING,
    attributes: u32,
    security_descriptor: *mut std::ffi::c_void,
    security_quality_of_service: *mut std::ffi::c_void,
}

#[repr(C)]
struct IO_STATUS_BLOCK {
    status: NTSTATUS,
    information: usize,
}

type NtCreateFileFunc = unsafe extern "system" fn(
    FileHandle: *mut HANDLE,
    DesiredAccess: FILE_ACCESS_FLAGS,
    ObjectAttributes: *mut OBJECT_ATTRIBUTES,
    IoStatusBlock: *mut IO_STATUS_BLOCK,
    AllocationSize: *mut i64,
    FileAttributes: u32,
    ShareAccess: FILE_SHARE_MODE,
    CreateDisposition: u32,
    CreateOptions: u32,
    EaBuffer: *mut std::ffi::c_void,
    EaLength: u32,
) -> NTSTATUS;

type NtOpenFileFunc = unsafe extern "system" fn(
    FileHandle: *mut HANDLE,
    DesiredAccess: FILE_ACCESS_FLAGS,
    ObjectAttributes: *mut OBJECT_ATTRIBUTES,
    IoStatusBlock: *mut IO_STATUS_BLOCK,
    ShareAccess: FILE_SHARE_MODE,
    OpenOptions: u32,
) -> NTSTATUS;

static mut ORIGINAL_NT_CREATE_FILE: Option<NtCreateFileFunc> = None;
static mut ORIGINAL_NT_OPEN_FILE: Option<NtOpenFileFunc> = None;

static mut NT_CREATE_FILE_TRAMPOLINE: [u8; 64] = [0; 64];
static mut NT_OPEN_FILE_TRAMPOLINE: [u8; 64] = [0; 64];

unsafe extern "system" fn hooked_nt_create_file(
    file_handle: *mut HANDLE,
    desired_access: FILE_ACCESS_FLAGS,
    object_attributes: *mut OBJECT_ATTRIBUTES,
    io_status_block: *mut IO_STATUS_BLOCK,
    allocation_size: *mut i64,
    file_attributes: u32,
    share_access: FILE_SHARE_MODE,
    create_disposition: u32,
    create_options: u32,
    ea_buffer: *mut std::ffi::c_void,
    ea_length: u32,
) -> NTSTATUS {
    if !HOOK_ENABLED.load(Ordering::Relaxed) {
        if let Some(original) = ORIGINAL_NT_CREATE_FILE {
            return original(
                file_handle,
                desired_access,
                object_attributes,
                io_status_block,
                allocation_size,
                file_attributes,
                share_access,
                create_disposition,
                create_options,
                ea_buffer,
                ea_length,
            );
        }
    }

    if !object_attributes.is_null() {
        let obj_attr = &*object_attributes;
        if !obj_attr.object_name.is_null() {
            let unicode_str = &*obj_attr.object_name;
            if !unicode_str.Buffer.is_null() && unicode_str.Length > 0 {
                let slice = std::slice::from_raw_parts(
                    unicode_str.Buffer.0,
                    (unicode_str.Length / 2) as usize,
                );

                let path = OsString::from_wide(slice);
                let path_str = path.to_string_lossy();

                if let Some(drive_letter) = extract_drive_letter(&path_str) {
                    let blocked = BLOCKED_DRIVES.lock();
                    if blocked.contains(&drive_letter) {
                        return STATUS_ACCESS_DENIED;
                    }
                }
            }
        }
    }

    if let Some(original) = ORIGINAL_NT_CREATE_FILE {
        return original(
            file_handle,
            desired_access,
            object_attributes,
            io_status_block,
            allocation_size,
            file_attributes,
            share_access,
            create_disposition,
            create_options,
            ea_buffer,
            ea_length,
        );
    }

    STATUS_ACCESS_DENIED
}

unsafe extern "system" fn hooked_nt_open_file(
    file_handle: *mut HANDLE,
    desired_access: FILE_ACCESS_FLAGS,
    object_attributes: *mut OBJECT_ATTRIBUTES,
    io_status_block: *mut IO_STATUS_BLOCK,
    share_access: FILE_SHARE_MODE,
    open_options: u32,
) -> NTSTATUS {
    if !HOOK_ENABLED.load(Ordering::Relaxed) {
        if let Some(original) = ORIGINAL_NT_OPEN_FILE {
            return original(
                file_handle,
                desired_access,
                object_attributes,
                io_status_block,
                share_access,
                open_options,
            );
        }
    }

    if !object_attributes.is_null() {
        let obj_attr = &*object_attributes;
        if !obj_attr.object_name.is_null() {
            let unicode_str = &*obj_attr.object_name;
            if !unicode_str.Buffer.is_null() && unicode_str.Length > 0 {
                let slice = std::slice::from_raw_parts(
                    unicode_str.Buffer.0,
                    (unicode_str.Length / 2) as usize,
                );

                let path = OsString::from_wide(slice);
                let path_str = path.to_string_lossy();

                if let Some(drive_letter) = extract_drive_letter(&path_str) {
                    let blocked = BLOCKED_DRIVES.lock();
                    if blocked.contains(&drive_letter) {
                        return STATUS_ACCESS_DENIED;
                    }
                }
            }
        }
    }

    if let Some(original) = ORIGINAL_NT_OPEN_FILE {
        return original(
            file_handle,
            desired_access,
            object_attributes,
            io_status_block,
            share_access,
            open_options,
        );
    }

    STATUS_ACCESS_DENIED
}

fn extract_drive_letter(path: &str) -> Option<char> {
    let path_upper = path.to_uppercase();

    if path_upper.len() >= 2 {
        let first_char = path_upper.chars().next()?;
        let second_char = path_upper.chars().nth(1)?;

        if first_char.is_ascii_alphabetic() && second_char == ':' {
            return Some(first_char);
        }
    }

    if path_upper.starts_with("\\??\\") && path_upper.len() >= 6 {
        let drive_char = path_upper.chars().nth(4)?;
        let colon = path_upper.chars().nth(5)?;

        if drive_char.is_ascii_alphabetic() && colon == ':' {
            return Some(drive_char);
        }
    }

    if path_upper.starts_with("\\DEVICE\\HARDDISKVOLUME") {
        return None;
    }

    if path_upper.starts_with("\\SYSTEMROOT\\") {
        return Some('C');
    }

    None
}

unsafe fn install_hook(
    target_func: *mut u8,
    hook_func: *const u8,
    trampoline: &mut [u8; 64],
) -> Result<(), String> {
    if target_func.is_null() {
        return Err("Target function is null".to_string());
    }

    let mut old_protect: PAGE_PROTECTION_FLAGS = PAGE_PROTECTION_FLAGS(0);
    VirtualProtect(
        target_func as *const _,
        64,
        PAGE_EXECUTE_READWRITE,
        &mut old_protect,
    )
    .map_err(|e| format!("Failed to change memory protection: {}", e))?;

    ptr::copy_nonoverlapping(target_func, trampoline.as_mut_ptr(), 64);

    let jmp_instruction: [u8; 12] = [
        0x48, 0xB8, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF, 0xE0,
    ];

    let mut patch = jmp_instruction;
    let hook_addr = hook_func as u64;
    patch[2..10].copy_from_slice(&hook_addr.to_le_bytes());

    ptr::copy_nonoverlapping(patch.as_ptr(), target_func, 12);

    VirtualProtect(
        target_func as *const _,
        64,
        old_protect,
        &mut old_protect,
    )
    .ok();

    Ok(())
}

unsafe fn get_ntdll_function(name: &str) -> Result<*mut u8, String> {
    let ntdll_name: Vec<u16> = "ntdll.dll"
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    let ntdll_handle = GetModuleHandleW(PCWSTR(ntdll_name.as_ptr()))
        .map_err(|e| format!("Failed to get ntdll.dll handle: {}", e))?;

    if ntdll_handle.is_invalid() {
        return Err("Invalid ntdll.dll handle".to_string());
    }

    let func_name_cstr = std::ffi::CString::new(name)
        .map_err(|_| "Invalid function name".to_string())?;

    let func_addr = GetProcAddress(ntdll_handle, windows::core::PCSTR(func_name_cstr.as_ptr() as *const u8));

    match func_addr {
        Some(addr) => Ok(addr as *mut u8),
        None => Err(format!("Function {} not found in ntdll.dll", name)),
    }
}

unsafe fn initialize_hooks() -> Result<(), String> {
    let nt_create_file_addr = get_ntdll_function("NtCreateFile")?;
    let nt_open_file_addr = get_ntdll_function("NtOpenFile")?;

    ORIGINAL_NT_CREATE_FILE = Some(mem::transmute(nt_create_file_addr as *const ()));
    ORIGINAL_NT_OPEN_FILE = Some(mem::transmute(nt_open_file_addr as *const ()));

    install_hook(
        nt_create_file_addr,
        hooked_nt_create_file as *const u8,
        &mut NT_CREATE_FILE_TRAMPOLINE,
    )?;

    install_hook(
        nt_open_file_addr,
        hooked_nt_open_file as *const u8,
        &mut NT_OPEN_FILE_TRAMPOLINE,
    )?;

    HOOK_ENABLED.store(true, Ordering::Relaxed);

    Ok(())
}

#[no_mangle]
pub extern "system" fn DllMain(
    _hinst_dll: *mut std::ffi::c_void,
    fdw_reason: u32,
    _lpv_reserved: *mut std::ffi::c_void,
) -> i32 {
    const DLL_PROCESS_ATTACH: u32 = 1;

    if fdw_reason == DLL_PROCESS_ATTACH {
        unsafe {
            if initialize_hooks().is_err() {
                return 0;
            }
        }
    }

    1
}

#[no_mangle]
pub extern "C" fn set_blocked_drives(drives: *const u8, count: usize) {
    if drives.is_null() || count == 0 {
        return;
    }

    unsafe {
        let drive_slice = std::slice::from_raw_parts(drives, count);
        let mut blocked = BLOCKED_DRIVES.lock();
        blocked.clear();

        for &drive_byte in drive_slice {
            let drive_char = (drive_byte as char).to_ascii_uppercase();
            if drive_char.is_ascii_alphabetic() {
                blocked.insert(drive_char);
            }
        }
    }

    HOOK_ENABLED.store(true, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn clear_blocked_drives() {
    let mut blocked = BLOCKED_DRIVES.lock();
    blocked.clear();
    HOOK_ENABLED.store(false, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn is_hook_active() -> i32 {
    if HOOK_ENABLED.load(Ordering::Relaxed) {
        1
    } else {
        0
    }
}

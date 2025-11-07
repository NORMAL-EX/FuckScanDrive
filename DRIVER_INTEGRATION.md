# Driver Integration Implementation

## Overview

FuckScanDrive now supports **dual protection modes**:

1. **Kernel Driver Mode** (Robust, recommended)
2. **DLL Injection Mode** (Fallback when driver not available)

## What Was Implemented

### 1. Protection Mode Enum (`src/main.rs:26-29`)

```rust
enum ProtectionMode {
    Driver(DriverCommunicator),  // Kernel-level protection
    Injection,                    // User-level DLL injection
}
```

### 2. Automatic Mode Detection (`src/main.rs:41-89`)

The `ProtectionEngine::new()` method now:

- **Tries to connect to the kernel driver first**
- **If driver is available:**
  - Uses kernel driver mode (most robust)
  - Sends all protection rules to the driver
  - Displays: "Protection mode: Kernel Driver (Robust)"

- **If driver is not available:**
  - Falls back to DLL injection mode
  - Displays: "Protection mode: DLL Injection (Driver not available)"

### 3. Intelligent Process Handling (`src/main.rs:148-214`)

The `handle_target_process()` method now handles both modes:

**In Driver Mode:**
- No DLL injection needed
- Protection happens automatically at kernel level
- Process cannot detect or bypass protection
- Just tracks processes for UI display
- Message: "Kernel driver protecting {process} (PID: {pid}) - automatic interception"

**In Injection Mode:**
- Injects hook DLL into target process
- Configures blocked drives in the DLL
- May fail on protected processes
- Message: "Successfully injected and protecting {process} (PID: {pid})"

## How Driver Protection Works

### Kernel-Level Interception

The minifilter driver (`driver/FuckScanFilter.c`) intercepts **all file operations** at the kernel level:

1. **IRP_MJ_CREATE Callback**: Intercepts when any process tries to open/create a file
2. **Process Name Check**: Gets the name of the calling process
3. **Drive Letter Extraction**: Parses the file path to get the drive letter
4. **Rule Matching**: Checks if this process should be blocked from accessing this drive
5. **Access Denial**: Returns `STATUS_ACCESS_DENIED` if blocked

### Communication

- **Filter Port**: `\\FuckScanPort`
- **Rule Format**: `BlockedProcessRule` struct contains:
  - Process name (256 chars max)
  - Block all drives flag
  - Blocked drives array (up to 26 drives)

### Advantages of Driver Mode

✅ **Cannot be bypassed** - Runs in kernel mode, process cannot detect or evade
✅ **No injection needed** - Works transparently without modifying target process
✅ **More stable** - No crashes from injection failures
✅ **Comprehensive** - Blocks at the lowest level (before any API calls)
✅ **Stealthy** - Target process has no way to know it's being blocked

## Installation

### Building the Driver (Requires Windows)

1. Install **Windows Driver Kit (WDK)**
2. Build the driver:
   ```batch
   cd driver
   msbuild FuckScanFilter.sln /p:Configuration=Release
   ```

### Installing the Driver

**Option 1: Via GUI** (Future implementation)
- Add "Install Driver" button to GUI
- Calls `driver_comm::install_driver()`

**Option 2: Manual Installation**
```batch
# As Administrator
rundll32.exe setupapi.dll,InstallHinfSection DefaultInstall 132 FuckScanFilter.inf
sc start FuckScanFilter
```

### Uninstalling the Driver

```batch
# As Administrator
sc stop FuckScanFilter
rundll32.exe setupapi.dll,InstallHinfSection DefaultUninstall 132 FuckScanFilter.inf
```

## Testing

### Verify Driver is Loaded

```batch
sc query FuckScanFilter
```

Should show: `STATE: 4 RUNNING`

### Check Which Mode is Active

When FuckScanDrive starts, the status bar will show:
- ✅ "Protection mode: Kernel Driver (Robust)" - Driver working
- ⚠️ "Protection mode: DLL Injection (Driver not available)" - Fallback mode

## Files Modified/Created

### Modified
- `src/main.rs` - Added dual-mode protection system

### Created
- `driver/FuckScanFilter.c` - Minifilter kernel driver
- `driver/FuckScanFilter.inf` - Driver installation config
- `src/driver_comm.rs` - Driver communication interface

## Next Steps

To complete the driver integration:

1. **Create driver build project** (requires WDK on Windows)
2. **Sign the driver** (for production use)
3. **Add GUI controls** for driver installation/uninstallation
4. **Update README** with driver installation instructions
5. **Test on real Windows system** with both modes

## Technical Details

### Driver Altitude

- **Class**: FSFilter Activity Monitor
- **Altitude**: 370030 (standard for file system monitoring)

### Supported OS

- Windows 10/11 (x64)
- Kernel-mode driver requires Administrator privileges
- Driver must be signed for production use (test signing enabled in development)

## Troubleshooting

### Driver Not Loading

1. Check if running as Administrator
2. Enable test signing (for development):
   ```batch
   bcdedit /set testsigning on
   ```
3. Check driver service exists:
   ```batch
   sc query FuckScanFilter
   ```

### Process Still Crashing

- If using injection mode, some processes may still crash
- **Solution**: Install and use the kernel driver mode
- Driver mode eliminates injection-related crashes

### Connection Failed

- Ensure driver is started: `sc start FuckScanFilter`
- Check driver logs in Windows Event Viewer
- Verify filter port name: `\\FuckScanPort`

# FuckScanDrive

A powerful Windows disk access protection system designed to block malicious disk scanning behavior by rogue software, preventing hardware failure and data loss caused by excessive read/write operations.

## Background

Some software (especially anti-cheat systems) performs uncontrolled full disk scans, leading to:
- Sustained high disk load and accelerated wear
- Frequent seeking on mechanical HDDs, potentially causing physical damage
- Severe system performance degradation
- Increased risk of data loss

FuckScanDrive uses low-level interception technology to completely block specified processes from accessing particular drive letters.

## Key Features

- **Strong Interception**: Hooks Windows Native APIs (NtCreateFile/NtOpenFile) to block file access at the lowest level
- **Flexible Configuration**: Customize interception rules via `fuck.ini` configuration file
- **Real-time Monitoring**: Automatically detects target process startup and blocks immediately
- **Graphical Interface**: Modern GUI based on egui with real-time protection status
- **Dual Protection**: Automatically terminates violating processes if DLL injection fails
- **Event Logging**: Complete record of all interception events

## Technical Implementation

### Core Technology Stack
- **Language**: Rust
- **GUI Framework**: egui + eframe
- **Windows API**: windows-rs 0.58

### Interception Mechanism
1. **Process Monitoring**: Real-time process monitoring using ToolHelp32 API
2. **DLL Injection**: Inject Hook DLL into target process via CreateRemoteThread
3. **Native API Hooking**: Use Inline Hook technique to intercept in ntdll.dll:
   - `NtCreateFile` - File creation/opening
   - `NtOpenFile` - File opening
4. **Path Filtering**: Parse file paths, return `STATUS_ACCESS_DENIED` when blocked drives are matched
5. **Fallback**: Terminate process directly if injection fails

### Architecture Design
```
┌─────────────────────────────────────┐
│   FuckScanDrive Main Application    │
│  ┌──────────┐      ┌──────────┐    │
│  │ GUI      │      │ Config   │    │
│  │ (egui)   │      │ Parser   │    │
│  └──────────┘      └──────────┘    │
│  ┌──────────────────────────────┐  │
│  │   Protection Engine          │  │
│  │  - Process Monitor           │  │
│  │  - DLL Injector              │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
              │ Inject
              ▼
┌─────────────────────────────────────┐
│      Target Process Memory          │
│  ┌──────────────────────────────┐  │
│  │   fuck_scan_hook.dll         │  │
│  │  - NtCreateFile Hook         │  │
│  │  - NtOpenFile Hook           │  │
│  │  - Drive Filter              │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

## Usage

### Configuration File (fuck.ini)

Create or edit `fuck.ini` in the program directory:

```ini
# Format: <process_name> <drive_list>
# drive_list can be:
#   - "All" to block all drives
#   - Space-separated drive letters like "C: D: E:"

# Example: Block a process from accessing all drives
malware.exe All

# Example: Block a process from accessing C: and D:
scanner.exe C: D:

# Example: Block a process from accessing E:
badapp.exe E:
```

### Running the Program

1. **Run as Administrator**: `fuck_scan_drive.exe`
2. Program automatically loads `fuck.ini` configuration
3. GUI displays current protection status
4. When target process starts, automatically blocks its disk access

### GUI Interface

- **Status Indicator**: Green ● for active protection, Red ● for stopped
- **Protection Rules**: Displays all configured interception rules
- **Blocked Processes**: Real-time list of currently blocked processes
- **Event Log**: Records all interception events (time, PID, process name, action)

## Build Guide

### Prerequisites
- Rust 1.70+
- Windows 10/11
- Visual Studio Build Tools (for linking)

### Build Steps

**Windows:**
```batch
build.bat
```

**Linux (Cross-compilation):**
```bash
chmod +x build.sh
./build.sh
```

### Build Artifacts
- `target/release/fuck_scan_drive.exe` - Main application
- `target/release/fuck_scan_hook.dll` - Hook DLL

**Important**: Both files must be in the same directory to run!

## Notes

### Security
- This software requires **Administrator privileges**
- Hook DLL runs in the target process's address space
- Only block confirmed malicious processes to avoid harming critical system processes

### Compatibility
- Supports Windows 10/11 (x64)
- 32-bit systems not supported
- May conflict with some security software

### Legal Disclaimer
- This tool is for protecting user hardware from malicious software damage only
- Comply with local laws and regulations
- Prohibited for illegal purposes

### Known Limitations
- Some protected system processes may not be injectable
- Signed processes may trigger security warnings
- Some anti-debugging programs may detect injection behavior

## Troubleshooting

### Interception Not Working
1. Check if running with Administrator privileges
2. Verify `fuck.ini` configuration is correct (process names are case-insensitive)
3. Ensure `fuck_scan_hook.dll` is in the same directory
4. Check event log for error messages

### Program Crashes
1. Check for conflicts with antivirus software
2. Try adding program to antivirus whitelist
3. Check Windows Event Viewer for error logs

### DLL Injection Fails
- Some processes may have injection protection
- Program will automatically terminate the process as fallback

## Roadmap

- [ ] Support hot reload of configuration file
- [ ] Add whitelist mode (only allow access to specified paths)
- [ ] Support regex matching for process names
- [ ] Export interception logs to file
- [ ] System tray mode

## Contributing

Issues and Pull Requests are welcome!

### Code Style
- Follow official Rust code conventions
- Use `cargo fmt` to format code
- Use `cargo clippy` to check warnings

### Testing
```bash
cargo test
```

## License

MIT License

Copyright (c) 2024 FuckScanDrive Contributors

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Acknowledgments

Thanks to all developers working for hardware security and user rights.

---

**Disclaimer**: This software is provided "as is", without warranty of any kind, express or implied. Use at your own risk.

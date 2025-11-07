# FuckScanDrive

[English](README_en.md) | [中文](README.md)

A powerful Windows disk access protection system designed to block malicious disk scanning behavior by rogue software, preventing hardware failure and data loss caused by excessive read/write operations.

---

## Why Do You Need This Tool?

In recent years, some "security software" has been scanning disks under the guise of anti-cheating. They scan hard drives crazily in the background, resulting in:
- Sustained high disk load and accelerated wear
- Frequent seeking on mechanical HDDs, potentially causing physical damage
- Severe system performance degradation, game stuttering, and frame drops
- Surge in SSD write volume and shortened lifespan
- Increased risk of data loss

### Real Case: The Helplessness of PC Manufacturers

In November 2024, a well-known PC manufacturer's official account published a video pointing out that a certain anti-cheat software "may not prevent cheating, but will definitely scan your hard drive to death". The video mentioned:
- The software performs crazy scans on the hard drive
- Causes massive memory usage
- Leads to game stuttering and frame drops
- **After-sales department received numerous complaints about hard drive failures caused by disk scanning**

However, shortly after the video was released, the manufacturer quickly deleted it and issued an apology statement, claiming the content was "not fact-checked" and contained "false evaluations".

**What's the truth behind this?**
- The manufacturer's technical team discovered real problems
- After-sales repair data showed abnormally high hard drive failure rates
- User complaints surged, repair costs increased dramatically
- But under certain pressure, they had to delete the video and apologize

**The Real Dilemma of Manufacturer After-Sales:**
> "Your hard drive is indeed broken, but it's not covered under warranty..."
> "We detected massive abnormal read/write operations, but it's caused by third-party software..."
> "We suggest you contact the game developer... oh wait, the anti-cheat software developer..."
> **"We're helpless too, but we can't say it out loud..."**

This is why we need FuckScanDrive—when even manufacturers dare not tell the truth, users can only protect their own hardware.

---

## Key Features

- **Strong Interception**: Hooks Windows Native APIs (NtCreateFile/NtOpenFile) to block file access at the lowest level
- **Flexible Configuration**: Customize interception rules via `fuck.ini` configuration file
- **Real-time Monitoring**: Automatically detects target process startup and blocks immediately
- **Graphical Interface**: Modern GUI based on egui with real-time protection status
- **Dual Protection**: Automatically terminates violating processes if DLL injection fails
- **Event Logging**: Complete record of all interception events

---

## Technical Implementation

### Core Technology Stack
- **Language**: Rust (Memory-safe, High-performance)
- **GUI Framework**: egui + eframe (Modern interface)
- **Windows API**: windows-rs 0.58 (Official Rust bindings)

### Interception Mechanism
1. **Process Monitoring**: Real-time process monitoring using ToolHelp32 API
2. **DLL Injection**: Inject Hook DLL into target process via CreateRemoteThread
3. **Native API Hooking**: Use Inline Hook technique to intercept in ntdll.dll:
   - `NtCreateFile` - File creation/opening (Lower level than CreateFile, harder to bypass)
   - `NtOpenFile` - File opening (Also Native API level)
4. **Path Filtering**: Parse file paths, return `STATUS_ACCESS_DENIED` when blocked drives are matched
5. **Fallback**: Terminate process directly if injection fails (No mercy)

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

---

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

# Example: Block a process from accessing E: (maybe your game drive)
badapp.exe E:

# Real-world scenario: Some "anti-cheat" software
# Note: Process names must match exactly (case-insensitive)
# some_anticheat.exe All
```

### Running the Program

1. **Run as Administrator**: `fuck_scan_drive.exe`
2. Program automatically loads `fuck.ini` configuration
3. GUI displays current protection status
4. When target process starts, automatically blocks its disk access
5. **It's recommended to run this program before starting games**

### GUI Interface

- **Status Indicator**:
  - 🟢 Green ● means protection is active and guarding your hard drive
  - 🔴 Red ● means stopped
- **Protection Rules**: Displays all configured interception rules
- **Blocked Processes**: Real-time list of currently blocked processes (PID, process name, full path)
- **Event Log**: Records all interception events (time, PID, process name, action type)

---

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

**⚠️ Important**: Both files must be in the same directory to run!

---

## Notes

### Security
- This software requires **Administrator privileges** (Must)
- Hook DLL runs in the target process's address space
- Only block confirmed harmful processes to avoid harming critical system processes
- **It's recommended to test first to ensure correct configuration before actual use**

### Compatibility
- ✅ Supports Windows 10/11 (x64)
- ❌ 32-bit systems not supported
- ⚠️ May conflict with some security software (but better than hard drive failure)

### Legal Disclaimer
- This tool is for protecting user hardware from malicious software damage only
- Users have the right to protect their hardware assets
- Comply with local laws and regulations
- Prohibited for illegal purposes
- **We're not targeting anyone, we're just protecting our own property**

### Known Limitations
- Some protected system processes may not be injectable
- Signed processes may trigger security warnings
- Some anti-debugging programs may detect injection behavior
- **For software that counterattacks after detecting injection, this program will directly terminate its process**

---

## Troubleshooting

### Interception Not Working
1. Check if running with Administrator privileges ✓
2. Verify `fuck.ini` configuration is correct (process names are case-insensitive) ✓
3. Ensure `fuck_scan_hook.dll` is in the same directory ✓
4. Check event log for error messages ✓
5. Confirm target process name matches exactly ✓

### Program Crashes
1. Check for conflicts with antivirus software
2. Try adding program to antivirus whitelist
3. Check Windows Event Viewer for error logs
4. **Some "security software" may interfere with this program's operation**

### DLL Injection Fails
- Some processes may have injection protection
- Program will automatically terminate the process as fallback
- Log will record: "Terminated (injection failed)"
- **This is expected behavior, your hard drive is protected**

### Hard Drive Still Being Scanned
- Check if process name is accurate (view via Task Manager)
- Some software may have multiple processes, need to add all to configuration file
- Check event log to confirm interception is working

---

## FAQ

**Q: Is this software legal?**
A: Completely legal. Users have the right to protect their hardware assets from damage. Just like you have the right to install a firewall.

**Q: Will it be detected?**
A: Possibly. Some software may detect DLL injection. But your hard drive will be protected, that's the point.

**Q: Does it affect game operation?**
A: No. We only intercept file access, not affecting normal game functions. With some "anti-cheat" systems blocked, games may even run smoother.

**Q: Why did the manufacturer apologize and call it "false information"?**
A: 😏 You know why.

**Q: What if my hard drive is already broken?**
A: Contact after-sales for repair. Although they may say "not covered under warranty", at least it can be documented. If similar problems increase, perhaps the truth will surface someday.

**Q: Why is the project named "FuckScanDrive"?**
A: Because we're really tired of endless disk scanning. When some software doesn't respect user hardware, we don't need to be polite either.

---

## Roadmap

- [ ] Support hot reload of configuration file
- [ ] Add whitelist mode (only allow access to specified paths)
- [ ] Support regex matching for process names
- [ ] Export interception logs to file (can be used as evidence for rights protection)
- [ ] System tray mode
- [ ] Hard drive health monitoring (detect abnormal read/write)
- [ ] Automatic backup function (when abnormal disk scanning is detected)

---

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

### Submit Issues
If you've encountered hard drive damage caused by disk scanning, welcome to submit an Issue to record:
- Hardware model
- Damage symptoms
- Time of occurrence
- Related software information

We will collect this information, perhaps one day it can form powerful evidence.

---

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

---

## Acknowledgments

Thanks to all developers working for hardware security and user rights.

Special thanks to the manufacturer's technical team who dared to tell the truth but was forced to apologize later—your video was deleted, but the truth won't be erased.

Thanks to all users who provided hard drive damage cases—your experiences made us realize the necessity of this tool.

---

## Related Links

- [Issue Tracker](https://github.com/NORMAL-EX/FuckScanDrive/issues) - Report issues and suggestions
- [Discussions](https://github.com/NORMAL-EX/FuckScanDrive/discussions) - Discussion and communication

---

**Disclaimer**: This software is provided "as is", without warranty of any kind, express or implied. Use at your own risk.

**Final Words**: If a software needs to "scan your hard drive crazily" to prevent cheating, perhaps the problem isn't with the players, but with the software design itself. We're just protecting our hardware while waiting for a more reasonable solution to emerge.

**Protect Your Hard Drive, Starting Now.**

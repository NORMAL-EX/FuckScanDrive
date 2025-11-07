# FuckScanDrive

一个强效的Windows磁盘访问保护系统，用于阻止流氓软件的恶意扫盘行为，防止硬盘因过度读写导致的故障和数据丢失。

## 项目背景

某些软件（尤其是反作弊系统）会进行无节制的全盘扫描，导致：
- 硬盘持续高负载，加速磨损
- 机械硬盘频繁寻道，可能导致物理损坏
- 系统性能严重下降
- 数据丢失风险增加

FuckScanDrive通过底层拦截技术，彻底阻止指定进程对特定盘符的访问。

## 主要特性

- **强效拦截**：Hook Windows Native API（NtCreateFile/NtOpenFile），在最底层阻止文件访问
- **灵活配置**：通过`fuck.ini`配置文件自定义拦截规则
- **实时监控**：自动检测目标进程启动并立即拦截
- **图形界面**：基于egui的现代化GUI，实时显示保护状态
- **双重保险**：DLL注入失败时自动强制终止违规进程
- **日志记录**：完整记录所有拦截事件

## 技术实现

### 核心技术栈
- **语言**：Rust
- **GUI框架**：egui + eframe
- **Windows API**：windows-rs 0.58

### 拦截机制
1. **进程监控**：使用ToolHelp32 API实时监控进程启动
2. **DLL注入**：通过CreateRemoteThread注入Hook DLL到目标进程
3. **Native API Hook**：使用Inline Hook技术拦截ntdll.dll中的：
   - `NtCreateFile` - 文件创建/打开
   - `NtOpenFile` - 文件打开
4. **路径过滤**：解析文件路径，匹配被禁止的盘符后返回`STATUS_ACCESS_DENIED`
5. **备用方案**：注入失败时直接终止进程

### 架构设计
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

## 使用方法

### 配置文件 (fuck.ini)

在程序目录下创建或编辑`fuck.ini`文件：

```ini
# 格式: <进程名> <盘符列表>
# 盘符列表可以是:
#   - "All" 表示禁止所有盘符
#   - 空格分隔的盘符列表，如 "C: D: E:"

# 示例：禁止某进程访问所有盘符
malware.exe All

# 示例：禁止某进程访问C盘和D盘
scanner.exe C: D:

# 示例：禁止某进程访问E盘
badapp.exe E:
```

### 运行程序

1. **以管理员权限运行** `fuck_scan_drive.exe`
2. 程序会自动加载`fuck.ini`配置
3. GUI界面显示当前保护状态
4. 当目标进程启动时，自动拦截其磁盘访问

### GUI界面说明

- **状态指示器**：绿色●表示保护已激活，红色●表示已停止
- **保护规则**：显示所有配置的拦截规则
- **被拦截进程**：实时显示当前被拦截的进程列表
- **事件日志**：记录所有拦截事件（时间、PID、进程名、操作）

## 构建指南

### 前置要求
- Rust 1.70+
- Windows 10/11
- Visual Studio Build Tools（用于链接）

### 构建步骤

**Windows:**
```batch
build.bat
```

**Linux (交叉编译):**
```bash
chmod +x build.sh
./build.sh
```

### 构建产物
- `target/release/fuck_scan_drive.exe` - 主程序
- `target/release/fuck_scan_hook.dll` - Hook DLL

**重要**：两个文件必须放在同一目录下运行！

## 注意事项

### 安全性
- 本软件需要**管理员权限**运行
- Hook DLL运行在目标进程的地址空间内
- 请仅拦截确认恶意的进程，避免误伤系统关键进程

### 兼容性
- 支持Windows 10/11（x64）
- 不支持32位系统
- 可能与某些安全软件冲突

### 法律声明
- 本工具仅用于保护用户硬件免受恶意软件损害
- 请遵守当地法律法规
- 禁止用于非法用途

### 已知限制
- 某些受保护的系统进程可能无法注入
- 已签名的进程可能触发安全警告
- 部分反调试程序可能检测到注入行为

## 故障排查

### 拦截不生效
1. 检查是否以管理员权限运行
2. 确认`fuck.ini`配置正确（进程名大小写不敏感）
3. 检查`fuck_scan_hook.dll`是否在同一目录
4. 查看事件日志中是否有错误信息

### 程序崩溃
1. 检查是否与杀毒软件冲突
2. 尝试添加程序到杀毒软件白名单
3. 检查Windows事件查看器中的错误日志

### DLL注入失败
- 某些进程可能有注入保护
- 此时程序会自动终止该进程作为备用方案

## 开发路线图

- [ ] 支持配置文件热重载
- [ ] 添加白名单模式（仅允许访问指定路径）
- [ ] 支持正则表达式匹配进程名
- [ ] 导出拦截日志为文件
- [ ] 系统托盘模式

## 贡献指南

欢迎提交Issue和Pull Request！

### 代码风格
- 遵循Rust官方代码规范
- 使用`cargo fmt`格式化代码
- 使用`cargo clippy`检查警告

### 测试
```bash
cargo test
```

## 许可证

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

## 致谢

感谢所有为硬件安全和用户权益而努力的开发者。

---

**免责声明**：本软件按"现状"提供，不提供任何明示或暗示的保证。使用本软件的风险由用户自行承担。

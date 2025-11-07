# FuckScanDrive

[English](README_en.md) | [中文](README.md)

一个强效的Windows磁盘访问保护系统，用于阻止流氓软件的恶意扫盘行为，防止硬盘因过度读写导致的故障和数据丢失。

---

## 为什么需要这个工具？

近年来，某些"安全软件"以反作弊之名，行扫盘之实。它们在后台疯狂扫描硬盘，导致：
- 硬盘持续高负载，加速磨损
- 机械硬盘频繁寻道，可能导致物理损坏
- 系统性能严重下降，游戏卡顿、掉帧
- 固态硬盘写入量激增，寿命缩短
- 数据丢失风险增加

### 真实案例：PC厂商的无奈

2024年11月，某知名PC制造商官方账号发布视频，指出某反作弊软件"可能防不住外挂，但一定会扫爆你的硬盘"。视频中提到：
- 该软件会对硬盘进行疯狂扫描
- 造成大量内存占用
- 导致游戏卡顿、掉帧
- **售后部门收到大量因扫盘导致的硬盘报废投诉**

然而，在视频发布后不久，该厂商迅速删除视频并发布道歉声明，称内容"未经事实查证"、存在"不实评价"。

**这背后的真相是什么？**
- 厂商技术团队发现了真实问题
- 售后维修数据显示硬盘故障率异常升高
- 用户投诉激增，维修成本大幅上升
- 但迫于某些压力，不得不删除视频并道歉

**厂商售后的真实困境：**
> "您的硬盘确实坏了，但不在保修范围内..."
> "我们检测到大量异常读写，但这是第三方软件造成的..."
> "建议您联系游戏开发商...哦不对，是那个反作弊软件的开发商..."
> **"我们也很无奈，但我们不能说..."**

这就是为什么我们需要FuckScanDrive——当厂商都不敢说真话时，用户只能自己保护自己的硬件。

---

## 主要特性

- **强效拦截**：Hook Windows Native API（NtCreateFile/NtOpenFile），在最底层阻止文件访问
- **灵活配置**：通过`fuck.ini`配置文件自定义拦截规则
- **实时监控**：自动检测目标进程启动并立即拦截
- **图形界面**：基于egui的现代化GUI，实时显示保护状态
- **双重保险**：DLL注入失败时自动强制终止违规进程
- **日志记录**：完整记录所有拦截事件

---

## 技术实现

### 核心技术栈
- **语言**：Rust（内存安全、高性能）
- **GUI框架**：egui + eframe（现代化界面）
- **Windows API**：windows-rs 0.58（官方Rust绑定）

### 拦截机制
1. **进程监控**：使用ToolHelp32 API实时监控进程启动
2. **DLL注入**：通过CreateRemoteThread注入Hook DLL到目标进程
3. **Native API Hook**：使用Inline Hook技术拦截ntdll.dll中的：
   - `NtCreateFile` - 文件创建/打开（比CreateFile更底层，更难绕过）
   - `NtOpenFile` - 文件打开（同样是Native API层级）
4. **路径过滤**：解析文件路径，匹配被禁止的盘符后返回`STATUS_ACCESS_DENIED`
5. **备用方案**：注入失败时直接终止进程（绝不手软）

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

---

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

# 示例：禁止某进程访问E盘（比如你的游戏盘）
badapp.exe E:

# 实战场景：某些"反作弊"软件
# 注意：进程名需要精确匹配（不区分大小写）
# some_anticheat.exe All
```

### 运行程序

1. **以管理员权限运行** `fuck_scan_drive.exe`
2. 程序会自动加载`fuck.ini`配置
3. GUI界面显示当前保护状态
4. 当目标进程启动时，自动拦截其磁盘访问
5. **建议在游戏启动前运行本程序**

### GUI界面说明

- **状态指示器**：
  - 🟢 绿色●表示保护已激活，正在守护你的硬盘
  - 🔴 红色●表示已停止
- **保护规则**：显示所有配置的拦截规则
- **被拦截进程**：实时显示当前被拦截的进程列表（PID、进程名、完整路径）
- **事件日志**：记录所有拦截事件（时间、PID、进程名、操作类型）

---

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

**⚠️ 重要**：两个文件必须放在同一目录下运行！

---

## 注意事项

### 安全性
- 本软件需要**管理员权限**运行（必须）
- Hook DLL运行在目标进程的地址空间内
- 请仅拦截确认有害的进程，避免误伤系统关键进程
- **建议先测试，确认配置正确后再实际使用**

### 兼容性
- ✅ 支持Windows 10/11（x64）
- ❌ 不支持32位系统
- ⚠️ 可能与某些安全软件冲突（但比硬盘报废好）

### 法律声明
- 本工具仅用于保护用户硬件免受恶意软件损害
- 用户有权保护自己的硬件资产
- 请遵守当地法律法规
- 禁止用于非法用途
- **我们不是针对谁，我们只是在保护自己的财产**

### 已知限制
- 某些受保护的系统进程可能无法注入
- 已签名的进程可能触发安全警告
- 部分反调试程序可能检测到注入行为
- **对于检测到注入后反制的软件，本程序会直接终止其进程**

---

## 故障排查

### 拦截不生效
1. 检查是否以管理员权限运行 ✓
2. 确认`fuck.ini`配置正确（进程名大小写不敏感）✓
3. 检查`fuck_scan_hook.dll`是否在同一目录 ✓
4. 查看事件日志中是否有错误信息 ✓
5. 确认目标进程名称完全匹配 ✓

### 程序崩溃
1. 检查是否与杀毒软件冲突
2. 尝试添加程序到杀毒软件白名单
3. 检查Windows事件查看器中的错误日志
4. **某些"安全软件"可能会干扰本程序运行**

### DLL注入失败
- 某些进程可能有注入保护
- 此时程序会自动终止该进程作为备用方案
- 日志会记录："Terminated (injection failed)"
- **这是预期行为，你的硬盘已受到保护**

### 硬盘仍然被扫描
- 检查进程名是否准确（可通过任务管理器查看）
- 某些软件可能有多个进程，需要全部添加到配置文件
- 查看事件日志确认拦截是否生效

---

## 常见问题 FAQ

**Q: 这个软件合法吗？**
A: 完全合法。用户有权保护自己的硬件资产不受损害。就像你有权安装防火墙一样。

**Q: 会不会被检测到？**
A: 可能会。某些软件可能检测到DLL注入。但你的硬盘会被保护，这是重点。

**Q: 影响游戏运行吗？**
A: 不会。我们只拦截文件访问，不影响游戏正常功能。某些"反作弊"系统被拦截后，游戏反而更流畅。

**Q: 为什么某厂商道歉了，还说是"不实信息"？**
A: 😏 你懂的。

**Q: 我的硬盘已经坏了怎么办？**
A: 联系售后维修。虽然他们可能会说"不在保修范围内"，但至少可以记录在案。如果类似问题多了，或许某天真相会浮出水面。

**Q: 为什么项目名叫"FuckScanDrive"？**
A: 因为我们真的厌倦了无休止的扫盘。当某些软件不尊重用户硬件时，我们也不需要客气。

---

## 开发路线图

- [ ] 支持配置文件热重载
- [ ] 添加白名单模式（仅允许访问指定路径）
- [ ] 支持正则表达式匹配进程名
- [ ] 导出拦截日志为文件（可用于维权证据）
- [ ] 系统托盘模式
- [ ] 硬盘健康监控（检测异常读写）
- [ ] 自动备份功能（在检测到异常扫盘时）

---

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

### 提交Issue
如果你遇到了因扫盘导致的硬盘损坏，欢迎提交Issue记录：
- 硬件型号
- 损坏现象
- 发生时间
- 相关软件信息

我们会收集这些信息，或许有一天能形成有力的证据。

---

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

---

## 致谢

感谢所有为硬件安全和用户权益而努力的开发者。

特别感谢那位敢于说真话、后来又被迫道歉的厂商技术团队——你们的视频虽然被删除了，但真相不会被抹去。

感谢所有提供硬盘损坏案例的用户——你们的经历让我们意识到这个工具的必要性。

---

## 相关链接

- [Issue Tracker](https://github.com/NORMAL-EX/FuckScanDrive/issues) - 报告问题和建议
- [Discussions](https://github.com/NORMAL-EX/FuckScanDrive/discussions) - 讨论和交流

---

**免责声明**：本软件按"现状"提供，不提供任何明示或暗示的保证。使用本软件的风险由用户自行承担。

**最后的话**：如果某个软件需要"疯狂扫描你的硬盘"才能防作弊，那或许问题不在玩家，而在软件设计本身。我们只是在等待一个更合理的解决方案出现之前，保护好自己的硬件。

**保护你的硬盘，从现在开始。**

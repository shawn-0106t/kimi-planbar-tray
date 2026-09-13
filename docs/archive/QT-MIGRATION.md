# Qt 技术栈迁移规划（QT-MIGRATION）

> 本文档规划 kimi-planbar-tray 的第三个实现版本：**C++ Qt6 + Qt Widgets** 版（`qt/` 目录）。
> 决策已定：UI 1:1 复刻现有设计（SPEC 第二篇数值契约不变）；`rust/` 保持活跃，与 `qt/` 双轨并行；`wpf/` 仍冻结只读。
> 本文档是规划与方案，不包含实现代码；Qt 版落地时按 Phase 0–5 执行，行为细节一律以 `docs/SPEC.md` 第二篇为契约。

---

## 1. GitHub 案例调研结论

检索目标：是否存在"Kimi 配额 + Qt"或同类的 Qt 托盘监控工具可直接复用。

### 1.1 结论：没有可直接套用的同类 case

- GitHub 上现有 Kimi quota 工具（`shawn-0106t/kimi-planbar-tray` 本身、`farion1231/cc-switch`、`Golden0Voyager/kimi-code-usage`、各 dsh/opencode 插件）均为 Tauri / TypeScript / CLI 实现，**没有 Qt 版本**。
- 业务逻辑（凭证链、`amountLeft` 1e-8 元换算、`isEnabled=false` 陷阱、刷新调度语义）只能从本仓库 `rust/` 移植，无外部来源。

### 1.2 可借鉴的参考项目

| 项目 | 栈 | 可借鉴点 | 局限 |
|---|---|---|---|
| [bozdemir/claude-usage-widget](https://github.com/bozdemir/claude-usage-widget)（MIT） | PySide6 | 同类业务（读本地 CLI OAuth token → 调官方 quota API → 无边框透明窗 + 托盘 + 轮询）；`FramelessWindowHint + WA_TranslucentBackground` + 自绘圆角进度条；daemon 线程轮询 + Qt Signal 回 GUI 线程；单实例锁；失败退避轮询；`QT_QPA_PLATFORM=offscreen` 无头测试思路 | Python 实现，代码不可直接复用；OSD 悬浮窗范式与本项目"托盘面板 + 4 窗口"不同 |
| [zhiyiYo/PyQt-Frameless-Window](https://github.com/zhiyiYo/PyQt-Frameless-Window) | PyQt/PySide | Win32 无边框窗效果处理清单（阴影、圆角、DPI 感知、DWM 属性） | Python 绑定；本项目窗口无需可缩放，只取思路 |
| [PsinaDev/pyside-frameless](https://github.com/PsinaDev/pyside-frameless) | PySide6 | DPI-aware hit-testing、Aero Snap 处理 | 同上 |

**可复用的是架构模式与 Win32 细节清单，不是代码。** Qt Widgets 的官方示例（`QSystemTrayIcon`、QSS 样式表、QPropertyAnimation）覆盖其余基础件。

---

## 2. Rust/Tauri 2 vs C++ Qt6 Widgets 对比

| 维度 | 现状：Rust/Tauri 2 | 目标：C++ Qt6 Widgets |
|---|---|---|
| UI 层 | WebView2 + HTML/CSS/TS（Vite 多页） | Qt Widgets + QSS（Qt Style Sheets，CSS 子集）+ QPainter 自绘，无 Web 依赖 |
| 前后端边界 | Tauri IPC（commands + events），前后端两语言 | 无 IPC：单语言单进程，信号槽直连，DTO 即 C++ struct |
| 运行时依赖 | WebView2 Runtime（Win11 自带，Win10 需装） | Qt6 DLL 随包分发（windeployqt），用户零依赖 |
| 产物形态 | 单 exe（~10 MB，不含 WebView2） | exe + DLL 目录（~15–25 MB）；静态单 exe 有 LGPL 合规负担，不推荐（见 4.8） |
| 常驻内存 | 4 个 WebView2 进程，~150–250 MB | 单进程 Widgets，~30–60 MB |
| 冷启动 | WebView2 初始化较慢 | 快（~100–200 ms），适合开机自启常驻 |
| 异步/HTTP | tokio + reqwest | QNetworkAccessManager（QtNetwork 自带，信号槽异步，免额外依赖） |
| JSON | serde_json | QJsonDocument / QJsonValue（QtCore 自带，零三方依赖） |
| 托盘 | tauri tray API | QSystemTrayIcon（Qt Widgets 模块自带） |
| 动画 | CSS transitions | QPropertyAnimation（opacity/geometry），SPEC 第 15 章时长照搬 |
| 主题 | CSS 变量（`theme.css`） | QSS 模板按主题生成切换（色值表照抄 SPEC 第 11 章） |
| 系统主题监听 | 注册表 + `WM_SETTINGCHANGE`（theme_watch.rs） | Qt ≥6.5：`QStyleHints::colorSchemeChanged`（Windows 上读的正是 `AppsUseLightTheme`） |
| DPI | tao 的 PhysicalPosition/LogicalSize 陷阱（SPEC 20） | Qt6 默认 PerMonitorV2，坐标统一逻辑像素，该陷阱天然不存在（见 4.5） |
| 单实例 | 命名 Mutex + tauri-plugin-single-instance | Win32 `CreateMutexW`，**同名 `KimiPlanbarTray.SingleInstance`** |
| 构建链 | cargo + npm + tauri CLI | CMake + MSVC（VS 生成工具 2026 已具备）+ Qt6 kit |
| 测试 | `cargo test` 仅 skills frontmatter；`--test-*` 无头自检 | Qt Test 可选；`--test-*` 无头自检同等实现（注意 4.6 控制台陷阱） |

**一句话总结**：Qt 版拿掉 WebView2 这层最重依赖，换来更低的内存与启动开销，代价是 UI 从"写 HTML/CSS"变为"写 C++ Widgets + QSS"，开发效率低于 Tauri 前端；对本项目这种小而稳的 UI，是值得的。

---

## 3. 模块映射（`rust/src-tauri/src/` → `qt/`）

Qt 版无前后端分离：Rust 后端模块与 TS 前端合并进同一 C++ 进程。目录与文件命名按下表（实现时可微调，保持与 SPEC 3.3 的可对照性）：

| Rust 模块 | Qt 版文件 | 落地方案 |
|---|---|---|
| `main.rs`（入口、`--test-*` 先于互斥锁） | `src/main.cpp` | 同样顺序：解析命令行 → 自检分支 → `CreateMutexW` 单实例 → QApplication |
| `lib.rs`（builder、IPC、窗口路由、DWM 圆角） | `src/app.h/.cpp` | 窗口单例管理、失焦/隐藏事件路由；`disable_dwm_corner_rounding` 平移（见 4.3） |
| `credentials.rs` | `src/credentials.h/.cpp` | 凭证链逻辑逐行平移；config.toml 仍手写逐行解析（Qt 无内置 TOML） |
| `quota.rs` | `src/quota.h/.cpp` | QNetworkAccessManager + QJsonDocument；防御性解析见 4.7 |
| `polling.rs` | `src/polling.h/.cpp` | QTimer 调度：2s 首刷、失败 30s 快重试、成功回正常周期、保旧值 |
| `tray.rs` | `src/trayicon.h/.cpp` | QSystemTrayIcon；tooltip 规则不变；hover-to-refresh 10s 节流（需 native event 或定时探测，见风险 7.3） |
| `panel.rs` | `src/panel.h/.cpp` | 面板/菜单定位与焦点行为、300ms 重入守卫；坐标全用逻辑像素（见 4.5） |
| `settings.rs` | `src/settings.h/.cpp` | settings.json 同 schema 同路径（QJsonDocument 读写）；HKCU Run 用 `QSettings("...\\Run", QSettings::NativeFormat)` |
| `skills.rs` | `src/skills.h/.cpp` | 只读扫描三处根目录、前 4 KiB、frontmatter 行解析、`fromUtf8` 容错、零后台开销——语义全平移 |
| `update.rs` | `src/update.h/.cpp` | QProcess 跑 `kimi --version`（5s 超时）；changelog `Range: bytes=0-4095` 请求 + GitHub API 兜底 |
| `theme_watch.rs` | —（并入 `app.cpp`） | `QStyleHints::colorSchemeChanged` 信号直驱主题切换（仅当设置为 system 时） |
| `state.rs` | `src/state.h/.cpp` | AppState：保旧值缓存、skills 缓存、手动刷新 2s 防抖 |
| 前端 4 页（`index/settings/skills/menu.html` + TS + CSS） | `src/windows/{panel,settings,skills,menu}window.h/.cpp` + `src/theme.cpp`（QSS 模板） | QWidget 子类 + 布局器 + QSS；SPEC 第二篇全部数值契约不变；hover 光晕用 QSS `:hover` + `QPropertyAnimation` 或 QGraphicsDropShadowEffect 近似（实际落地：`windows/hoverglow.h` 自绘，见第 9 章） |

前端安全约束平移：skills 名称/描述等外部数据在 Qt Widgets 下天然纯文本渲染（`QLabel::setText` 无 HTML 解析风险，注意不要用 `Qt::RichText`），与"禁 innerHTML"等价。

---

## 4. Qt 版关键技术方案（SPEC 陷阱的 Qt 落地）

### 4.1 无边框透明圆角窗（SPEC 10）
- 窗口 flags：`Qt::FramelessWindowHint | Qt::WindowStaysOnTopHint | Qt::Tool`；属性：`WA_TranslucentBackground`；`setAttribute(Qt::WA_ShowWithoutActivating)` 视焦点策略而定。
- 圆角外观由内部容器 QWidget 承担：容器 QSS `border-radius: 14px; background: <WindowBg>;`，顶层窗口透明——与 WPF/Web 的"Border + Margin 阴影空间"同构。
- 阴影：优先 QSS 容器外边距 + `QGraphicsDropShadowEffect`（BlurRadius 24 / Offset 2 / Opacity 0.25，SPEC 10.1 数值照搬）。
- **注意**：QSS `border-radius` 对顶层窗体本身不可靠，必须作用在子容器上（这是 Qt 版特有的实现要点）。

### 4.2 托盘（SPEC 14）
- `QSystemTrayIcon`，不设 `setContextMenu`（右键弹的是自定义菜单窗，不是 QMenu）。
- `activated(QSystemTrayIcon::Trigger)` → 左键 toggle 主面板；`activated(QSystemTrayIcon::Context)` → 右键：关旧菜单、新建菜单窗、光标处弹出。
- tooltip 文案规则不变；图标仍用 kimi-logo.png 包装的 ICO（QIcon 直接加载 PNG 亦可，托盘 ICO 兜底蓝球逻辑平移）。

### 4.3 Win11 圆角禁用（SPEC 20 / AGENTS 陷阱）
- 与 rust 版相同：`DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND)`，对每个窗口句柄执行一次；圆角由 QSS 自绘。

### 4.4 单实例
- Win32 `CreateMutexW(NULL, TRUE, L"KimiPlanbarTray.SingleInstance")` + `GetLastError() == ERROR_ALREADY_EXISTS` 即退出。**锁名不得更改**——rust / qt / wpf 三版靠同名互斥锁互斥（SPEC 20.1）。

### 4.5 DPI 与坐标（SPEC 20）
- Qt6 默认启用 Per-Monitor DPI Awareness V2；`QCursor::pos()`、`QScreen::availableGeometry()`、窗口 `move()/resize()` 一律是**逻辑像素（DIP）**，Qt 内部处理物理换算。
- 因此 SPEC 20 的 tao 物理/逻辑双轨陷阱在 Qt 下**天然不存在**：面板按主屏 `availableGeometry()` 右下角定位、菜单按 `QCursor::pos()` 所在屏定位即可。
- 仍需验证多屏异 DPI 下 `screenAt(QCursor::pos())` 的归属正确性（见风险 7.2）。

### 4.6 `--test-*` 控制台输出（Qt 版特有陷阱）
- GUI 子系统（`WIN32_EXECUTABLE`）的 exe 在终端里运行时**没有挂载控制台**，printf/qInfo 看不到。
- 方案：test 分支开头 `AttachConsole(ATTACH_PARENT_PROCESS)`，成功后用 `FILE* out = freopen("CONOUT$", "w", stdout)` 重定向再打印；失败（无父控制台）也不报错。
- `--test-ui` 需构造窗口验证资源：用 `QApplication`（不是 QCoreApplication）+ 各窗口构造后立即打印 OK 并退出，不进入事件循环常驻。

### 4.7 业务陷阱平移（SPEC 16.3 / 20）
- `amountLeft` 单位 1e-8 元：`(raw + 500000) / 1000000` 四舍五入到分；`priceInCents` 已是分。
- `isEnabled=false` → 整张 Extra 卡 "Not activated"。
- **JSON 数字按字符串建模**：QJsonValue 解析处需同时容忍 `isString()` 与 `isDouble()`（`toString()` 对 double 返回空串，必须先判型）。
- 刷新语义：失败保旧值 + 30s 快重试、2s 首刷、手动刷新 2s 防抖、托盘 hover 10s 节流、面板 300ms 重入守卫——全部平移。
- 凭证链顺序、30s 过期余量、`KIMI_CODE_HOME` 覆盖、MatchProvider 三条件——逐行平移。

### 4.8 LGPL 许可边界
- Qt 以 LGPL v3 为主。**动态链接 + windeployqt 随包分发 Qt DLL 是标准合规路线**（用户可替换 Qt 库）。
- 静态链接单 exe 会触发 LGPL 的"提供可重新链接目标文件"义务，本项目不采用。
- 发布 zip 中 Qt 版为目录形态（exe + DLLs + plugins），与 rust 版单 exe 不同；LICENSE/NOTICE 保持原样，另附 Qt 的 LGPL 声明（Qt 自带 `licenses/` 目录可一并打包）。

---

## 5. 分阶段实施计划

| Phase | 内容 | 验收 |
|---|---|---|
| 0 环境 | 安装 Qt 6 LTS（MSVC 2022 64-bit）：推荐 `pip install aqtinstall` 后 `aqt install-qt windows desktop 6.9.x win64_msvc2022_64`（免管理员）；CMake 用 VS 生成工具内置或 `winget install Kitware.CMake`；脚手架 `qt/CMakeLists.txt`（`find_package(Qt6 COMPONENTS Widgets Network)`） | `cmake -B build` 配置通过，空窗口能编译运行 |
| 1 无头核心 | credentials / quota / settings / update / state 五个模块 + `--test-fetch` / `--test-update`（AttachConsole） | 两个自检输出与 rust 版对照一致（同机先后跑两版对比 JSON） |
| 2 托盘+主面板 | QSystemTrayIcon、主面板窗（布局/卡片/进度条/Extra/版本行/4 按钮）、QSS 双主题、显隐动画、失焦收起、定位 | 与 `docs/screenshot-*.png` 基准目检一致；`--test-ui` 打印 `MainWindow OK` |
| 3 其余窗口 | 设置窗（含回填/保存/自启）、托盘菜单窗、Skills 窗（扫描+缓存+Refresh） | `--test-ui` 四窗全 OK；设置持久化与 rust 版 settings.json 互通（同 schema） |
| 4 Parity 检查 | 对照 SPEC 第二篇逐章打勾（10-21 章）；双主题截图对比；多 DPI 多屏定位验证；三版互斥验证（rust/qt 同时只跑起一个） | parity checklist 全过 |
| 5 打包发布 | windeployqt 产出目录；`make_release_zip.py` 支持 Qt 版（zip 内目录形态）；README 增补 Qt 版说明；SHA256SUMS 流程不变 | 干净机器（无 Qt）上解压即运行 |

并行约束：Phase 1–2 期间 rust/ 若发新版，qt/ 功能基线以**开始实现时的 rust 版本**为准，追赶差异放到 Phase 4 parity 环节，避免双移动靶。

---

## 6. 版本与发布策略

- `qt/` 独立版本号，从 **0.1.0** 起步（未达到 parity 前不与 rust 版同步）；达到 parity（Phase 4 完成）后并入统一版本清单，届时版本同步点变为五处：现有四处 + `qt/CMakeLists.txt` 的 `project(VERSION ...)`。
- 发布资产命名建议：`KimiPlanbarTray-qt-v<x.y.z>.zip`（目录形态）；rust 版维持现有单 exe zip。
- 回上游（shawn-0106t/kimi-planbar-tray）前同样先与用户确认 PR 还是独立仓库。

---

## 7. 风险清单

| # | 风险 | 缓解 |
|---|---|---|
| 7.1 | QSS 表达能力弱于 CSS（无 2 行 clamp、无 `::after` 伪元素），hover 光晕（SPEC 15.3 的描边+外发光过渡）需用 QGraphicsDropShadowEffect + QPropertyAnimation 近似，视觉可能有细微差异 | Phase 4 截图对比逐项确认；差异超出可接受度时该处改 QPainter 自绘（实际落地：2 行 clamp 用 `clamplabel.h` 自绘，光晕用 `hoverglow.h` 自绘——QGraphicsDropShadowEffect 方案已废弃） |
| 7.2 | 多屏异 DPI 下菜单/面板定位虽无 tao 陷阱，但 `QCursor::pos()` → `screenAt()` 的归属需实测 | Phase 4 专项验证（对照 `inspect_window_dpi.ps1` 思路） |
| 7.3 | QSystemTrayIcon **无 hover（MouseMove）事件**，SPEC 14 的 hover-to-refresh 10s 节流在 Qt 下没有直接信号 | 方案 A：nativeEventFilter 监听托盘图标句柄消息（实现复杂）；方案 B：放弃 hover 预取，tooltip 始终显示最近缓存值（Qt 版行为偏差，需在 SPEC 14 补注）。落地时先试 A，超预算则取 B 并同步 SPEC |
| 7.4 | 托盘图标 hide→show 后再 show 失效是已知 Qt on Windows 兼容性问题（Qt bugtracker 有记录） | 规避：托盘图标生命周期内只 `show()` 一次，退出时才销毁；不反复 hide/show |
| 7.5 | 静态链接诱惑（单 exe 好看）与 LGPL 合规冲突 | 坚持动态链接（4.8）；zip 目录形态已在发布策略中接受 |
| 7.6 | QJsonValue 数字/字符串双型处理遗漏导致金额显示归零 | Phase 1 用真实凭证 `--test-fetch` 与 rust 版逐字段对比 |
| 7.7 | Qt kit 体积大（~1-2 GB 磁盘）、aqtinstall 下载慢 | Phase 0 一次性投入；写清 .gitignore 排除 Qt 目录 |

---

## 8. 环境前置（Phase 0 明细）

本机现状（据 `~/.kimi-code/env-snapshot/环境依赖清单_2026-09-06.md`，30 天有效期内）：

- 已具备：Visual Studio 生成工具 2026（MSVC 编译器）、Windows SDK 10.0.26100、VC++ 2022 运行库、Python 3.13（可装 aqtinstall）
- 缺失：Qt6 kit、独立 CMake

```bash
# 推荐安装路径（免管理员）
pip install aqtinstall
PYTHONUTF8=1 python -m aqt install-qt windows desktop 6.9.3 win64_msvc2022_64 -O C:/Qt
# CMake：优先用 VS 生成工具内置；或 winget install Kitware.CMake
```

构建（落地时）：

```bash
cd qt
cmake -B build -G "Visual Studio 18 2026" -A x64 -DCMAKE_PREFIX_PATH=C:/Qt/6.9.3/msvc2022_64
cmake --build build --config Release
# 打包
windeployqt --release build/Release/kimi-planbar-tray.exe
```

---

## 9. 实施状态（随 Phase 进度更新）

### Phase 0 环境 — 已完成（2026-09-12）

- **Qt 6.9.3** 已通过 aqtinstall 安装至 `C:/Qt/6.9.3/msvc2022_64`（`--archives qtbase qtsvg` 精简安装，含 Widgets/Network/Gui/Core 与 windeployqt，未装 multimedia/webengine 等无关模块）。
- **CMake 4.2.3**：使用 VS 生成工具内置（`C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe`），未单独安装；**MSVC 14.50**（VS Build Tools 2026）编译。
- 脚手架已建：`qt/CMakeLists.txt`（`project(VERSION 0.1.0)`、`find_package(Qt6 COMPONENTS Widgets Network)`、`WIN32_EXECUTABLE`）、`qt/src/main.cpp`（无边框透明空窗 stub）、`qt/.gitignore`（排除 `build/`）。
- **验收通过**：`cmake -B build` 配置通过；Release 编译成功；空窗口 exe 实测可启动（与 rust 版 GUI 实例同机并存）。

### Phase 1–4：已完成（2026-09-13）

详见 `docs/archive/HANDOFF-qt.md` 第 1 节（每 Phase 的验收证据与踩坑记录都在那里）。要点：无头核心五项与 rust 逐字段一致；托盘+四窗全部落地并截图对比通过；SPEC 10–21 逐章 parity 完成（hover 光晕最终落地为 `windows/hoverglow.h` 自绘——QGraphicsDropShadowEffect 方案因 blur=0 隐藏控件、无边框透明窗内闪烁而废弃；托盘 hover-to-refresh 用 geometry 轮询实现——两条 7.x 风险的最终处理见 HANDOFF）。

### Phase 5 打包 — 已完成（2026-09-13）

- `qt/package_release.py`：重建 Release + windeployqt（`--release --no-translations --compiler-runtime --dir qt/dist`）+ 手动拷贝 MSVC CRT（VS Build Tools 无 VCINSTALLDIR，`--compiler-runtime` 找不到；VS 2026 的目录是 `Microsoft.VC145.CRT`）。
- 产出：`qt/dist/` 目录形态，28 文件 / 36.1 MB，动态链接（LGPL 合规路线）。
- 验收：PATH 无 Qt 目录时 dist exe 四项自检与 rust 一致，GUI 托盘/面板实测正常。

*Qt 版五个 Phase 全部完成。*

---

*文档版本：v1.1（2026-09-12 Phase 0 完成，新增状态章节）。Qt 版开始实现后，本文档的状态章节随 Phase 进度更新。*

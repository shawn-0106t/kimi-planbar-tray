# egui 技术栈迁移规划（EGUI-MIGRATION）

> 本文档规划 kimi-planbar-tray 的第四个实现版本：**纯 Rust egui** 版（`egui/` 目录）。
> 决策已定：UI 按现有设计 1:1 复刻（SPEC 第二篇第 10–21 章数值契约不变，像素级验收对照 `docs/*.png`）；`egui/` 为独立 crate（版本从 0.1.0 起步，不跟随 `rust/` 的 1.7.3）；`rust/` 保持活跃，`qt/` 实验完成归档，`wpf/` 冻结只读，三者一律不动。
> 业务后端模块（credentials / quota / polling / settings / update / skills 等）已存在于 `rust/src-tauri/src/`，后续 Phase 直接拷贝复用，不重写。
> 本文档是规划与方案；行为细节一律以 `docs/SPEC.md` 第二篇为契约。

---

## 1. 技术栈选型

| 维度 | egui（选定） | iced | Slint |
|---|---|---|---|
| License | MIT/Apache-2.0 | MIT/Apache-2.0 | GPL-3.0 / 商业双授权（Free 桌面可免，但条款需复核） |
| CJK 支持 | 自带字体回退链 + 系统字体加载（skrifa 解析 TTC，实测 msyh.ttc 直读成功） | 需自行加载字体，回退链较原始 | 内建 i18n 较好，但自定义字体管线封闭 |
| 自绘自由度 | 即时模式 + epaint 全自绘（圆角、阴影、进度条都手画），最贴合"透明窗口 + 自绘圆角卡片"需求 | 布局模型约束较多，自绘需 Custom API | DSL + 编译期编译器，运行时自绘受限 |
| 托盘 | 配 `tray-icon` crate（Tauri 同款，成熟） | 无内建，同样配 tray-icon | 无内建 |
| 一句话 | 即时模式最适合像素级复刻小面板；版本 Churn 靠锁死版本号管理 | — | license 是硬伤 |

选型结论：**egui + tray-icon**，渲染后端首选 glow（备选 wgpu，Phase 0 两者均验证通过）。

---

## 2. 目标架构

```
egui/
├── Cargo.toml                # kimi-planbar-tray-egui, eframe/glow+wgpu + tray-icon + windows
└── src/
    ├── main.rs               # 入口：--test-* 参数在单实例 mutex 之前处理（照搬 rust/ main.rs 顺序）
    ├── app.rs                # App 状态机 + 根 update 循环 + 4 viewport 注册
    ├── ui/
    │   ├── theme.rs          # SPEC 11 色表（Moonlit/Moondark 两套 Color32 常量）
    │   ├── panel.rs          # 主面板卡片绘制（SPEC 12）
    │   ├── settings.rs       # 设置窗（SPEC 17）
    │   ├── skills.rs         # 技能窗（SPEC 21）
    │   ├── menu.rs           # 托盘菜单（SPEC 13）
    │   └── widgets.rs        # 进度条、Badge、单选丸等复用件
    ├── tray.rs               # tray-icon 封装：图标/tooltip/事件转发
    ├── windows.rs            # HWND 后处理：DWM donotround + WS_EX_TOOLWINDOW + SetForegroundWindow
    ├── panel_pos.rs          # show_panel / show_menu 定位移植（SPI_GETWORKAREA、MonitorFromPoint、GetDpiForMonitor）
    ├── icons.rs              # 蓝 ball 兜底图标 + Kimi logo 加载
    └── credentials.rs / quota.rs / polling.rs / settings_store.rs / update.rs / skills.rs
                              # 从 rust/src-tauri/src/ 拷贝的纯 Rust 业务模块（去 tauri 化：回调/channel 代替 AppHandle）
```

关键机制（Phase 0 已验证可行）：

- **repaint 唤醒**：tray-icon 事件走 `TrayIconEvent::receiver()` channel；独立线程阻塞 recv，任何事件 → `egui::Context::request_repaint()`（Context 为 Send+Sync）。winit 的事件泵 `PeekMessageW(hwnd=0)` 会顺带分发 tray-icon 的消息窗回调，托盘必须建在主线程。
- **4 窗口 = 1 Context + 4 viewport**：root = 主面板（NativeOptions.viewport），settings/skills/menu 用 `show_viewport_deferred` 每帧注册；可见性一律用 `send_viewport_cmd_to(vp, ViewportCommand::Visible(..))` 管理，builder 固定 `visible(false)`，保证"先定位后显示"的命令顺序确定性（viewport 命令按入队序执行）。
- **菜单高度在 Rust 侧计算**：`send_viewport_cmd_to(menu, InnerSize(...))`，替代 Tauri 前端回报高度的模式。
- **tokio 运行时**：业务端口后放独立线程 + `std::sync::mpsc`/`tokio::sync::mpsc` 回 GUI；HTTP 拉取不阻塞 UI 线程。
- **GUI 子系统 stdout**：release 以 `windows_subsystem = "windows"` 编译（不弹控制台），`--test-*` 输出走 qt/ 版的 `WriteFile(GetStdHandle(STD_OUTPUT_HANDLE))` 模式（兼容 Git Bash 管道与 cmd）。
- **egui 定位语义**：`ViewportCommand::OuterPosition` 接收**逻辑点**，egui-winit 内部乘以"该窗口当前 ppp"转物理 —— 与 tao 相反。本机 225% 单屏下 `物理 = 逻辑 × 2.25` 精确成立；混合 DPI 多屏时存在与 panel.rs 注释同类的陷阱（窗口 ppp ≠ 目标显示器 ppp），届时需改走 HWND `SetWindowPos` 直设物理坐标（windows.rs 已具备 HWND 通道）。

---

## 3. Phase 计划（0–5）

| Phase | 内容 | 验收 |
|---|---|---|
| **0** | 可行性 spike（本文档第 6 节） | 透明度/托盘/焦点/DPI/CJK 全过 → GO |
| **1** | 无头核心移植：credentials/quota/polling/settings/update 从 `rust/` 拷入并去 tauri 化 | `--test-fetch`、`--test-update` 输出与 rust 版**逐字段一致**（同一台机器 diff） |
| **2** | 托盘 + 主面板（SPEC 10/11/12/14/15）：左键 toggle、右键菜单、hover 节流、失焦自动收起、300ms 再入防抖、面板出入场动画 | 对照 docs/*.png 截图；托盘事件时延 <100ms |
| **3** | settings + skills + menu 三窗（SPEC 17/21/13 + 10.3）：拖动、失焦关闭、菜单高度计算、边界钳制/上翻 | SPEC 数值逐项核对 |
| **4** | 一致性清单：`--test-ui` 等价自检、DPI 225%/100%、单实例互斥（同锁名 `KimiPlanbarTray.SingleInstance` 与其余三版互斥）、深色主题 | `--test-ui` 通过 + 截图对照 |
| **5** | 打包单 exe（glow 后端、release、图标/版本资源）、README/CHANGELOG/AGENTS/SPEC 文档同步 | release zip 集成或独立分发（沿用 make_release_zip 流程扩展） |

---

## 4. 文档维护矩阵（哪个 Phase 动哪些 md；文档滞后 = Phase 未完成）

| 文档 | P1 | P2 | P3 | P4 | P5 |
|---|---|---|---|---|---|
| `docs/SPEC.md` / `docs/SPEC_EN.md`（7.3 版本清单 + egui 版说明） | | x | x | x | x |
| `AGENTS.md`（目录结构 / 构建命令 / 测试 / 版本清单） | x | x | | x | x |
| `README.md` / `README_CN.md`（第四版本说明） | | | | | x |
| `CHANGELOG.md`（双语条目，Phase 1 起 Unreleased 首记） | x | x | x | x | x |
| `CONTRIBUTING.md`（egui 构建前置 + CI gate） | | | | x | x |
| `.github/workflows/ci.yml`（egui cargo check/test job） | 暂不纳入（qt 先例；待用户裁决后再定 Phase） | | | | |
| `docs/EGUI-MIGRATION.md`（实施状态节，每 Phase 收尾更新） | x | x | x | x | x |

---

## 5. 风险清单

1. **viewport 透明度**（历史 issue emilk/egui#3632）：glow 延迟 viewport 曾不支持透明。**0.36.2 已修复**（源码含 "Preserve explicitly requested transparent child viewports on Windows"），Phase 0 双后端实测通过；若升级 eframe 后回归，回退梯子 = glow → wgpu → 原生 winit + egui_glow 按窗直建（spike_raw 思路），三层均未失效过。
2. **viewport API churn**：eframe 0.36 相对 0.35 已有破坏性改名（`App::update(ctx)` → `App::ui(&mut Ui)`、`AppCreator` 返回 `Result`、`run_native` 去掉 `Option` 包裹）。**对策：Cargo.toml 锁死 `=0.36.x` 精确版本，升级视为一次移植任务**。
3. **skip taskbar**：`ViewportBuilder::with_taskbar(false)` → winit `skip_taskbar` 已生效；HWND 手工 `WS_EX_TOOLWINDOW` 作为兜底双保险保留（windows.rs）。
4. **CJK 字体**：msyh.ttc（index 0）实测可直读（epaint 0.36 用 skrifa 解析，支持 TTC collection）。候选链 msyh → simhei → simsun；全失败则打包 Noto Sans SC 子集（约 1–2 MB）。
5. **阴影柔度差异**：epaint `Shadow::as_shape` 为羽化矩形，与 WPF `DropShadowEffect`（Blur 24/Opacity 0.25）观感相近但不逐像素等同；像素级验收以"±若干像素带内可接受"对待，或后续换预烘焙阴影贴图。
6. **焦点获取**：纯程序化 `Visible(true)`（无用户手势）可能拿不到前台（Windows 前台锁），Phase 0 已实测；真实场景显示均由托盘点击触发（有手势 → 必得焦点，已实测）。settings 窗同理靠拖拽/点击手势。必要时 windows.rs 走 `SetForegroundWindow` 补救（菜单路径已验证 ok=true）。
7. **混合 DPI**：单屏 225% 全部精确；多屏混合 DPI 时 OuterPosition 的"窗口 ppp"换算陷阱见第 2 节，P4 前需决定是否补 HWND 直设路径。
8. **wgpu 内存**：debug 全量链接下 wgpu 后端 RSS ~449 MB vs glow ~172 MB（release 单后端会显著低于此，但 glow 更轻是确定的）——选 glow 为主后端的直接依据。

---

## 6. 实施状态

### Phase 0 — 可行性 spike（已完成，2026-10-14）

**环境**：Windows 11，主屏 225%（2880x1800 物理 / 1280x800 逻辑），Git Bash + PowerShell 5.1 驱动真实输入（UIAutomation 定位托盘图标 + SendInput 级 mouse_event 真实点击）。

**锁定版本**：eframe = egui = egui-winit = egui_glow = egui-wgpu = epaint = **0.36.2**，tray-icon **0.26.1**，winit 0.30.13，windows 0.62.2（`cargo build` 零 warning）。

#### 结果矩阵

| 检查项 | eframe-glow | eframe-wgpu | 备注 |
|---|---|---|---|
| V1 构建 + 4 viewport 创建/隐藏/退出 | PASS | PASS | `cargo build` ~80s 首次（tuna 镜像）；`run_native` 干净返回 `Ok(())` |
| V2 逐像素透明（窗口角 = 背后桌面像素） | PASS | PASS | 见下方采样值；两条后端数字一致 |
| V2 卡片中心 = SPEC #f3f4f6 | PASS | PASS | shown 采样 (243,244,246) 与色值逐字节相等 |
| V4 托盘左键 toggle（真实点击） | PASS | 未重跑（与渲染器无关） | click → 首帧重绘 **101.0 ms**（含首次定位） |
| V4 托盘右键菜单在光标处 | PASS | 未重跑 | `SetForegroundWindow ok=true` |
| V4 菜单边界钳制 + 底边上翻 | PASS | 未重跑 | 数学精确：光标 (2870,1690)→逻辑 (1103.0,602.1)；(200,200)→(69.9,77.9) |
| V4 hover Enter/Move/Leave | PASS | 未重跑 | 10s 节流日志 + Leave 事件 |
| V5 焦点事件 + SPEC 15 失焦自动收起 | PASS | PASS | `focused Some(true)↔Some(false)` 转换日志；`--autohide` 实测隐藏后截图区域为空 |
| V6 HWND donotround + WS_EX_TOOLWINDOW + 无任务栏按钮 | PASS | PASS | exstyle `0x40118→0x40198`；任务栏截图无 spike 按钮 |
| V7 DPI：native_ppp + 物理/逻辑尺寸 | PASS | PASS | 4 viewport 全部 `native_ppp=2.25`；物理外形尺寸精确 = 逻辑 × 2.25（主面板 954x1170 = 424x520×2.25） |
| V8 CJK（加油包余额 ¥12.34 — 五小时用量） | PASS | PASS | 截图确认无豆腐块；加载字体见下 |
| V9 RSS / 空闲 CPU（debug，双渲染器全链接） | 172.1→169.2 MB；0.3–0.5% 单核 | 449.3 MB；0.2% 单核 | 全窗口隐藏后 10s 采样（GetProcessTimes/GetProcessMemoryInfo） |
| 拖动（settings 标题栏 StartDrag） | PASS | PASS | 真实拖拽 +260,+60 生效，日志 `StartDrag requested` |

**透明度采样**（主面板，物理 px；基线 = 窗口隐藏时全屏截帧同点采样）：

| 采样点 | shown (glow / wgpu) | baseline | 判定 |
|---|---|---|---|
| 中心 (2376,1080) | 243,244,246 / 243,244,246 | 12,12,12 | 卡片精确 #f3f4f6 |
| 右上角 (2849,499) | 12,12,12 / 12,12,12 | 12,12,12 | 与桌面逐字节相等 → 全透明 |
| 角 +4px (1903,499) | 11,11,11 / 11,11,11 | 12,12,12 | 阴影羽化 ±1，基本全透明 |
| 角 +30/+60px | 12,12,12 | 12,12,12 | 与桌面相等 |

**CJK 字体**：第一候选 `C:\Windows\Fonts\msyh.ttc`（19.7 MB，TTC index 0，epaint 0.36 skrifa 解析）即成功覆盖 CJK + ¥，视觉确认无豆腐块。注意坑：`set_fonts` 在下一帧 `begin_pass` 才重建字体集，同帧 `has_glyph` 校验会误报失败（须跨帧校验）。

**结论：GO。** 推荐路径 = **eframe-glow**（内存最低、透明度/托盘/焦点/DPI/CJK 全过）；eframe-wgpu 完整可用作备选；`spike_raw.rs`（原生 winit 逐窗 DisplayBuilder 回退）**无需实现** —— #3632 在 0.36.2 已修复，回退梯子只留在纸面。

**Phase 0 附加发现（后续 Phase 必读）**：

1. eframe 0.36 API 相对 0.35 破坏性变更：`App::update(ctx, frame)` → `App::ui(&mut Ui, frame)`；`AppCreator` 须返回 `Result<Box<dyn App>>`；`run_native` 第三参不再套 `Option`。所有旧教程/示例按 0.35 写的都要换算。
2. root viewport 首帧后被 eframe 强制 `set_visible(true)`（post_rendering，防白闪机制）——需要"启动隐藏"时在首帧立刻补发 `ViewportCommand::Visible(false)`（首帧全透明，无可见闪烁）。
3. 可见性管理走 `send_viewport_cmd_to(…, Visible)`；builder 的 `visible` 差分也会推命令，两条路径并存时顺序不确定 —— spike 固定 builder `visible(false)` 只走命令通道，"先 OuterPosition 后 Visible"顺序可靠。
4. 托盘图标必须建在主线程（winit 消息泵 `PeekMessageW(0,0,0)` 会分发同线程所有窗口消息）；事件线程阻塞 `receiver().recv()` + `request_repaint()` 即可，无轮询成本。
5. 程序化 show（无用户手势）不获焦点（Windows 前台锁），`--autohide` 测试中因此立即触发失焦收起；托盘点击触发的 show 必获焦点（实测 `focused=Some(true)`）。真实产品路径全部经托盘手势，不受影响；settings 拖拽同理。
6. 混合 DPI：`OuterPosition` 用"窗口自身 ppp"换算（egui-winit `pixels_per_point(ctx, window)` = zoom × window.scale_factor()）；单屏精确，多屏混合 DPI 需评估窗口 ppp ≠ 目标屏 ppp 的场景（同 panel.rs 注释的陷阱族）。
7. 测试驱动基建：Git Bash 的 FIFO 对子进程 stdin 不可靠，`tail -f 文件 | exe` 可靠；Windows 侧真实输入用 PowerShell `SetProcessDpiAwareness(2)` + UIAutomation（Win11 托盘折叠区图标名 = tooltip 文本，chevron 名为"显示隐藏的图标"）+ `mouse_event`。脚本在 `egui/target/spike-shots/`（gitignore 内，作参考不发布）。

**产物**：`egui/`（Cargo.toml、.gitignore、src/bin/spike.rs、Cargo.lock）；截图与日志在 `egui/target/spike-shots/`。spike 代码按裁决保留作 Phase 1 参考。`rust/` `qt/` `wpf/` `scripts/` `.github/` 与既有文件零改动；未做任何 git 提交。

### Phase 1 — 无头核心移植（已完成，2026-10-14）

**落地内容**（`egui/src/`，全部从 `rust/src-tauri/src/` 逐行拷贝后去 tauri 化）：

- 业务模块 8 个：`credentials.rs` / `quota.rs` / `settings.rs` / `skills.rs`（含 frontmatter 单测两例）/ `update.rs` / `state.rs` 六个零 tauri 依赖模块逐字节原样拷贝；`polling.rs` / `theme_watch.rs` 改签名为接收 `AppCtx`。
- 去 tauri 映射：`AppHandle` → `AppCtx`（`ctx.rs`：`Arc<AppState>` + `tokio::sync::mpsc::UnboundedSender<AppEvent>`，Clone 句柄）；`app.emit("quota-updated"/"update-status"/"theme-changed")` → `AppEvent::QuotaUpdated/UpdateStatus/ThemeChanged` 事件总线（`lib.rs::build_state()` 返回 receiver 供 Phase 2 GUI 消费）；`tray::update_tooltip` 调用点由 `QuotaUpdated` 事件承载（Phase 2 托盘接手）；`tauri::async_runtime` → 调用方自建 tokio runtime（`--test-*` 走 rust 版 main.rs 同款 `block_on` 短生命周期 runtime；GUI 待 Phase 2 在后台线程建常驻 runtime）。
- `main.rs` 新入口：`--test-fetch` / `--test-update` / `--test-ui` 先于单实例互斥锁（与 rust 版同序）；`--test-ui` 为 Phase 2 占位，输出 `UI-FAIL: NotImplementedError: Phase 2`（沿用 rust 版 `UI-FAIL: {type}: {message}` 格式）；单实例 `CreateMutexW` 锁名 `KimiPlanbarTray.SingleInstance` 与其余三版互斥（照搬 rust 版实现）；GUI 路径暂打印 stderr 占位后退出；`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]`（debug 带控制台、release 为 GUI 子系统）。
- `--test-*` 输出移植 qt/ 版 `printStdout` 模式：`WriteFile(GetStdHandle(STD_OUTPUT_HANDLE))` 为主（Git Bash 管道 / cmd 控制台均覆盖），AttachConsole + `CONOUT$` 兜底，`println!` 末级回退。
- `Cargo.toml`：业务依赖按 rust/ 版同版本加入（tokio 1 [rt-multi-thread, macros, time, process, sync]、reqwest 0.13 [json]、serde derive、serde_json、regex、chrono 0.4 [serde]、winreg 0.55）；windows 0.62 在原 features 上加 `Win32_System_Console` / `Win32_Storage_FileSystem` / `Win32_Security`；eframe `=0.36.2` / tray-icon `=0.26.1` 原锁定不动，`src/bin/spike.rs` 未动。

**验证**：`cargo build` 与 `cargo test`（skills 单测两例通过）零 warning；`cargo clippy -D warnings` 对 lib 与 main bin 全干净——`--all-targets` 下仅 `src/bin/spike.rs` 存在 2 处 Phase 0 遗留 lint（该文件按裁决不动，egui 包尚无 CI job 约束它）；`rustfmt --check` 对 Phase 1 新增文件全干净（spike.rs 的格式漂移为 Phase 0 原状，未动）。本机 `--test-fetch` / `--test-update` 与 rust 版 exe 同机对照：`--test-update` 单行输出逐字节一致（`local=2.1.1 latest=2.1.1 updateAvailable=False checkFailed=False`）；`--test-fetch` JSON 逐字段一致（仅 `fetchedAt` 时间戳随运行时刻变动）。debug（控制台子系统）与 release（GUI 子系统）两个 exe 均验证——release 经 Git Bash 管道输出正常，即 `WriteFile(GetStdHandle(STD_OUTPUT_HANDLE))` 模式生效。`--test-fetch` 当前返回 `error: "no-token"` 为本机真实状态（OAuth access_token 已过期且 config.toml 的 `managed:kimi-code` provider api_key 为空），与 rust 版行为一致，属正确的凭证链回退。单实例互斥实测：本机有 rust 版实例（`KimiPlanbarTray-rust.exe`）持锁运行时，egui 无参启动静默退出（同锁名跨版本互斥按设计生效）；GUI 占位 stderr 文案在该锁释放后才可见（Phase 2 移除该占位）。

**文档**（本 Phase 按第 4 节矩阵触达）：`SPEC.md` / `SPEC_EN.md`（3.1 / 3.3 / 4 / 7.1 / 7.2 / 7.3 / 9）、`AGENTS.md`（版本清单 / 概览 / 目录结构 / 构建命令 / 测试）、`CHANGELOG.md`（双语 Unreleased 条目）、本文档本节。矩阵中的 `.github/workflows/ci.yml`（egui cargo check/test job）按 Phase 1 标记，但本期约束不动 `.github/`，留待后续 Phase / 用户裁决。

### Phase 2 — 托盘 + 主面板（已完成，2026-10-14）

**落地内容**（`egui/src/`，SPEC 10 / 11 / 12 / 14 / 15）：

- `app.rs`：eframe `App`（0.36 `App::ui(&mut Ui)` 新签名）——面板状态机 `Hidden / Showing / Shown / Hiding`（SPEC 15.1/15.2 动画：入场 fade 160ms 线性 + translateY 16→0 220ms CubicEase-out，出场 fade 130ms + →12 160ms CubicEase-in，170ms 时真隐藏，与 rust/ 前端的 170ms 完成定时器对齐）；失焦自动收起带 `panel_was_focused` 闩 + `settings_open` / `skills_open` / `test_ui` 抑制；`UiCmd` 跨线程命令通道（TogglePanel / UpdateTooltip / Quit，push 即 `request_repaint` 唤醒）；300ms 再入防抖（`finish_hide` 盖 `last_hide` 戳，show 自 Hiding 起始即替换状态机——陈旧 finish 永不触发，语义同 rust/ panel.rs）。
- `ui/panel.rs`：SPEC 12 全布局 painter 自绘（窗口阴影 + 368x464 卡片 + 双用量卡 flex 吸收余高 + Extra 卡 + 版本行 + 4 图标按钮），自底向上布局（底部三段锚定，对应 CSS flex 盒模型）；SPEC 15 出入场数学；`PanelAction` 回传执行（Console / Refresh 含 2s 手动刷新防抖 / Settings / Skills / Quit，后两者为 Phase 3 占位）。
- `ui/theme.rs`：SPEC 11 双主题色表（Moonlit / Moondark const）；字体链 Segoe UI regular/bold/semibold 三档命名 family + CJK 探测链 msyh → simhei → simsun（缺文件仅降级，命名 family 永远绑定，文本布局不可能 panic）；`FontProbe` 每帧一步跨帧校验覆盖。
- `ui/widgets.rs`：SPEC 15.3 hover glow——`GlowTracker` 140ms CSS ease（cubic-bezier(0.25,0.1,0.25,1.0)，带单测）渐显渐隐 + 自绘 1px accent 描边圈 + 向外 alpha 递减 halo（暗色主题更宽更强，同 SPEC 参数）；`KPT_GLOW_DEBUG` 可视化开关。
- `tray.rs`：托盘 1:1 移植（SPEC 14）——左键 MouseUp toggle、hover Enter/Move 刷新 10s 节流、tooltip 文案（`5h x% · week y%` + " (update failed)" 后缀）；右键菜单窗口为 Phase 3（现 no-op）；tray-icon 建在主线程，事件线程阻塞 `receiver().recv()`。
- `panel_pos.rs`：`show_panel` 定位（SPI_GETWORKAREA 物理坐标 ÷ 主屏 scale → `OuterPosition` 逻辑点，+22px 阴影留白补偿）；每次 show 落盘 `kpt-windows.json`（exe 旁，诊断专用，随 target/ gitignore）。
- `windows.rs`：HWND watcher 线程（200ms 轮询可见性标志）——`DWMWA_WINDOW_CORNER_PREFERENCE = DONOTROUND` + `WS_EX_TOOLWINDOW` 兜底（每次 show 重施，幂等）；`spawn_focus_steal`（60/200/450ms 重试 `SetForegroundWindow`）。
- `icons.rs` + `assets/`：Kimi logo（rust/ 同源 PNG）+ 4 按钮 SVG 预光栅 PNG（96x96 白色填充，绘制时按主题文本色 tint，对应 SVG `fill="currentColor"` 语义）；`format_util.rs`：fmtYuan / formatReset 等格式化移植。
- `main.rs`：GUI 组装（SPEC 20 顺序：settings → services → 单实例 → tray（主线程）→ `run_native`）；tokio runtime 常驻后台线程（2s 首刷 / 主题监听 / 版本检查）+ 事件 drain 线程 + `SetConsoleCtrlHandler` 走 Quit 路径保托盘图标移除；`--test-ui` 实装（首帧 `show_panel` + `MainWindow OK`，6s 定时退出 + 5s 硬退出兜底），settings/skills/menu 三行按计划 Phase 3。

**验证**：

- 静态链：`cargo fmt --check` 干净；`cargo clippy --all-targets -- -D warnings` 全干净（含 spike.rs 收尾修复 2 处遗留 lint + fmt）；release 构建通过；`cargo test` 4 通过（2 frontmatter + 2 css_ease）。
- 自检：`--test-update` 与 rust 版逐字节一致；`--test-fetch` 同构（本机 no-token 真实状态，与 rust 版一致）；`--test-ui` 输出 `MainWindow OK` 后 ~6s 退出码 0。
- GUI 门禁（PowerShell UIAutomation 定位 + 真实鼠标点击，脚本与截图在 `egui/target/phase2-shots/`，gitignore 内）：亮色主题像素采样 卡片间 #f3f4f6 / 卡内 #ffffff / 进度条槽 #e5e7eb / 四角 = 桌面像素（逐像素透明成立）；卡片圆角切线法实测 ~14 逻辑 px（225% 下 31.5 物理圆角几何一致）；失焦收起：桌面点击后 ~200ms 内 `IsWindowVisible` 从 V 转 .（160ms 滑出 + 隐藏）；300ms 再入防抖：面板可见时点托盘图标 = 失焦收起 + toggle 被吞（4 个循环一致），`finish_hide` 后 ~175ms 的快速再点被吞（面板保持隐藏 1.2s），>300ms 再点正常显示（滑入 220ms）；5 连击 toggle 循环无闪烁无卡死；暗色主题采样 bg #17191e / 卡 #23262d / 槽 #3a3e47；空闲（面板隐藏）10s 采样 CPU 0.2% 单核 / RSS ~183 MB。
- 对照 `docs/screenshot-light.png`（rust 版参考，逻辑 px）：版本行 y=382-419 vs 383-420、按钮行 y=436-471 vs 436-473（±2px 内对齐）；用量卡 +36px / Extra 卡顶边 +38px 的差异完全由本机无 token 状态解释——rust 版 `renderQuota(null)` 同样隐藏 Extra 卡 monthly 子面板 → Extra 卡变矮 → 用量卡 flex 吸收余高，"--" / "Update failed" / 真实 CLI 2.1.1 均与 rust 版同状态渲染逻辑一致。accent #1a88ff 因无数据进度条为空未截到，以 `ui/theme.rs` 色表常量对照 SPEC 11.1 核验。

**踩坑（Phase 2 新增，前 3 条为 Phase 0 发现本期落地为必要逻辑）**：

1. root viewport 首帧后被 eframe 强制 `set_visible(true)`（防白闪）→ 首帧立即补发 `ViewportCommand::Visible(false)`（首帧全透明，无可见闪烁）。
2. `ViewportCommand::OuterPosition` 接收**逻辑点**：物理工区目标坐标须除以窗口 ppp 再下发（panel_pos.rs），与 tao 的 `PhysicalPosition` 相反。
3. `set_fonts` 次帧（begin_pass 重建字体集）才生效：同帧 `has_glyph` 校验误报失败，覆盖校验必须跨帧（`FontProbe`）；未绑定的命名 weight family 会 panic，安装时永远绑定（缺文件回退 regular）。
4. 托盘点击后单独的 `ViewportCommand::Focus` 在 Win11 不可靠（实测面板 `focused=false`）→ `spawn_focus_steal` 在 shell 前台授权窗口内重试 `SetForegroundWindow`（托盘点击手势给了授权窗口）。
5. egui 对"已显示但从未获焦"的窗口报告 `focused == Some(false)`：失焦自动收起必须加 `panel_was_focused` 闩（真实获焦过才启用），否则程序化 show（`--test-ui`）立即被收起——tauri 侧对应的是 Focused(true)→Focused(false) 事件对，语义不同。
6. 测试基建：Win11 托盘折叠区 flyout 在面板获焦后仍存活，可对固定图标坐标裸 `mouse_event` 连击驱动 toggle / 防抖测试；UIAutomation 单次查询约 200-500ms，无法用于 300ms 防抖窗口内的定时点击（须预取坐标）；chevron 点击是 toggle 语义，测试脚本不得盲点（会翻转 flyout 开合状态）。

### Phase 3 — settings / skills / menu 三窗（已完成，2026-10-14）

**落地内容**（`egui/src/`，SPEC 10.2 / 10.3 / 13 / 14 / 18 / 21）：

- `ui/settings_win.rs`：设置窗 404x464（卡片 348x408 = 阴影留白 28），SPEC 13 全布局 painter 自绘——标题栏（logo 18 + 15px SemiBold + ✕ 28px `ChromeCloseButton`，整行 `Sense::drag` → `StartDrag`，拖动区排除 ✕）、ThemeRadio x3（18px 圆 1.5px 次级描边 + 选中 8px accent 圆点 + 悬停文字变 accent）、PillRadio x4（padding 12,6 / radius 9 / 选中 accent 白字 / 悬停 ButtonHover，选中+悬停保持 accent）、ThemeCheckBox（18px 圆角 5 方块 1.5px 边框，选中 accent + 白色对勾）、全宽 ActionButton Save（含 15.3 glow）。每次打开按持久化设置回填 draft（越界值回落默认，同 rust/ 前端语义）；✕ 只隐藏不保存（s4/s5 截图对照验证：未保存的 Moondark 选择重开后回到 Moonlit）；Save 走 rust save_settings 同链路：写 settings.json（PascalCase + 2 空格缩进）→ `apply_auto_start`（HKCU Run 增删，实测往返一致）→ `theme_watch::apply_now`（system 跟随重估）→ `reschedule.notify_one` → 关窗。
- `ui/skills_win.rs`：Skills 窗 404x520，同款标题栏 + 汇总行（"N skills" / "No skills found" / 扫描中 "Loading…"）+ `btn-mini` Refresh（手绘 ⟳ 弧线箭头 glyph，不依赖 U+27F3 字体覆盖）+ ScrollArea 分组列表（非浮动 8px 滚动条，thumb = progress_track、track 透明、圆角 4，经 scoped `WidgetVisuals` 上色）；卡片 radius 10 padding 10x12，名称 13 SemiBold 单行省略号（二分前缀 + U+2026），描述 12px 两行 clamp（`clamp_rows`：按字符二分使 galley 行数 ≤ 2），完整描述挂卡片 hover tooltip（等价 qt clamplabel / CSS line-clamp）；首次打开懒扫描一次入缓存（tokio `spawn_blocking`），Refresh 强制重扫（旧缓存渲染到新列表落地）。
- `ui/menu_win.rs` + `panel_pos::show_menu_at`：托盘右键菜单 188 逻辑宽、高度 = 内容 + 2x24 阴影留白（`menu_window_height` 直接量 egui 字型——rust/ 前端回报高度的黑科技变本机直测）；5 个 `MenuItemButton`（padding 14,9 / radius 8 / 悬停 ButtonHover + glow）；定位 = rust panel.rs `show_menu` 逐行移植（光标物理 px → MonitorFromPoint/GetDpiForMonitor → 水平钳制 + 底边上翻 + 旧布局 -19px 补偿），数学抽成纯函数 `menu_position` 并单测 5 例（其中两例复算 Phase 0 spike 记录的 (2870,1690)→(1103.0,602.1) 与 (200,200)→(69.9,77.9)）；失焦关闭带 `menu_was_focused` 闩（同 Phase 2 gotcha 5），`Visible(false)` 走命令队列 = SPEC 10.3 延迟安全语义。
- `tray.rs` 右键接通：`UiCmd::ShowMenu { px, py }` 携带 tray-icon 物理光标坐标，root pass 执行定位 + show + `spawn_focus_steal`；菜单项点击 = 关菜单（回调内 `Visible(false)`）+ `WinAction` 入队 + `request_repaint_of(ROOT)` 唤醒 root pass 执行（Open=toggle_panel(false) 无 300ms 防抖、Refresh=2s 防抖手动刷新、Settings/Skills=开窗、Exit=退出），与 rust menu_action 语义一一对应。
- `app.rs`：`WinBus`（draft / 菜单开合闩 / 动作队列 / 三窗各自 GlowTracker / icons 句柄）+ `register_viewports`（三个 deferred viewport 每帧注册——不注册即销毁原生窗，singleton 复用必须常驻；可见性一律命令通道）；设置/技能窗打开时回填 draft + 居中（work area 中心）+ 尺寸钉死；`settings_open` / `skills_open` 抑制面板失焦收起（实测开关两侧行为正确）。
- `--test-ui`：依次 show 面板 + 三窗（不抢前台，同 rust run_ui_test 的 show-only 语义；菜单落当前光标位置），输出四行 OK 后 ~6s 退出。

**验证**：

- 静态链：`cargo fmt --check` 干净；`cargo clippy --all-targets -- -D warnings` 零告警；`cargo build --release` 通过；`cargo test` 9 通过（2 frontmatter + 2 css_ease + 5 菜单定位数学）。
- 自检：`--test-fetch` 与 rust 版逐字段一致（仅 `fetchedAt` 为运行时刻）；`--test-update` 单行逐字节一致（`local=2.1.1 latest=2.1.1 updateAvailable=False checkFailed=False`）；`--test-ui` 四行 OK、退出码 0。
- GUI 门禁（release/debug exe + PowerShell 真实点击，脚本与截图在 `egui/target/phase3-shots/`，gitignore 内）：菜单落光标数学与 `GetWindowRect` 实测零偏差——托盘右键（光标 1812,1518 → DIP 805.3,674.7，底边上翻分支）期望物理 (1769,1019) == 实测 (1769,1019)；右缘钳制以 `--test-ui` 菜单落点验证（光标 2850,700 → 期望 (2482,675) == 实测 (2482,675)，min(cx, wa_right-150-8)-19 精确命中）；`Open` 菜单项两次验证 toggle 生效；设置窗居中实测 (986,324)+909x1044 == 工区中心 DIP (1440.5,376)；主题 Save 实时切换像素级核验（同面板实例 dark #17191E/#23262D → light #F3F4F6/#FFFFFF 精确，settings/skills/menu 窗同步跟随，其中 skills 窗为开启状态下被 Save 直接换色）；设置互操作全链路实测：`1 min`→`5 min` 丸选择、autostart 复选往返（HKCU Run 值增删 + 引号完整路径格式）、✕ 不保存（重开回填持久值）；portable.dat 重定向实测（Save 落 exe 目录、%APPDATA% 不动、清理后无残留）；skills 列表 112 项与文件系统 SKILL.md 计数一致，CJK（将飞书电子表格的全部可见 sheet 复刻为本地 Excel）渲染无豆腐块，滚轮滚动前后截图逐点比对 1425 采样点变化，Refresh 重扫计数不变。
- 空闲（面板隐藏）10s x2 采样：CPU 0.0% / 0.5% 单核，RSS ~213 MB（glow + 4 常驻 viewport；Phase 2 单窗 ~183 MB，+3 隐藏窗的 glow surface 增量 ~30 MB）。

**踩坑（Phase 3 新增）**：

1. **eframe 每视口独立 pass**：`run_ui_and_paint` 只跑"收到事件的那个窗口"的 pass——子视口回调执行时 `App::ui` 不运行（`EpiIntegration::update` 里二者是 if/else）。子窗交互必须写入共享队列（WinBus.actions）并 `request_repaint_of(ViewportId::ROOT)` 唤醒 root pass 执行；`Context::request_repaint()` 只重绘"当前 pass 的视口"，跨窗唤醒一律用 `request_repaint_of`。
2. **`ViewportInfo.visible` 无人写入**：egui-winit 0.36 的 `Visible` 命令只调 `window.set_visible`，`update_viewport_info` 也不写 `info.visible`——它恒为 `None` → eframe `show_ui = is_visible().unwrap_or(true)` 恒真 → root 隐藏时 `App::ui` 仍随 RedrawRequested 执行（Phase 2 的托盘 toggle 正是依赖这一点；升级 egui 时若此行为修复，隐藏态命令处理需要换成 `App::logic` 或显式 `set_visible`）。反之这也是"3 个隐藏常驻 viewport 零空闲成本"的前提：隐藏窗不 paint 但 pass 不跑。
3. **菜单右键重开 = 原窗重定位**：rust 版"先关旧实例再 new"在 egui 单例窗上等价于重新 InnerSize/OuterPosition/Visible（命令按入队序执行，先定位后显示顺序可靠）。
4. **✕/✓/⟳ 画原语不用字形**：Segoe UI / msyh 不保证覆盖 U+2715/U+2713/U+27F3，直接两段线/折线/弧线+三角绘制，跨机像素一致（与 CSS 字形视觉等价，记为微小偏差）。
5. **子窗 pass 里不能碰 App**：deferred viewport 回调是 `Fn + Send + Sync + 'static`，捕获 `Arc<AppCtx>` + `Arc<WinBus>`；App 持有的 tray（Rc）不可跨线程——图标纹理经 `WinBus.icons: OnceLock<Icons>` 下发（`Icons` 派生 Clone，TextureHandle 本就是引用计数）。

### Phase 4 — SPEC parity 全面审计（已完成，2026-10-17）

**范围**：SPEC 10–21 逐章对抗式 checklist（每条款 PASS / DEVIATION / UNTESTABLE + 证据）、跨版本互斥锁实测、混合 DPI/多屏审计、性能正式测量、双主题截图结构化对比、真实 parity bug 修复。审计正文与全部原始数据（脚本、日志、截图、采样）在 `egui/target/phase4-shots/`（gitignored）：`checklist.md` 为结论正本，`mutex_matrix.log` / `test_while_locked.log` / `qt_holder_test.log` / `reopen-settings-{before,after}.log` / `menu-real-{egui,rust}.log` / `menu-2nd-egui.log` / `cold-{egui,rust}.log` / `geom_compare4.log` / `shoot_themes.log` 为实测原始记录。

**修复的 parity bug（2 处，均已过静态链 + 实测回归）**：

1. **混合 DPI 多屏定位**（暂停前已入库）：`panel_pos.rs` 新增 `window_dpi_scale`——`ViewportCommand::OuterPosition` 是逻辑点且乘**窗口自身** scale（窗口所在屏），混合 DPI 下须按 `GetDpiForWindow` 换算（SPEC 20 启动器屏陷阱族）；`show_panel` / `show_centered` / `show_menu_at` 三个入口全部改走该换算。实测：设置窗拖至 125% 副屏（dpi 120）后重开精确落回主屏中心 (986,324)（修复前公式会落 (547,180)）；菜单落副屏光标点 (3840,930) 实测 (1379,653) vs 理论 (1378.75,652.9)。
2. **设置/Skills 窗每次重开被强制居中**（本次审计发现）：`show_centered` 原实现每次打开都居中，偏离 rust/（conf `center: true` 一次性）与 WPF（WindowStartupLocation 构造期一次性）的单例语义——用户拖动后的位置重开即丢。修复：`show_centered(..., recenter)` 首开居中、后续仅 pin 尺寸（DPI 守卫保留）；`App` 增 `settings_placed` / `skills_placed` 闩。回归实测：首开居中 (986,324) ✓，移至副屏 (6300,1080) → WM_CLOSE 隐藏 → 托盘菜单重开**位置保留 (6300,1080)** ✓。

**checklist 结论**（正本 `phase4-shots/checklist.md`，含每条证据引用）：PASS 99 / DEVIATION 2（均 minor，详见下）/ UNTESTABLE 6 / 修复 2（其中 1 项即上列第 2 条，另一项为暂停前的 DPI 修复，均转为 PASS）。

- **DEV-minor ×2**：① SPEC 19 的 `UI-FAIL: {type}: {message}` 错误路径行——egui --test-ui 若窗构造 panic 不打印该行（无 catch_unwind；正常路径四行 OK 与 rust 逐字节一致，触发前提是 eframe 自身 bug，裁决不修）；② SPEC 12.6 版本行首查完成前的亚秒瞬态——rust/ 前端显示 HTML 初值 `--`，egui 直显 `Not detected`（稳态两侧一致，裁决不修）。
- **UNTESTABLE ×6**：100%/其他单档 DPI 全量（Qt 先例；225%+125% 双档已实测）；启动器居副屏的首开定位（与已实测的"副屏窗口重开主屏"同一换算路径）；托盘图标驻留副屏任务栏（单任务栏主机）；Update available 徽标亮灯态（本机 latest 2.1.1 < local 2.2.0 永不亮；代码布局+色表核验）；`--test-skills`/`--screenshot`（Qt/WPF 专属）；真实 token 下的数据渲染实拍（无 token 真实状态；fmt_yuan 单测 + FormatReset 逐分支 + 三陷阱值代码核对替代）。

**互斥锁矩阵**（SPEC 20，全部用 repo 构建产物）：egui/rust/qt 三版两两（含自配对）共 9 组合，持锁方存活时后启方 **9/9 静默退出**；持锁期间 `--test-fetch` / `--test-update` / `--test-ui` 对三版挑战者 **27/27 可用**（`test_while_locked.log` + `qt_holder_test.log` + gate 补测 `gate_ui_locked.log`，自检先于 mutex 的设计成立）；`--test-update` 输出三版逐字节一致。

**DPI/多屏审计**（225% 主屏 2880x1800 物理 + 125% 副屏 1920x1080 物理，混合 DPI 拓扑）：真实托盘右键菜单落点 A/B——光标 (1812,1518)→菜单 (1769,1019) 423x524 与公式逐像素一致，rust 版同机同式命中 (2021,1247)@光标(2064,1746)，菜单物理高度两版逐字节同值（524）；--test-ui 菜单落副屏光标 ±0.3px；主面板 (1949,545) vs 理论 (1948.5,544.5)；设置窗跨屏重开见上。100% 单档与副屏任务栏如实标 UNTESTABLE（Qt 先例）。

**性能**（同机对照，方法：spawn→tray-icon 消息窗（类 `tray_icon_app`，两版同 crate 同信号）15ms 轮询，3 次取中位；空闲为面板隐藏态 10s 采样）：

| 指标 | egui | rust（同机） |
|---|---|---|
| 冷启动 spawn→托盘就绪（中位，3 次） | **91 ms**（142/91/63） | **1060 ms**（1131/959/1060，含 WebView2 环境创建） |
| 空闲 CPU（10s，单核折算） | 0.2%（0.016s/10s） | 0.0% |
| 空闲 RSS（进程树口径） | 213 MB（单进程） | ~665 MB（主进程 45-52 MB + 8 个 msedgewebview2 子进程） |
| exe 体积 | 21.8 MB（单文件） | 6.97 MB + WebView2 Runtime 系统共享 |

**双主题截图结构化对比**（参照物 = `docs/screenshot-{light,dark}.png` rust mock；本机无 token 为真实状态，数据驱动子状态按约定以代码/单测佐证）：底部锚定行逐块对比（geom_compare4.py）——版本行 REF y=382-419 vs EGUI 383-420（+1px）、按钮行顶边 436 vs 436（0px，底边 +3px 为文字降部渲染差，行盒锚定精确）；卡高差异（用量卡 +36/Extra 卡顶 +38）与 Phase 2 结论同源（rust renderQuota(null) 同样折叠 Extra 月度子面板）。SPEC 11 色值采样（sample_px.py，两主题 × 四窗）：winbg #f3f4f6/#17191e、卡 #ffffff/#23262d、菜单卡=winbg、设置 Save=ButtonBg #e9ecf0/#2c3039、动作按钮 rest=CardBg ✓——全部逐字节命中；透明边距采样=桌面像素（逐像素透明持续成立）。

**静态链与回归**（修复后终态）：`cargo fmt --check` ✓；`cargo clippy --all-targets -- -D warnings` 零告警；`cargo build --release` ✓；`cargo test` 9/9（2 frontmatter + 2 css_ease + 5 菜单定位数学）；`--test-ui` 四行 OK 退出码 0；`--test-fetch` 与 rust 版逐字段一致（仅 fetchedAt 运行时刻差异）、`--test-update` 逐字节一致（`local=2.2.0 latest=2.1.1 updateAvailable=False checkFailed=False`，2026-10-17 实测）。

**结论**：Phase 4 通过——SPEC 10–21 无未决 DEVIATION（2 项 minor 已定级留档），egui 版与 rust/ 契约在本机可测范围内 parity 成立；余项见 UNTESTABLE 清单（均为环境不可达，非代码缺口）。Phase 5（打包与文档收尾）待启动。

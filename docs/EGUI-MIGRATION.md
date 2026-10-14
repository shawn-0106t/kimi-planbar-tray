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
| `CHANGELOG.md`（双语条目） | | | | | x |
| `CONTRIBUTING.md`（egui 构建前置 + CI gate） | | | | x | x |
| `.github/workflows/ci.yml`（egui cargo check/test job） | x | | | x | |
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

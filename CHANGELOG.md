# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Entries for v1.7.2 and earlier were reconstructed from the GitHub release notes of each tag.

## [Unreleased]

### Added
- WPF edition revival, aligned with rust/ 1.7.3: the read-only Skills window end-to-end (scanner + window + tray-menu item + lazy scan with cache + panel focus-loss suppression while open, plus `--test-skills` and the fourth `--test-ui` OK line), the icon action row with the Console button (panel height 512 → 520, four icon buttons on the card background), the self-drawn hover glow (SPEC 15.3: accent ring + halo, 140 ms, theme-dependent parameters), and the 1.7.1 hardening backport (`KIMI_CODE_HOME` override, 2 s manual-refresh debounce, RefreshMinutes clamp 1–30 at save, saturating arithmetic, non-finite percent zeroing, stale-reschedule guard for the 2 s first refresh, hide/show animation generation counter, settings backfill tolerating malformed persisted values). The WPF edition ships via Releases again.
- Tag-triggered release automation: pushing a `v*` tag now builds the release exe on CI, runs the packager, and opens a draft GitHub Release with all three assets attached — publishing stays a manual decision.
- Experimental egui edition (Phase 1 + Phase 2 + Phase 3): the headless core ported from rust/ (credentials, quota, polling, settings, skills, update, theme_watch, state) with the Tauri dependencies removed, plus `--test-fetch`/`--test-update` output parity with the Rust edition. Phase 2 adds the tray icon and the main panel at SPEC 10-15 parity: left-click toggle, 10 s hover-refresh throttle, focus-loss auto-hide with the 300 ms re-entry guard, enter/exit slide+fade animations, the 140 ms hover glow, and Moonlit/Moondark themes. Phase 3 completes the four-window parity: the Settings window (theme radios, refresh-interval pills, autostart checkbox, Save chain with live theme apply + polling reschedule, draggable titlebar), the read-only Skills window (lazy one-shot scan with cache, grouped list, 2-line description clamp with hover tooltips, themed scrollbar, `112 skills` verified against the filesystem on the dev machine), and the tray right-click menu at the cursor (edge clamping, bottom-edge flip-up, Open/Refresh/Settings/Skills/Exit, focus-loss close) — `--test-ui` now constructs all four windows and prints `MainWindow OK` / `SettingsWindow OK` / `SkillsWindow OK` / `TrayMenuWindow OK`.
- egui edition completion (Phase 4 + Phase 5): a full SPEC 10-21 parity audit (99 PASS / 2 minor DEVIATION / 6 UNTESTABLE, cross-edition mutex matrix 9/9, mixed-DPI audit, same-machine perf comparison — 91 ms cold start and a ~21.8 MB single exe vs 1060 ms + WebView2 for the Rust build) with two real parity fixes (mixed-DPI window positioning via `GetDpiForWindow`, first-open-only centering for the settings/skills windows), two code-review fix rounds (root close guard for Alt+F4/WM_CLOSE, PID-filtered HWND lookup, quit-shortcut/zoom hardening), CI coverage (the same fmt/clippy/build/test gate now runs on `egui/` in ci.yml), and the final positioning decision: **experimental test-only, like qt/** — buildable from source, not distributed via Releases.
- Known-limitation documentation: the egui edition shows a faint 1-2px DWM hairline tracing the window rect on bright desktops (winit 0.30.13's undecorated-shadow NCCALCSIZE hack + the WS_CAPTION border drawn over content; tao's inset scheme is why rust/ is clean) — root-caused 2026-10-17, fix attempts verified then rolled back, accepted as-is for the experimental edition; full diagnosis and future options in `docs/archive/EGUI-MIGRATION.md` section 7. (Superseded: fixed later in this cycle — see Fixed below.)

### Fixed
- egui edition window frames on Windows 11 26H2 (build 26300+): DWM now draws each window's retained WS_CAPTION style as a full native white title bar + close button over the transparent content, and winit 0.30.13's undecorated-shadow WM_NCCALCSIZE hack (`top += 1`) leaves a 1px gray strip at every window's top edge (the previously documented "hairline" limitation, now gone). Fixed in `egui/src/windows.rs::style_hwnd`: `DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED` stops DWM from drawing the NC frame, and a wndproc subclass fully collapses the non-client area (the egui UI paints its own shadow per SPEC 10, so the DWM shadow the hack preserved was redundant). The HWND-styling watcher styles each window as soon as its HWND exists instead of on first show, so even the very first show is frameless, and re-styles on every hidden→visible transition because winit's `apply_diff` rewrites GWL_STYLE/GWL_EXSTYLE on every visibility flip (wiping the added WS_EX_TOOLWINDOW).
- egui edition one-frame white caption flash on activation changes (same 26H2 root family): with `DWMNCRP_DISABLED` stopping DWM from painting the non-client area, `DefWindowProc` GDI-paints the caption directly when winit forwards `WM_NCACTIVATE` (which winit must receive — it is the only message that sets `is_active`, so it cannot be swallowed). Fixed in `egui/src/windows.rs::nc_collapse_proc`: WS_CAPTION is stripped for the duration of the forwarded call and restored immediately after, so DefWindowProc finds no caption to paint. Verified by burst-capture regression across all four activation scenarios (open settings, tray right-click menu, menu items opening windows, close-settings focus fallback) — no flash frames.
- egui edition lost UI wakes (the "window state desync" hunt of the same arc): eframe 0.36.2 drops a `UserEvent::RequestRepaint` as "outdated" when two passes of the target viewport complete between the repaint callback and the event being processed, and a wake issued right as a pass burst ends can be the last wake source — dropped, it leaves the queued command undrained until some unrelated event arrives (observed as lost tray TogglePanel clicks and a settings window whose `SettingsClose` action never arrived, dead X button). Fixed in `egui/src/app.rs::wake_with_retry`: every cross-thread queue push additionally re-issues the repaint 120 ms later; egui's repaint-delay gate suppresses the retry when passes are flowing, so it costs nothing in the common case.
- egui edition click-dead re-shown windows: egui-winit learns the pointer position only from `CursorMoved` and drops all mouse-button input while none is known, but winit keeps `mouse.last_position` across a hide/show cycle — a window re-shown under an unmoved cursor emits no `CursorMoved` (position unchanged) and ignores clicks until the cursor moves to a NEW position. Fixed in `egui/src/windows.rs::spawn_pointer_rearm`: two synthetic WM_MOUSEMOVEs carrying the current cursor position (the second nudged 1px to break any tie with the stale pixel) are posted on every show.
- WPF edition main panel drifting from the rust baseline in the icon action row and the rounded card corners: the four action-row icons rendered oversized with per-icon scale drift because the `Viewbox`-wrapped `Path`s carried no explicit size — a bare `Path` measures its own geometry bounds, so the Viewbox scale factor was 16/bounds instead of the SVG viewBox's 16/24 (fixed with explicit `Width`/`Height="24"` on all four Paths); the action buttons, the version row, and the Skills Refresh button painted their at-rest background on the template `Grid` — a square rectangle that showed square corners against the `WindowBg`, since the rounded hover/glow layers only fade in on hover — the at-rest background now lives on a base rounded `Border` (CornerRadius 10 / 12 / 8); `IconActionButton` padding corrected to the SPEC 12.7 value `0,5,0,6` (was `5,0,5,6`, a misreading of the CSS `padding: 5px 0 6px` shorthand). SPEC 15.3's WPF template-layer description synced in both languages.

### Changed
- The WPF edition is maintained again (no longer frozen at v1.5.0): its `<Version>` joins the shared version sync list, `ci.yml` gains a compile-level `dotnet build wpf -c Release` gate, and `release.yml` publishes the two WPF exes — release assets grow from three to five (`KimiPlanbarTray-wpf.exe` + `KimiPlanbarTray-wpf-selfcontained.exe`, the historical v1.5.0 names; `.gitignore`'s `KimiPlanbarTray-*.exe` glob already covers both). SPEC 10.1/10.2 window sizes corrected to the current 424×520 / 404×464.
- The version bump checklist grows from 9 to 11 files: the egui/ edition's version (`egui/Cargo.toml` + the `kimi-planbar-tray-egui` entry in `egui/Cargo.lock`) now follows the rust/qt sync list (1.7.3) instead of its independent 0.1.0. The migration plan and handoff docs moved to `docs/archive/` (`EGUI-MIGRATION.md`, `HANDOFF-egui.md`).
- egui edition frozen as of 2026-10-17 (user decision — no further maintenance planned; the edition stays buildable and CI-covered). One residual known issue is accepted as-is: a full native caption can linger for a few seconds on a window's very first show within an instance (observed once across three instance first-shows during the flash regression, self-clears; not reproducible on demand). Details: `docs/archive/HANDOFF-egui-26H2-ncframe.md`.

## [1.7.3] - 2026-10-10

Maintenance release — no feature or UI changes to the Rust edition.

### Added
- Experimental Qt edition (C++ Qt6 Widgets, no WebView dependency) in-tree at SPEC parity with rust/; build from source only, not distributed via Releases.

### Changed
- Runtime dependency refresh: reqwest 0.12 → 0.13, windows 0.61 → 0.62, winreg 0.52 → 0.55, tauri 2.11 → 2.12 (group bump).
- Tooling: fmt/clippy CI gates, full-history gitleaks scan, Dependabot, dev toolchain (TypeScript 7, Vite 8); hardened release packager (staged outputs with rollback).

### Removed
- The frozen WPF binaries no longer ship inside release zips (the v1.5.0-built binaries remain downloadable from the v1.5.0 release; the `wpf/` source stays in-tree as the behavior/UI reference). Source-snapshot zip shrinks ~63 MB → 3.7 MB.

### Security
- rustls 0.23.45 (runtime) and source-map-js 1.2.2 (dev-only) security updates.

## [1.7.2] - 2026-09-04

### Added
- Hover glow: an accent ring + soft halo fades in (140 ms) over the panel's bottom buttons, the CLI version row, tray menu items and the Skills window's Refresh button, with the background-color change transitioned in sync. Pre-painted on a pseudo-element, opacity-only animation (compositor-friendly, no per-frame repaint).

## [1.7.1] - 2026-09-04

Review-hardening round; no new features.

### Fixed
- Overflow safety: saturating arithmetic in refresh-interval scheduling and the Extra Usage 1e-8-yuan conversion; refresh interval clamped to 1–30 min at the IPC boundary; non-finite quota percentages zeroed.
- Race conditions: a stale reschedule hint can no longer clobber the 2 s first refresh; panel hide/show animation guard; window event listeners registered before any await; settings backfill tolerates malformed persisted values.
- Manual refreshes debounce within 2 s; credentials honor the `KIMI_CODE_HOME` override; Skills window dropped the never-functional disabled badge (Kimi Code persists no per-skill state).

### Security
- Dropped the unused `opener:default` capability; added a minimal CSP (verified on the release build).

## [1.7.0] - 2026-08-29

### Added
- Console button (leftmost action) opening the Kimi Code console in your browser.
- Icon action row: all four buttons (Console / Refresh / Settings / Exit) show a 16 px inline-SVG icon above the label; panel height 512 → 520.

### Changed
- Docs restructure: UI-SPEC merged into `docs/SPEC.md` (project-level part 1; behavior chapters renumbered 10–21); English translation added at `docs/SPEC_EN.md`.

## [1.6.0] - 2026-08-22

### Added
- Skills window (Rust edition): read-only, grouped list of local Kimi Code skills (`~/.kimi-code/skills`, `~/.agents/skills`, managed plugins); lazy scan on open, cached, zero background polling.

### Changed
- WPF edition declared unmaintained: frozen at v1.5.0, source kept for reference; new features land in the Rust edition only.

## [1.5.0] - 2026-08-15

### Changed
- Full English UI copy for both editions, terminology aligned with the Kimi console (Weekly usage, 5-hour usage, Extra Usage, "Resets in…", Not activated / No data, Moonlit / Moondark). Copy only — no functional or layout changes.

## [1.4.0] - 2026-08-08

### Added
- Rust edition (Tauri 2) alongside the WPF edition: identical UI/UX, no .NET dependency, single-file exe, feature parity; monorepo layout (`wpf/` + `rust/`) with a unified version.

### Fixed
- Drop shadow fully decays before the window edge (both editions) — the faint rectangular halo around the panel is gone.
- Mixed-DPI multi-monitor hardening: window sizes pinned on show; tray menu converts cursor pixels using the cursor's monitor DPI; first-open placement computed in physical pixels regardless of the launching monitor.

## [1.3.0] - 2026-08-02

### Added
- Tray hover prefetch: hovering the tray icon refreshes quota in the background (10 s throttle) — zero steady-state overhead (no new timers).

### Changed
- CLI version check reads the official changelog first (4 KB range request, GitHub API fallback) — works even when api.github.com is unreachable or rate-limited.

### Fixed
- Booster wallet honors `isEnabled` — untopped wallets no longer risk showing an estimated value as the balance.

## [1.2.0] - 2026-08-02

### Added
- Extra Usage card: booster wallet balance with monthly charge usage/limit progress; graceful "not activated / no data" states for untopped wallets; balance retained on transient fetch failures (keep-last-good).

## [1.1.0] - 2026-08-01

### Added
- Themed right-click tray menu (self-drawn, unified with the popup/settings visuals); redesigned settings window (custom title bar, themed radio/checkbox, segmented interval pills); popup slide + fade animations.

### Fixed
- HiDPI: tray menu positioned correctly at any scaling rate (125 %–250 %+).

## [1.0.0] - 2026-08-01

Initial release: Windows tray app for Kimi Code plan quota — 5-hour and weekly usage cards with reset countdowns, Moonlit/Moondark themes following the system, portable and UAC-free (.NET 8 / WPF).

[Unreleased]: https://github.com/shawn-0106t/kimi-planbar-tray/compare/v1.7.3...HEAD
[1.7.3]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.7.3
[1.7.2]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.7.2
[1.7.1]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.7.1
[1.7.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.7.0
[1.6.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.6.0
[1.5.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.5.0
[1.4.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.4.0
[1.3.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.3.0
[1.2.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.2.0
[1.1.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.1.0
[1.0.0]: https://github.com/shawn-0106t/kimi-planbar-tray/releases/tag/v1.0.0

---

# 更新日志（中文）

> 以下为上方英文各版本的中文对照（发版时双语同步维护）；版本链接见英文区。

## [Unreleased]

### 新增
- WPF 版复活并对齐 rust/ 1.7.3：Skills 只读窗口端到端（扫描器 + 窗口 + 托盘菜单项 + 首开懒扫描缓存 + 打开期间抑制面板失焦收起，外加 `--test-skills` 与 `--test-ui` 第四行 OK），带 Console 按钮的图标动作行（面板高度 512 → 520，四按钮图标化且底色改卡片色），自绘 hover glow（SPEC 15.3：accent 描边 + 光晕、140ms、随主题取参），以及 1.7.1 加固回移植（`KIMI_CODE_HOME` 覆盖、手动刷新 2s 防抖、保存时 RefreshMinutes clamp 1–30、饱和算术、非有限百分比归零、过期 reschedule 不覆盖 2s 首刷、hide/show 动画代际守卫、设置回填容忍损坏持久化值）。WPF 版恢复随 Releases 分发。
- tag 触发的发版自动化：push `v*` tag 后在 CI 上构建 release exe、运行打包脚本并创建挂好三件资产的 draft GitHub Release——发布仍为手动决策。
- 实验性 egui 版（Phase 1 + Phase 2 + Phase 3）：从 rust/ 移植无头业务核心（credentials、quota、polling、settings、skills、update、theme_watch、state）并去除 Tauri 依赖，`--test-fetch`/`--test-update` 输出与 Rust 版逐字段一致。Phase 2 落地托盘图标与主面板，达到 SPEC 10-15 parity：左键 toggle、hover 刷新 10s 节流、失焦自动收起 + 300ms 再入防抖、出入场滑入淡入动画、140ms hover 光晕、Moonlit/Moondark 双主题。Phase 3 补齐四窗 parity：设置窗（主题单选、刷新间隔丸、开机自启复选框、Save 链路含主题实时生效与轮询重排、可拖动标题栏）、Skills 只读窗（首开懒扫描 + 缓存、分组列表、描述两行截断 + 悬停完整提示、主题化滚动条，开发机上 `112 skills` 与文件系统逐一核对）、托盘右键菜单（光标定位、右缘钳制、底边上翻、Open/Refresh/Settings/Skills/Exit、失焦关闭）——`--test-ui` 现构造全部四窗并输出 `MainWindow OK` / `SettingsWindow OK` / `SkillsWindow OK` / `TrayMenuWindow OK`。
- egui 版完结（Phase 4 + Phase 5）：SPEC 10-21 全面 parity 审计（99 PASS / 2 项轻微偏差 / 6 项环境不可测，跨版本互斥矩阵 9/9，混合 DPI 审计，同机性能对照——冷启动 91 ms、单 exe ~21.8 MB，对比 Rust 版 1060 ms + WebView2），修复两处真实 parity bug（经 `GetDpiForWindow` 的混合 DPI 窗口定位、设置/Skills 窗仅首开居中），两轮 code review 修复（Alt+F4/WM_CLOSE 根窗口关闭守卫、按 PID 过滤的 HWND 查找、退出快捷键/缩放加固），CI 接入（ci.yml 对 `egui/` 运行同款 fmt/clippy/build/test 门禁），以及最终定位决策：**experimental 测试版，与 qt/ 同款**——仅从源码构建，不随 Releases 分发。
- 已知限制文档化：egui 版在亮色桌面下窗口矩形边缘有一圈 1-2px 的 DWM hairline（winit 0.30.13 无装饰窗口 NCCALCSIZE hack + WS_CAPTION 边框叠绘所致；tao 的 inset 方案是 rust/ 无此问题的原因）——2026-10-17 完成根因定位，修复尝试经实测验证后回滚，experimental 版接受现状；完整诊断与后续选项见 `docs/archive/EGUI-MIGRATION.md` 第 7 节。（已被取代：本周期内已修复——见下方"修复"。）

### 修复
- egui 版窗口在 Windows 11 26H2（build 26300+）上的原生外框：DWM 开始把窗口保留的 WS_CAPTION 样式画成完整的白色标题栏 + 关闭按钮覆盖在透明内容上，且 winit 0.30.13 无装饰阴影 hack（WM_NCCALCSIZE `top += 1`）在每个窗口顶边留下一条 1px 灰线（即此前文档化的 "hairline" 已知限制，现已消除）。修复位于 `egui/src/windows.rs::style_hwnd`：以 `DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED` 关闭 DWM 非客户区绘制，并子类化窗口过程把非客户区完全折叠（egui UI 按 SPEC 10 自绘阴影，该 hack 所保留的 DWM 阴影本就冗余）。HWND 样式 watcher 改为窗口句柄一出现就处理（而非首次显示时），因此窗口首次显示即无外框；并在每次 隐藏→显示 跳变时重新处理，因为 winit 的 `apply_diff` 在每次可见性切换时都会用其自身标志集重写 GWL_STYLE/GWL_EXSTYLE（抹掉我们添加的 WS_EX_TOOLWINDOW）。
- egui 版窗口激活变化瞬间的单帧白色原生标题栏闪烁（同一 26H2 根因族）：`DWMNCRP_DISABLED` 关掉 DWM 的非客户区绘制后，winit 转发 `WM_NCACTIVATE` 时 `DefWindowProc` 会用 GDI 直接画 caption（该消息是 winit 唯一设置 `is_active` 的途径，不能吞掉）。修复位于 `egui/src/windows.rs::nc_collapse_proc`：转发调用期间临时剥除 WS_CAPTION、返回后立即恢复，DefWindowProc 找不到 caption 可画。经高速连拍回归覆盖全部四个激活场景（打开设置、托盘右键菜单、菜单项开各窗口、关设置焦点回落）——无闪帧。
- egui 版 UI 唤醒丢失（同一工作弧中的"窗口状态脱钩"追查）：eframe 0.36.2 会把两次 pass 之间发出的 `UserEvent::RequestRepaint` 判为 "outdated" 丢弃，而恰在 pass 突发结束时发出的唤醒可能是最后一个唤醒源——被丢后队列里的命令一直无人 drain，直到某个无关事件到来（实测表现为托盘 TogglePanel 点击丢失、settings 窗口 X 按钮点击后 `SettingsClose` 永不到达）。修复位于 `egui/src/app.rs::wake_with_retry`：每次跨线程入队额外在 120ms 后补发一次唤醒；egui 的 repaint-delay 门控会在 pass 正常流动时抑制这次补发，常态零成本。
- egui 版重新显示窗口的点击失灵：egui-winit 只从 `CursorMoved` 学习指针位置，位置未知时丢弃全部鼠标按键输入；而 winit 在 隐藏→显示 周期间保留 `mouse.last_position`——窗口在光标原地不动的情况下重新显示时不产生 `CursorMoved`（位置未变），点击被忽略直到光标移到新位置。修复位于 `egui/src/windows.rs::spawn_pointer_rearm`：每次显示时 post 两条携带当前光标位置的合成 WM_MOUSEMOVE（第二条偏移 1px 以打破与陈旧坐标的平局）。
- WPF 版主面板在图标动作行与卡片圆角上偏离 rust 基线：动作行四图标整体偏大且比例互相漂移，原因是 `Viewbox` 内的 `Path` 未显式给尺寸——裸 `Path` 按自身 geometry bounds 计量，Viewbox 缩放系数成了 16/bounds 而非 SVG viewBox 的 16/24（四个 Path 已补显式 `Width`/`Height="24"`）；动作按钮、版本行与 Skills 窗口 Refresh 按钮的静止底色画在模板 `Grid`（直角矩形）上，在 `WindowBg` 上露出直角（圆角的 hover/glow 层只在悬停时淡入）——静止底色改由最底层圆角 `Border` 承载（CornerRadius 10 / 12 / 8）；`IconActionButton` Padding 修正为 SPEC 12.7 的 `0,5,0,6`（原 `5,0,5,6` 系对 CSS `padding: 5px 0 6px` 简写的误读）。SPEC 15.3 的 WPF 模板层描述已双语同步。

### 变更
- WPF 版恢复维护（不再冻结于 v1.5.0）：`<Version>` 纳入共享版本同步清单，`ci.yml` 新增编译级 `dotnet build wpf -c Release` 门禁，`release.yml` 发布两个 WPF exe——release 资产从三件变五件（`KimiPlanbarTray-wpf.exe` + `KimiPlanbarTray-wpf-selfcontained.exe`，沿用 v1.5.0 历史命名；`.gitignore` 的 `KimiPlanbarTray-*.exe` 通配已覆盖）。SPEC 10.1/10.2 窗口尺寸修正为现行的 424×520 / 404×464。
- 版本 bump 清单从 9 处扩展为 11 处：egui/ 版本号（`egui/Cargo.toml` + `egui/Cargo.lock` 的 `kimi-planbar-tray-egui` 条目）改为随 rust/qt 主清单同步（1.7.3），不再独立维护 0.1.0。迁移方案与交接文档移入 `docs/archive/`（`EGUI-MIGRATION.md`、`HANDOFF-egui.md`）。
- egui 版自 2026-10-17 起冻结（用户决策——不再继续维护；仍可从源码构建并保留 CI 门禁）。接受一项残留已知问题：窗口在实例内的首次显示时，原生标题栏可能持续出现数秒（闪帧回归中三个实例首秀仅观察到一次，自行消失；无法按需复现）。详见 `docs/archive/HANDOFF-egui-26H2-ncframe.md`。

## [1.7.3] - 2026-10-10

维护版本——Rust 版无功能与 UI 变化。

### 新增
- 实验性 Qt 版（C++ Qt6 Widgets，无 WebView 依赖）源码入库，与 rust/ 达到 SPEC parity；仅从源码构建，不随 Releases 分发。

### 变更
- 运行时依赖刷新：reqwest 0.12 → 0.13、windows 0.61 → 0.62、winreg 0.52 → 0.55、tauri 2.11 → 2.12（组升级）。
- 工具链：fmt/clippy CI 门禁、全历史 gitleaks 扫描、Dependabot、开发工具链（TypeScript 7、Vite 8）；发布打包器加固（暂存产物 + 回滚）。

### 移除
- 冻结的 WPF 二进制不再打入 release zip（v1.5.0 构建的二进制仍可在 v1.5.0 Release 下载；`wpf/` 源码保留在仓库内作为行为/UI 参照）。源码快照 zip 从约 63 MB 降至 3.7 MB。

### 安全
- rustls 0.23.45（运行时）与 source-map-js 1.2.2（仅开发依赖）安全更新。

## [1.7.2] - 2026-09-04

### 新增
- 悬停光晕：强调色描边 + 柔和光晕（140 ms）淡入面板底部按钮、CLI 版本行、托盘菜单项与 Skills 窗口的 Refresh 按钮，背景色切换同步过渡。光晕预绘制在伪元素上、仅过渡 opacity（合成器友好，无逐帧重绘）。

## [1.7.1] - 2026-09-04

审查加固轮次；无新功能。

### 修复
- 溢出安全：刷新间隔调度与 Extra Usage 1e-8 元换算改饱和算术；刷新间隔在 IPC 边界钳制到 1–30 分钟；非有限额度百分比归零。
- 竞态：过期的重调度提示不再覆盖 2 秒首刷；面板隐藏/显示动画守卫；窗口事件监听先于任何 await 注册；设置回填容忍损坏的持久化值。
- 手动刷新 2 秒防抖；凭据遵循 `KIMI_CODE_HOME` 覆盖；Skills 窗口移除从未生效的 disabled 徽章（Kimi Code 不持久化 per-skill 状态）。

### 安全
- 移除未使用的 `opener:default` capability；添加最小 CSP（release 构建上验证）。

## [1.7.0] - 2026-08-29

### 新增
- Console 按钮（最左侧动作位）在浏览器打开 Kimi Code 控制台。
- 图标动作行：四个按钮（Console / Refresh / Settings / Exit）在标签上方显示 16 px 内联 SVG 图标；面板高度 512 → 520。

### 变更
- 文档重组：UI-SPEC 并入 `docs/SPEC.md`（新增项目级第一篇；行为章节重编为 10–21）；新增英文翻译 `docs/SPEC_EN.md`。

## [1.6.0] - 2026-08-22

### 新增
- Skills 窗口（Rust 版）：只读、分组展示本地 Kimi Code skills（`~/.kimi-code/skills`、`~/.agents/skills`、托管插件）；首次打开惰性扫描并缓存，零后台轮询。

### 变更
- WPF 版宣告停止维护：冻结于 v1.5.0，源码保留参考；新功能只进 Rust 版。

## [1.5.0] - 2026-08-15

### 变更
- 双版本 UI 文案全面英文化，术语对齐 Kimi 控制台（Weekly usage、5-hour usage、Extra Usage、"Resets in…"、Not activated / No data、Moonlit / Moondark）。纯文案调整——无功能与布局变化。

## [1.4.0] - 2026-08-08

### 新增
- Rust 版（Tauri 2）与 WPF 版并存：UI/UX 一致、无 .NET 依赖、单文件 exe、功能对齐；monorepo 结构（`wpf/` + `rust/`）统一版本号。

### 修复
- 阴影在到达窗口边缘前完全衰减（双版本）——面板周围的方框光晕消失。
- 多屏异 DPI 加固：显示时锁定窗口尺寸；托盘菜单用光标所在显示器的 DPI 换算坐标；首开落点按物理像素计算，与启动所在屏无关。

## [1.3.0] - 2026-08-02

### 新增
- 托盘悬停预热：鼠标悬停图标时后台预取额度（10 秒节流）——零稳态开销（无新增定时器）。

### 变更
- CLI 版本检测优先读取官方 changelog（4 KB Range 请求，GitHub API 兜底）——api.github.com 不可达或限流时也能工作。

### 修复
- booster 钱包遵循 `isEnabled`——未启用的钱包不再有把估算值误显示为余额的风险。

## [1.2.0] - 2026-08-02

### 新增
- Extra Usage 卡片：booster 钱包余额 + 本月已用/上限进度；未开通钱包优雅显示"未开通 / 无数据"；瞬时拉取失败保留旧值（keep-last-good）。

## [1.1.0] - 2026-08-01

### 新增
- 主题化托盘右键菜单（自绘，与悬浮窗/设置窗视觉统一）；设置窗重绘（自定义标题栏、主题化单选/复选、分段选项丸）；悬浮窗滑动 + 淡入动画。

### 修复
- HiDPI：托盘菜单在任意缩放比（125 %–250 %+）下正确落位。

## [1.0.0] - 2026-08-01

初始发布：Kimi Code 套餐额度的 Windows 托盘应用——5 小时/每周用量卡片与重置倒计时、月之亮面/暗面双主题跟随系统、便携免管理员（.NET 8 / WPF）。

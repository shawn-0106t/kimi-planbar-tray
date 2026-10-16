# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Entries for v1.7.2 and earlier were reconstructed from the GitHub release notes of each tag.

## [Unreleased]

### Added
- Tag-triggered release automation: pushing a `v*` tag now builds the release exe on CI, runs the packager, and opens a draft GitHub Release with all three assets attached — publishing stays a manual decision.
- Experimental egui edition (Phase 1 + Phase 2 + Phase 3): the headless core ported from rust/ (credentials, quota, polling, settings, skills, update, theme_watch, state) with the Tauri dependencies removed, plus `--test-fetch`/`--test-update` output parity with the Rust edition. Phase 2 adds the tray icon and the main panel at SPEC 10-15 parity: left-click toggle, 10 s hover-refresh throttle, focus-loss auto-hide with the 300 ms re-entry guard, enter/exit slide+fade animations, the 140 ms hover glow, and Moonlit/Moondark themes. Phase 3 completes the four-window parity: the Settings window (theme radios, refresh-interval pills, autostart checkbox, Save chain with live theme apply + polling reschedule, draggable titlebar), the read-only Skills window (lazy one-shot scan with cache, grouped list, 2-line description clamp with hover tooltips, themed scrollbar, `112 skills` verified against the filesystem on the dev machine), and the tray right-click menu at the cursor (edge clamping, bottom-edge flip-up, Open/Refresh/Settings/Skills/Exit, focus-loss close) — `--test-ui` now constructs all four windows and prints `MainWindow OK` / `SettingsWindow OK` / `SkillsWindow OK` / `TrayMenuWindow OK`.
- egui edition completion (Phase 4 + Phase 5): a full SPEC 10-21 parity audit (99 PASS / 2 minor DEVIATION / 6 UNTESTABLE, cross-edition mutex matrix 9/9, mixed-DPI audit, same-machine perf comparison — 91 ms cold start and a ~21.8 MB single exe vs 1060 ms + WebView2 for the Rust build) with two real parity fixes (mixed-DPI window positioning via `GetDpiForWindow`, first-open-only centering for the settings/skills windows), two code-review fix rounds (root close guard for Alt+F4/WM_CLOSE, PID-filtered HWND lookup, quit-shortcut/zoom hardening), CI coverage (the same fmt/clippy/build/test gate now runs on `egui/` in ci.yml), and the final positioning decision: **experimental test-only, like qt/** — buildable from source, not distributed via Releases.

### Changed
- The version bump checklist grows from 9 to 11 files: the egui/ edition's version (`egui/Cargo.toml` + the `kimi-planbar-tray-egui` entry in `egui/Cargo.lock`) now follows the rust/qt sync list (1.7.3) instead of its independent 0.1.0. The migration plan and handoff docs moved to `docs/archive/` (`EGUI-MIGRATION.md`, `HANDOFF-egui.md`).

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
- tag 触发的发版自动化：push `v*` tag 后在 CI 上构建 release exe、运行打包脚本并创建挂好三件资产的 draft GitHub Release——发布仍为手动决策。
- 实验性 egui 版（Phase 1 + Phase 2 + Phase 3）：从 rust/ 移植无头业务核心（credentials、quota、polling、settings、skills、update、theme_watch、state）并去除 Tauri 依赖，`--test-fetch`/`--test-update` 输出与 Rust 版逐字段一致。Phase 2 落地托盘图标与主面板，达到 SPEC 10-15 parity：左键 toggle、hover 刷新 10s 节流、失焦自动收起 + 300ms 再入防抖、出入场滑入淡入动画、140ms hover 光晕、Moonlit/Moondark 双主题。Phase 3 补齐四窗 parity：设置窗（主题单选、刷新间隔丸、开机自启复选框、Save 链路含主题实时生效与轮询重排、可拖动标题栏）、Skills 只读窗（首开懒扫描 + 缓存、分组列表、描述两行截断 + 悬停完整提示、主题化滚动条，开发机上 `112 skills` 与文件系统逐一核对）、托盘右键菜单（光标定位、右缘钳制、底边上翻、Open/Refresh/Settings/Skills/Exit、失焦关闭）——`--test-ui` 现构造全部四窗并输出 `MainWindow OK` / `SettingsWindow OK` / `SkillsWindow OK` / `TrayMenuWindow OK`。
- egui 版完结（Phase 4 + Phase 5）：SPEC 10-21 全面 parity 审计（99 PASS / 2 项轻微偏差 / 6 项环境不可测，跨版本互斥矩阵 9/9，混合 DPI 审计，同机性能对照——冷启动 91 ms、单 exe ~21.8 MB，对比 Rust 版 1060 ms + WebView2），修复两处真实 parity bug（经 `GetDpiForWindow` 的混合 DPI 窗口定位、设置/Skills 窗仅首开居中），两轮 code review 修复（Alt+F4/WM_CLOSE 根窗口关闭守卫、按 PID 过滤的 HWND 查找、退出快捷键/缩放加固），CI 接入（ci.yml 对 `egui/` 运行同款 fmt/clippy/build/test 门禁），以及最终定位决策：**experimental 测试版，与 qt/ 同款**——仅从源码构建，不随 Releases 分发。

### 变更
- 版本 bump 清单从 9 处扩展为 11 处：egui/ 版本号（`egui/Cargo.toml` + `egui/Cargo.lock` 的 `kimi-planbar-tray-egui` 条目）改为随 rust/qt 主清单同步（1.7.3），不再独立维护 0.1.0。迁移方案与交接文档移入 `docs/archive/`（`EGUI-MIGRATION.md`、`HANDOFF-egui.md`）。

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

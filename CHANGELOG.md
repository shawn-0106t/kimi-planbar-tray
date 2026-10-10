# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html). Entries for v1.7.2 and earlier were reconstructed from the GitHub release notes of each tag.

## [Unreleased]

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

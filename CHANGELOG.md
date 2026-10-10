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

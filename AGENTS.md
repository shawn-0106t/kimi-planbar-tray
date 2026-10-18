# AGENTS.md — Kimi Planbar Tray

Guidance for AI coding agents working in this repository. Read this first; it assumes no prior knowledge of the project.

## Project overview

Kimi Planbar Tray is a lightweight **Windows-only system tray app** that shows Kimi Code plan quota (5-hour window + weekly usage, reset countdowns, "Extra Usage" booster wallet) one click away from the tray. It reads the local Kimi Code CLI OAuth token and calls `GET https://api.kimi.com/coding/v1/usages`.

Current version: **1.8.0** (full sync list: `docs/SPEC.md` section 7.3 step 1).

This is a **monorepo with four editions**:

- `rust/` — **actively developed (dual-track with `qt/`)**. Tauri 2 + Rust backend + vanilla HTML/CSS/TypeScript frontend (no framework, no React).
- `qt/` — **completed, experimental**. C++ Qt6 + Qt Widgets, no WebView dependency; reached SPEC parity with rust/ 1.7.3. Architecture plan, stack comparison, and module mapping: `docs/archive/QT-MIGRATION.md` (archived).
- `egui/` — **completed, experimental (test-only, like qt/ — builds a standalone exe but is NOT distributed via Releases; CI-covered)**. Pure-Rust UI: egui (eframe, pinned 0.36.2) + tray-icon — no WebView, no Node toolchain. All phases (0-5) are complete: headless business core ported from rust/, tray icon + main panel at SPEC 10-15 parity, the settings/skills/tray-menu windows at SPEC 10.2/10.3/13/21 parity — `--test-ui` prints all four OK lines — the Phase 4 SPEC 10-21 parity audit: 99 PASS / 2 minor DEVIATION / 6 UNTESTABLE, cross-edition mutex matrix 9/9, mixed-DPI audit, perf numbers, plus two positioning fixes — `window_dpi_scale` and first-open-only centering for settings/skills — and Phase 5 closure (version synced to 1.7.3 with the rust/qt list, CI gate, docs archived). **Frozen since 2026-10-17 (user decision — no further maintenance; do not add features or chase the residual known issues; they are accepted as-is and documented in `docs/archive/HANDOFF-egui-26H2-ncframe.md`).** Architecture plan and phase record: `docs/archive/EGUI-MIGRATION.md` (archived).
- `wpf/` — native .NET 8 / WPF edition, **maintained again since its 1.7.3 revival** (aligned with rust/: the Skills window, the icon action row, the hover glow, and the 1.7.1 hardening backport; zero third-party NuGet dependencies remains a hard rule). Ships via Releases again as two exes: `KimiPlanbarTray-wpf.exe` (framework-dependent, needs the .NET 8 Desktop Runtime) and `KimiPlanbarTray-wpf-selfcontained.exe`. Revival plan and execution record (archived): `docs/archive/WPF-REVIVAL.md`.

All editions share the same UI/UX (fully specified in `docs/SPEC.md`) and the same `settings.json` schema.

Other top-level files and directories:

- `docs/SPEC.md` — the single authoritative project spec (in Chinese; English translation at `docs/SPEC_EN.md`, identical chapter numbering): Part 1 (chapters 1-9) covers project scope, architecture, data flow, security, build/release; Part 2 (chapters 10-21) is the behavior/UI detail spec (window sizes, colors, animation timings, API parsing rules). Consult it before changing behavior.
- `docs/archive/QT-MIGRATION.md` — Qt edition (C++ Qt6 Widgets) migration plan (archived; development complete): GitHub case survey, Rust-vs-Qt stack comparison, module mapping, phased roadmap.
- `docs/*.png` — reference screenshots for visual comparison (regenerate with `scripts/release/make_screenshots.py`).
- `docs/archive/HANDOFF.md` — archived history of the WPF→Rust rewrite (in Chinese); frozen, do not update. The archive also holds the completed Qt-edition handoffs (`HANDOFF-qt.md`, `QT-MIGRATION.md`, `HANDOFF-code-review.md`, `HANDOFF-docs-update.md`), the scripts/ reorganization + release-artifact handoff (`HANDOFF-scripts-reorg.md`), the completed egui-edition records (`EGUI-MIGRATION.md`, `HANDOFF-egui.md`, plus the Win11 26H2 frame/flash/wake-loss fix arc `HANDOFF-egui-26H2-ncframe.md`), and the completed WPF-revival plan + execution record (`WPF-REVIVAL.md`) — historical snapshots, likewise frozen.
- `scripts/release/` — maintained tooling: `make_release_zip.py` (release packaging, see Release process), `make_screenshots.py` (regenerates `docs/screenshot-*.png` via headless Chrome/Edge, `KPT_CHROME` overrides the browser, see Testing / self-checks), `verify_icons.py` (byte-compares the inline button SVGs in `rust/index.html` against the source icon library).
- `scripts/diagnostics/` — one-off diagnostic/measurement scripts kept for reference, all path-argument driven: `csp_visual_check.ps1` (shows the tray panel via UIAutomation, locale-independent, and screenshots the bottom-right screen region; verifies the WebView renders under the CSP in `tauri.conf.json` on a release exe), `dump_tray_windows.ps1`, `dump_egui_styles.ps1` (`-TargetPid`; window style/exstyle/rect + DWM extended-frame bounds dump), `ncfix_experiment.ps1` (launches the egui `--test-ui`, applies DWMWA_NCRENDERING_POLICY and/or strips WS_CAPTION on the live windows, screenshots the virtual screen), `measure_run.ps1`, `inspect_window_dpi.ps1` (`-ExePath`), `analyze_wpf_shadow.py` (PNG path as argv[1]).
- `.github/workflows/ci.yml` — GitHub Actions checks on every push to `main` and every PR: a `gitleaks` full-history secret-scan job and a `build-and-test` gate (fmt/clippy + frontend tsc/build + cargo build/test on rust/ and egui/, plus a compile-level dotnet build on wpf/). It runs no runtime `--test-*` self-checks (no Kimi Code credentials / no interactive desktop on hosted runners). Full job contents and branch-protection details: SPEC section 7.2.
- `.github/dependabot.yml` — weekly Dependabot updates for cargo (`/rust/src-tauri` and `/egui`), npm (`/rust`), and github-actions: minor+patch grouped into one PR per ecosystem, majors open individual PRs. Dependabot alerts and security updates are enabled on the repo. The `main` branch is protected — both CI jobs are required checks (admins included), so changes land via PR and direct pushes to `main` are rejected.
- `.github/workflows/release.yml` — tag-triggered (`v*` push) draft-release pipeline (gitleaks first, then build + package + **draft** Release with the five assets); publishing stays manual. Details: SPEC section 7.3 step 2.
- `CONTRIBUTING.md` / `CHANGELOG.md` / `SECURITY.md` / `CODE_OF_CONDUCT.md` — the four community files are all English-first-with-Chinese-below; AGENTS.md itself stays English-only. They cover: contributor-facing dev setup, CI gates and conventions (CONTRIBUTING); per-release change log in Keep a Changelog format, English + Chinese updated together as Release-process step 1 (CHANGELOG); the security policy — private vulnerability reporting, supported versions, token data-flow scope (SECURITY); and Contributor Covenant v2.1 with a Chinese translation whose English text prevails (CODE_OF_CONDUCT; conduct reports go through the private reporting channel). `.github/ISSUE_TEMPLATE/` (bilingual bug/feature templates + contact link) and `.github/PULL_REQUEST_TEMPLATE.md` (bilingual verification checklist mirroring CONTRIBUTING) structure issue/PR creation — field-level "English / 中文" labels, not separate per-language template sets.

## Repository layout (active code)

```
rust/
├── index.html / settings.html / skills.html / menu.html   # one page per window
├── src/                         # vanilla TS frontend (Vite multi-page build)
│   ├── main.ts / settings.ts / skills.ts / menu.ts        # per-window logic
│   ├── common.ts                # shared DTO types + formatting helpers (fmtYuan, formatReset, clampPercent, initTheme)
│   ├── theme.css                # Moonlit/Moondark CSS variables (single source of theme colors)
│   └── main.css / settings.css / skills.css / menu.css
├── vite.config.ts               # 4-page rollup input; assetsInlineLimit 100 KB inlines the logo
└── src-tauri/
    ├── Cargo.toml               # tauri 2, reqwest, tokio, serde, windows 0.62, winreg, regex, chrono
    ├── tauri.conf.json          # 4 frameless/transparent/always-on-top windows: main 424x520, settings 404x464, skills 404x520, menu 188x160; minimal CSP (script-src 'self', style-src + 'unsafe-inline', img-src + data:)
    ├── capabilities/default.json# permissions: core:default only, all 4 windows
    └── src/
        ├── main.rs              # entry; handles --test-* args BEFORE single-instance check; named mutex KimiPlanbarTray.SingleInstance
        ├── lib.rs               # Tauri builder, all #[tauri::command]s, window event routing (focus-loss / hide-on-close), --test-ui
        ├── credentials.rs       # token chain (KIMI_CODE_HOME-aware kimi_home): credentials/kimi-code.json -> config.toml fallback
        ├── quota.rs             # HTTP fetch + defensive JSON parsing (see traps below)
        ├── polling.rs           # refresh timer, 2 s first refresh, 30 s fast retry on failure, keep-last-good
        ├── tray.rs              # tray icon, left-click toggle, right-click menu window, tooltip text
        ├── panel.rs             # panel positioning/animation, focus-loss auto-hide, 300 ms re-entry guard
        ├── settings.rs          # settings.json persistence, portable.dat detection, HKCU Run autostart
        ├── skills.rs            # read-only scan of local Kimi Code skills (v1.6 feature, SPEC section 21)
        ├── update.rs            # kimi --version + changelog Range request + GitHub API fallback
        ├── theme_watch.rs       # system light/dark watch via registry
        └── state.rs             # AppState shared state (RwLock fields, skills cache, reschedule notify, manual-refresh debounce)

qt/                              # COMPLETED (experimental): C++ Qt6 + Widgets (rationale & mapping: docs/archive/QT-MIGRATION.md sections 2-3)
├── CMakeLists.txt               # find_package(Qt6 COMPONENTS Widgets Network Svg); project(VERSION ...) synced with rust/
├── package_release.py           # one-shot rebuild + windeployqt into qt/dist/ (28 files / ~36 MB, directory form); KPT_QT_DIR/KPT_CMAKE/KPT_MSVC_REDIST overrides
└── src/
    ├── main.cpp                 # entry; --test-* args BEFORE single-instance check; same named mutex; WriteFile(STD_OUTPUT_HANDLE) for test output
    ├── app.h/.cpp               # window singletons, event routing, DWM corner opt-out, theme watch (QStyleHints::colorSchemeChanged)
    ├── credentials.h/.cpp       # same token chain as rust/ (credentials/kimi-code.json -> config.toml fallback)
    ├── quota.h/.cpp             # QNetworkAccessManager fetch + defensive QJsonDocument parsing
    ├── polling.h/.cpp           # QTimer scheduling: 2 s first refresh, 30 s fast retry on failure, keep-last-good
    ├── trayicon.h/.cpp          # QSystemTrayIcon: left-click toggle, right-click menu window, tooltip, hover via 500 ms geometry() polling
    ├── panel.h/.cpp             # panel/menu positioning (logical coords), focus-loss auto-hide, 300 ms re-entry guard
    ├── settings.h/.cpp          # settings.json (same schema), portable.dat detection, HKCU Run autostart via QSettings
    ├── skills.h/.cpp            # read-only skills scan (scan once on first open, cache; zero background cost)
    ├── update.h/.cpp            # kimi --version (QProcess) + changelog Range request + GitHub API fallback
    ├── state.h/.cpp             # AppState: last-good cache, skills cache, manual-refresh debounce
    ├── theme.h/.cpp / icons.h / format_util.h / http_util.h / async_util.h
    └── windows/                 # panel/settings/skills/menu QWidget windows + titlebar.h / clamplabel.h / hoverglow.h (self-drawn halo)

egui/                            # COMPLETED (experimental, test-only): pure-Rust UI (plan & phase record: docs/archive/EGUI-MIGRATION.md)
├── Cargo.toml                   # kimi-planbar-tray-egui (version synced with the rust/qt list); eframe =0.36.2 (glow+wgpu), tray-icon =0.26.1
├── src/
│   ├── main.rs                  # entry: --test-* args BEFORE the single-instance mutex (same mutex name); GUI assembly (SPEC 20 order, tray on main thread)
│   ├── lib.rs                   # module wiring: build_state + check_and_emit (ports of the Tauri lib.rs helpers)
│   ├── app.rs                   # eframe App: panel state machine, focus latches, root close guard, viewport registration/routing
│   ├── ctx.rs                   # AppCtx (shared state) + AppEvent bus, replacing AppHandle/emit
│   ├── panel_pos.rs             # show_panel/show_centered/show_menu_at positioning (work area, menu clamp + flip-up, window_dpi_scale)
│   ├── tray.rs / windows.rs / icons.rs / format_util.rs   # tray events, HWND post-processing (DWM donotround + NCRP-disabled + NC-collapse subclass + WS_EX_TOOLWINDOW), logo/icon decode, formatting
│   ├── credentials.rs / quota.rs / polling.rs / settings.rs / skills.rs / update.rs / theme_watch.rs / state.rs
│   │                            # 1:1 ports of the rust/src-tauri modules (de-tauri'd)
│   ├── ui/                      # theme.rs / panel.rs / widgets.rs / settings_win.rs / skills_win.rs / menu_win.rs (painter-drawn windows)
│   └── bin/spike.rs             # Phase 0 feasibility spike, kept untouched as reference

wpf/                             # maintained (net8.0-windows, WPF, zero third-party NuGet deps; aligned with rust/ 1.7.3)
├── MainWindow / SettingsWindow / SkillsWindow / TrayMenuWindow (.xaml + .xaml.cs)
├── TrayManager.cs, App.xaml.cs  # App.xaml.cs hosts --test-fetch/--test-update/--test-skills/--test-ui/--screenshot + the 2s manual-refresh debounce
├── Services/{QuotaService,SettingsService,SkillsService,UpdateService,ThemeService}.cs
└── Themes/{Shared,Light,Dark}.xaml  # Shared.xaml also carries the action-icon StreamGeometries + the glow-layer button styles
```

## Build and run

Prerequisites (Windows): Rust stable (MSVC toolchain), Node.js 18+, WebView2 Runtime. The WPF edition needs the .NET 8 SDK.

Rust edition (the one you normally build):

```bash
cd rust
npm install
npm run dev          # vite only (frontend); for full app: npx tauri dev
npx tauri build      # release exe at rust/src-tauri/target/release/kimi-planbar-tray.exe
```

Warning: a plain `cargo build` debug exe does **not** embed the frontend — its windows point at the Vite `devUrl`, so launching it without `npm run dev` shows a WebView2 "localhost ERR_CONNECTION_REFUSED" page and pops a console window (debug builds are console-subsystem). Only the release exe (and `npx tauri dev`) render the UI. The headless `--test-*` args work on the debug exe because they never load web content.

qt/, egui/ and wpf/ build commands and their caveats: SPEC section 7.1. One warning worth repeating: before rebuilding the qt edition, kill any running qt GUI process by PID/path (exe file lock) — never `taskkill /IM kimi-planbar-tray.exe` (the rust edition shares the image name).

Note: Bash on this machine is Git Bash — use POSIX syntax, forward slashes, single quotes. Python scripts run with `PYTHONUTF8=1` prefix (host is Chinese Windows, code page 936/GBK). Text file I/O in scripts must specify UTF-8 explicitly.

## Testing / self-checks

There is **no unit-test suite** beyond the `skills.rs` frontmatter-parser tests (`cargo test` runs only those; `npm test` runs nothing). Verification is via headless self-check command-line args, which run **before** the single-instance mutex so they work while a GUI instance is live (SPEC section 19):

```bash
kimi-planbar-tray.exe --test-fetch    # fetch quota once, print indented JSON, exit
kimi-planbar-tray.exe --test-update   # one-line: local=... latest=... updateAvailable=... checkFailed=...
kimi-planbar-tray.exe --test-ui       # construct all 4 windows, print "MainWindow OK" etc., exit after ~6 s
```

The qt/, egui/ and wpf/ editions implement the same args (qt and wpf add `--test-skills`; every `--test-ui` variant constructs and shows the four windows and prints the same four OK lines), so results can be diffed across editions on the same machine — e.g. run rust and qt `--test-fetch` back to back and compare the JSON field by field (the wpf edition's output is field-compatible: numeric text formatting and timestamp offset representation differ, see `docs/archive/WPF-REVIVAL.md` appendix C). The Rust edition has **no `--screenshot` arg** (WPF only); any unrecognized arg falls through to launching the GUI. The manual headless-Chrome screenshot recipe for `rust/dist/index.html` is in SPEC section 7.2.

After Rust changes: `cd rust && cargo build` (in `src-tauri/`) plus `npm run build` to bundle the frontend (`npm run build` is just Vite/esbuild and does no type checking — run `npx tsc --noEmit` for that), then run `--test-fetch` and `--test-ui` against the built exe. For visual changes, compare against `docs/*.png` screenshots per `docs/SPEC.md`. CI covers compile-level checks only (SPEC section 7.2); runtime self-checks remain local-only.

## Release process

Follow the checklist in `docs/SPEC.md` section 7.3 (mirror: `docs/SPEC_EN.md`): bump the version per the sync list (includes `<Version>` in `wpf/KimiPlanbarTray.csproj`) plus the "Current version" line at the top of this file, finalize the release's bilingual `CHANGELOG.md` entry (moved out of `Unreleased`), then push the `v<x.y.z>` tag — `.github/workflows/release.yml` builds, packages, and opens a **draft** Release with the five assets; review and publish manually. The manual fallback steps, the packager's `.part` failure semantics, and the local checksum-verification recipe are all documented there.

## Code style and conventions

- **Code, comments, and commit messages are in English**; conversation with the user is in Chinese. Repo docs are mixed: README in English, `docs/SPEC.md` in Chinese (English translation: `docs/SPEC_EN.md`), `docs/archive/QT-MIGRATION.md` in Chinese, `docs/archive/HANDOFF.md` in Chinese.
- Rust edition mirrors the WPF reference implementation **1:1** — when behavior is ambiguous, `docs/SPEC.md` is the contract and `rust/` is the implementation baseline (`wpf/` is aligned with it since its 1.7.3 revival). Comments in the Rust code cite SPEC sections (e.g. `SPEC 16.5`); keep those citations accurate when you change behavior.
- Error-handling baseline: all IO, registry, process, and HTTP failures are **silently swallowed** and surfaced only via UI text ("Update failed", "Not detected") or state fields. Never pop up error dialogs.
- Frontend renders external data (skill names/descriptions) with `textContent` only, never `innerHTML`.
- Tauri IPC: all frontend↔backend interaction goes through the `#[tauri::command]`s registered in `lib.rs::invoke_handlers()` plus events (`panel-show`, `settings-show`, `skills-show`, `update-status`). Windows are singletons — hidden, never destroyed; open paths must re-pin the logical size (DPI guard) and emit the `*-show` event.
- The Skills window is read-only and zero-background-cost: scan once on first open, cache in `AppState`, rescan only when the Refresh button passes `refresh=true`. There is no per-skill enabled/disabled state to display — Kimi Code does not persist one, and `~/.agents/.skill-lock.json` is lark-cli's installer lock file (no `disabled` key); it is not read at all.

## Critical behavioral traps (do not regress)

- **Extra Usage units**: `boosterWallet.balance.amountLeft` is in **1e-8 yuan**; convert to cents with `(raw + 500000) / 1000000`. `priceInCents` fields are already cents. All JSON numbers are modeled as **strings** (tolerate numeric fallback).
- **`isEnabled=false` trap**: when the booster wallet is not enabled, `amountLeft` is an estimate, not a real balance — the whole card must show "Not activated".
- **Credential chain**: `~/.kimi-code/credentials/kimi-code.json` `access_token` (valid only if `expires_at` > now + 30 s) → fallback to an `api_key` in `~/.kimi-code/config.toml` whose `[providers.*]` section has `base_url` containing `api.kimi.com/coding`. `KIMI_CODE_HOME` overrides the home dir.
- **Failure semantics**: on fetch failure keep the last good values on screen and retry after 30 s; on success return to the configured interval (1/5/10/30 min, default 5). First refresh fires 2 s after launch.
- **Single instance**: named mutex `KimiPlanbarTray.SingleInstance` is created in `main.rs` (in addition to `tauri-plugin-single-instance`) so the Rust, Qt, and WPF editions are mutually exclusive. Do not rename it.
- **DPI rules (Tauri/tao)**: position windows with `PhysicalPosition` (panel against primary work area, menu against the cursor's monitor) but size them with `LogicalSize` — giving physical sizes gets double-scaled on `WM_DPICHANGED`. See SPEC section 20.
- **Portable mode**: an empty `portable.dat` next to the exe redirects `settings.json` to the exe directory instead of `%APPDATA%\KimiPlanbarTray\`.
- **Win11 corners**: `lib.rs::disable_dwm_corner_rounding` opts every window out of DWM auto-rounding; the CSS paints its own radius. Keep this when adding windows.
- **Qt edition specifics** (`qt/`, full detail in `docs/archive/QT-MIGRATION.md` section 4): all coordinates are logical pixels (Qt 6 defaults to PerMonitorV2) — the tao PhysicalPosition/LogicalSize trap does not apply; the tray icon is `show()`n exactly once per lifetime (repeated hide→show is a known Qt/Windows issue); QSS `border-radius` is only reliable on child containers, so window rounding = translucent top-level + QSS-rounded child container; Qt ships dynamically linked via windeployqt (LGPL compliance) — no static single-exe builds; the same mutex name `KimiPlanbarTray.SingleInstance` is shared by all editions to keep them mutually exclusive; `QSystemTrayIcon` has no hover event, so SPEC 14 hover-to-refresh is implemented as 500 ms polling of `QSystemTrayIcon::geometry()` (cursor inside the icon rect AND moved since last tick = one hover), still 10 s throttled; the SPEC 15.3 hover glow is self-drawn (`windows/hoverglow.h`, a halo child widget of the top-level window) — do NOT use QGraphicsDropShadowEffect (enabling it with blur=0 hides the whole widget); only 225% DPI was tested (100% untested — covered by PerMonitorV2 logical-coordinate semantics).
- **WPF edition specifics** (`wpf/`, revived at 1.7.3): the SPEC 15.3 hover glow is template-layered — each clickable control's `ControlTemplate` stacks a hover-background overlay `Border` + a glow `Border` (carrying the `DropShadowEffect`) over the base background; the Effect must NEVER sit on the content control itself (BlurRadius=0 renders it invisible — the Qt trap), and hover background transitions must animate the overlay's `Opacity`, never a theme brush's `Color` (DynamicResource brushes are shared instances — mutating one bleeds across controls); glow stroke/blur/halo parameters come from the theme dictionaries (`GlowStrokeBrush` / `GlowBlurRadius` / `GlowHaloOpacity`, light 35%/8/0.18 vs dark 45%/10/0.22); the action-row icons are hand-transcribed `StreamGeometry` resources in `Themes/Shared.xaml` with `FillRule="Nonzero"` (SVG default) — token-verified against `rust/index.html` by `scripts/diagnostics/verify_wpf_icon_paths.py`, sized via a 16 px `Viewbox` (NOT `Path.Stretch`, which scales per-icon ink bounds); the hide/show animation guard is a generation counter (`MainWindow._animGen` — show/hide each increment, the pending fade.Completed no-ops when superseded); refresh scheduling carries `_scheduleGen` so an in-flight `SafeRefresh` cannot re-time the timer over a freshly installed 2 s first-refresh tick after a settings save; the manual-refresh debounce lives in `App.ManualRefreshAsync` (UI-thread only); WPF `AllowsTransparency` layered windows get NO DWM auto corner rounding (verified 2026-10-18 via a live screen grab during `--test-ui` — the card corners show only the painted radius) — no `DWMWA_WINDOW_CORNER_PREFERENCE` P/Invoke needed, unlike rust/egui; `--test-ui` shows all four windows for ~6 s (visual-inspection vehicle); settings.json stays PascalCase (`Theme`/`RefreshMinutes`/`AutoStart`) and shared across editions — never change the keys.
- **egui edition specifics** (`egui/`, phases in `docs/archive/EGUI-MIGRATION.md`): viewport transparency requires eframe 0.36.2+ (egui issue #3632 fixed there — do not downgrade); eframe force-shows the root viewport after the first frame, so re-send `ViewportCommand::Visible(false)` on frame 1 (the first frame is fully transparent — no visible flash); `ViewportCommand::OuterPosition` takes LOGICAL points — divide physical work-area targets by the window's pixels_per_point (opposite of tao); `set_fonts` takes effect on the NEXT frame, so CJK coverage is verified a frame later (FontProbe) and named weight families must stay always-bound (an unbound family panics); focus-loss auto-hide needs the `panel_was_focused` latch — egui reports `focused == Some(false)` for a shown-but-never-focused window (the tray menu uses the same `menu_was_focused` latch); `settings.json` is shared across all editions (same schema, same `%APPDATA%\KimiPlanbarTray` path) — never change one edition's keys without the others; eframe runs ONE egui pass per viewport — a secondary window's pass does NOT run `App::ui`, so child-window interactions go through the `WinBus` action queue plus `ctx.request_repaint_of(ViewportId::ROOT)` (plain `request_repaint` only wakes the current viewport); `ViewportInfo.visible` is never written by egui-winit 0.36, which is why `App::ui` still runs while the root window is hidden — if an egui upgrade fixes that, hidden-state command handling must move to `App::logic`; unregistering a deferred viewport destroys its native window, so the settings/skills/menu viewports are re-registered every pass and only ever hidden (singleton reuse); if the root window is ever minimized or fully occluded, eframe runs only `App::logic` (update_logic_only — not implemented here), so the `UiCmd` queue would stall; currently unreachable because all four windows are borderless WS_EX_TOOLWINDOW (never taskbar-minimizable), recorded as a known trap only (the root close guard sits in `App::ui` as well, so a close arriving in that mode would exit for real — same unreachability); the root viewport close guard (app.rs) cancels Alt+F4/WM_CLOSE with `ViewportCommand::CancelClose` + hide (SPEC 20) and stands down only when the app itself raised `App.quitting` first — egui-winit emits the same `ViewportEvent::Close` for its own `ViewportCommand::Close`, so any new Quit path must set `quitting` before sending Close or the guard will swallow it; egui's built-in Ctrl/Cmd+Q quit shortcut and keyboard zoom are both cleared in `App::new` (the shortcut would only re-trigger the cancelled close path, and zoom would descale every `OuterPosition` landing); on Win11 26H2 (build 26300+) DWM draws the windows' retained WS_CAPTION style (winit keeps it for aero-snap) as a full native white title bar + close button over the transparent content, and winit 0.30.13's undecorated-shadow WM_NCCALCSIZE `top+=1` hack leaves a 1px gray strip at every window's top edge (the hairline documented on 2026-10-17) — fixed in `windows.rs::style_hwnd` by (1) `DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED` and (2) subclassing each wndproc to fully collapse the non-client area (client height unchanged — winit also does `bottom+=1`; the egui UI paints its own shadow per SPEC 10, so the DWM shadow the hack preserved was redundant); the hwnd-watcher styles each window as soon as its HWND exists (not on first show, so even the first show is frameless) and re-styles on every hidden→visible transition, because winit's `apply_diff` rewrites GWL_STYLE/GWL_EXSTYLE from its own flag set on every visibility flip (wiping our WS_EX_TOOLWINDOW — without it an unowned visible window could appear on the taskbar); the DWM attributes and the wndproc subclass survive that rewrite — tao's per-DPI client-inset scheme is why rust/ is unaffected; pre-fix diagnosis history: shadow tuning, transparency-settings A/B, a Glow→Wgpu switch, `DWMWA_BORDER_COLOR=COLOR_NONE` (removes the four sides but not the top strip), and WS_CAPTION stripping (kills the line but not via a supported state) were all tried — `docs/archive/EGUI-MIGRATION.md` section 7; eframe 0.36.2 drops a `UserEvent::RequestRepaint` as "outdated" when two passes of the target viewport complete between the repaint callback and the event being processed, and a wake issued right as a pass burst ends can be the last wake source — dropped, it leaves the queued command undrained until an unrelated event arrives (observed as lost tray TogglePanel clicks and a settings X button whose `SettingsClose` never arrived): every cross-thread queue push goes through `app.rs::wake_with_retry`, which re-issues the repaint 120 ms later (egui's repaint-delay gate suppresses the retry when passes are flowing, so it costs nothing in the common case); egui-winit learns the pointer position only from `CursorMoved` and drops all mouse-button input while none is known, but winit keeps `mouse.last_position` across a hide/show cycle — a window re-shown under an unmoved cursor emits no `CursorMoved` and is click-dead until the cursor moves to a NEW position: `windows.rs::spawn_pointer_rearm` posts two synthetic WM_MOUSEMOVEs (the second nudged 1px) on every show (`panel_pos.rs` calls it from all three show paths); known accepted residual (frozen, do not chase): a full native caption can linger for a few seconds on a window's very first show within an instance — seen once across three instance first-shows in the 2026-10-17 flash regression, self-clears, not reproducible on demand (`docs/archive/HANDOFF-egui-26H2-ncframe.md`). On 2026-10-18 the user unfroze the edition once to land the caption-flash hardening (spec + validation record: `docs/EGUI-CAPTION-FLASH-FIX.md`): (a) a thread-level `WH_CBT` hook (`egui/src/windows.rs::install_cbt_hook`, installed on the main thread before `eframe::run_native`) styles each viewport window inside CreateWindowEx by matching the window name against `WINDOW_TITLES` — ANY new window title MUST be added there or it silently loses creation-time styling; if the hook fails to install, `spawn_fast_startup_watcher` (10ms poll for the first 3 seconds) is the spec's A′ fallback; (b) the subclass now strips `WS_CAPTION` permanently (swallows `WM_NCUAHDRAWCAPTION`/`WM_NCUAHDRAWFRAME` 0xAE/0xAF, strips the caption bit again on every `WM_STYLECHANGED` — rewriting the forwarded STYLESTRUCT snapshot caption-free — and keeps the `WM_NCACTIVATE` temporary strip plus a `WM_SHOWWINDOW` DWM re-assert for subclassed windows), so do NOT reason about these windows' styles under the old "winit keeps WS_CAPTION" model: while the subclass is installed the live style has no caption bit, and `DefWindowProc` can never paint a caption from any message path. The egui edition is FROZEN as of 2026-10-17 (user decision), with this single unfreeze on 2026-10-18 for the caption-flash fix: no further maintenance, no feature work, known issues accepted as-is.

## Security considerations

Full policy: `docs/SPEC.md` section 6 and `SECURITY.md`. Highlights: the OAuth token / API key is **read-only** and goes only to `https://api.kimi.com/coding/v1/usages` as a Bearer token — never logged, persisted elsewhere, or sent to any other endpoint; autostart writes HKCU only (no admin); keep the README non-affiliation notice and the `LICENSE` / `NOTICE` attribution intact.

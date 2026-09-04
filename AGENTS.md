# AGENTS.md — Kimi Planbar Tray

Guidance for AI coding agents working in this repository. Read this first; it assumes no prior knowledge of the project.

## Project overview

Kimi Planbar Tray is a lightweight **Windows-only system tray app** that shows Kimi Code plan quota (5-hour window + weekly usage, reset countdowns, "Extra Usage" booster wallet) one click away from the tray. It reads the local Kimi Code CLI OAuth token and calls `GET https://api.kimi.com/coding/v1/usages`.

Current version: **1.7.1** (kept in sync across `rust/package.json`, `rust/src-tauri/Cargo.toml`, `rust/src-tauri/tauri.conf.json`, and `make_release_zip.py`).

This is a **monorepo with two editions**:

- `rust/` — **the only actively developed edition**. Tauri 2 + Rust backend + vanilla HTML/CSS/TypeScript frontend (no framework, no React). New features land here only.
- `wpf/` — original .NET 8 / WPF edition, **frozen at v1.5.0, unmaintained**. Kept as read-only reference for behavior/UI parity. Do not delete it; do not add features to it.

Both editions share the same UI/UX (fully specified in `docs/SPEC.md`) and the same `settings.json` schema.

Other root-level files:

- `docs/SPEC.md` — the single authoritative project spec (in Chinese; English translation at `docs/SPEC_EN.md`, identical chapter numbering): Part 1 (chapters 1-9) covers project scope, architecture, data flow, security, build/release; Part 2 (chapters 10-21) is the behavior/UI detail spec (window sizes, colors, animation timings, API parsing rules). Consult it before changing behavior.
- `docs/*.png` — reference screenshots for visual comparison (regenerate with `make_screenshots.py`).
- `docs/archive/HANDOFF.md` — archived history of the WPF→Rust rewrite (in Chinese); frozen, do not update.
- `make_release_zip.py` — release packaging script (see Release process).
- `make_screenshots.py` — regenerates `docs/screenshot-*.png` via headless Chrome (see Testing / self-checks).
- `verify_icons.py` — byte-compares the inline button SVGs in `rust/index.html` against the source icon library.
- `analyze_wpf_shadow.py`, `dump_tray_windows.ps1`, `inspect_window_dpi.ps1`, `measure_run.ps1` — one-off diagnostic/measurement scripts kept for reference.
- `csp_visual_check.ps1` — shows the tray panel via UIAutomation (locale-independent) and screenshots the bottom-right screen region; used to verify the WebView renders under the CSP in `tauri.conf.json` on a release exe.

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
    ├── Cargo.toml               # tauri 2, reqwest, tokio, serde, windows 0.61, winreg, regex, chrono
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

wpf/                             # UNMAINTAINED reference (net8.0-windows, WPF, zero third-party NuGet deps)
├── MainWindow / SettingsWindow / TrayMenuWindow (.xaml + .xaml.cs)
├── TrayManager.cs, App.xaml.cs
├── Services/{QuotaService,SettingsService,UpdateService,ThemeService}.cs
└── Themes/{Shared,Light,Dark}.xaml
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

WPF edition (reference only):

```bash
cd wpf
dotnet publish -c Release -r win-x64 --self-contained false -p:PublishSingleFile=true -o publish
```

Note: Bash on this machine is Git Bash — use POSIX syntax, forward slashes, single quotes. Python scripts run with `PYTHONUTF8=1` prefix (host is Chinese Windows, code page 936/GBK). Text file I/O in scripts must specify UTF-8 explicitly.

## Testing / self-checks

There is **no unit-test suite** beyond the `skills.rs` frontmatter-parser tests (`cargo test` runs only those; `npm test` runs nothing). Verification is via headless self-check command-line args, which run **before** the single-instance mutex so they work while a GUI instance is live (SPEC section 19):

```bash
kimi-planbar-tray.exe --test-fetch    # fetch quota once, print indented JSON, exit
kimi-planbar-tray.exe --test-update   # one-line: local=... latest=... updateAvailable=... checkFailed=...
kimi-planbar-tray.exe --test-ui       # construct all 4 windows, print "MainWindow OK" etc., exit after ~6 s
```

The Rust edition has **no `--screenshot` arg** (that exists only in the frozen WPF edition); any unrecognized arg falls through to launching the GUI. For visual checks, screenshot the built `rust/dist/index.html` with headless Chrome: strip the `crossorigin` attributes (file:// blocks them), set `data-theme="light|dark"` on `<html>` and `class="enter"` on `<body>` (the panel stays `opacity:0` until the backend emits `panel-show`), and pin `body{width:424px;height:520px;overflow:hidden}` because headless Chrome clamps tiny windows to ~534 px wide.

After Rust changes: `cd rust && cargo build` (in `src-tauri/`) plus `npm run build` to type-check/bundle the frontend, then run `--test-fetch` and `--test-ui` against the built exe. For visual changes, compare against `docs/*.png` screenshots per `docs/SPEC.md`.

## Release process

1. Bump the version in all four places: `rust/package.json`, `rust/src-tauri/Cargo.toml`, `rust/src-tauri/tauri.conf.json`, `make_release_zip.py` (`VERSION` constant) — plus the "Current version" line at the top of this file.
2. Build the Rust release exe (and WPF exes only if the WPF edition was exceptionally touched).
3. Run `python make_release_zip.py` — it zips the full source tree (excluding build outputs) plus the release binaries at the zip root, and regenerates `SHA256SUMS.txt` for the GitHub Release assets.
4. Release zips and `SHA256SUMS.txt` are gitignored; they are uploaded to GitHub Releases manually. Do not commit binaries.
5. Before releasing back to the upstream repo (shawn-0106t/kimi-planbar-tray), confirm with the user whether to open a PR or fork a separate repo.

## Code style and conventions

- **Code, comments, and commit messages are in English**; conversation with the user is in Chinese. Repo docs are mixed: README in English, `docs/SPEC.md` in Chinese (English translation: `docs/SPEC_EN.md`), `docs/archive/HANDOFF.md` in Chinese.
- Rust edition mirrors the WPF reference implementation **1:1** — when behavior is ambiguous, `docs/SPEC.md` is the contract and `wpf/` is the reference source. Comments in the Rust code cite SPEC sections (e.g. `SPEC 16.5`); keep those citations accurate when you change behavior.
- Error-handling baseline: all IO, registry, process, and HTTP failures are **silently swallowed** and surfaced only via UI text ("Update failed", "Not detected") or state fields. Never pop up error dialogs.
- Frontend renders external data (skill names/descriptions) with `textContent` only, never `innerHTML`.
- Tauri IPC: all frontend↔backend interaction goes through the `#[tauri::command]`s registered in `lib.rs::invoke_handlers()` plus events (`panel-show`, `settings-show`, `skills-show`, `update-status`). Windows are singletons — hidden, never destroyed; open paths must re-pin the logical size (DPI guard) and emit the `*-show` event.
- The Skills window is read-only and zero-background-cost: scan once on first open, cache in `AppState`, rescan only when the Refresh button passes `refresh=true`. There is no per-skill enabled/disabled state to display — Kimi Code does not persist one, and `~/.agents/.skill-lock.json` is lark-cli's installer lock file (no `disabled` key); it is not read at all.

## Critical behavioral traps (do not regress)

- **Extra Usage units**: `boosterWallet.balance.amountLeft` is in **1e-8 yuan**; convert to cents with `(raw + 500000) / 1000000`. `priceInCents` fields are already cents. All JSON numbers are modeled as **strings** (tolerate numeric fallback).
- **`isEnabled=false` trap**: when the booster wallet is not enabled, `amountLeft` is an estimate, not a real balance — the whole card must show "Not activated".
- **Credential chain**: `~/.kimi-code/credentials/kimi-code.json` `access_token` (valid only if `expires_at` > now + 30 s) → fallback to an `api_key` in `~/.kimi-code/config.toml` whose `[providers.*]` section has `base_url` containing `api.kimi.com/coding`. `KIMI_CODE_HOME` overrides the home dir.
- **Failure semantics**: on fetch failure keep the last good values on screen and retry after 30 s; on success return to the configured interval (1/5/10/30 min, default 5). First refresh fires 2 s after launch.
- **Single instance**: named mutex `KimiPlanbarTray.SingleInstance` is created in `main.rs` (in addition to `tauri-plugin-single-instance`) so the Rust and WPF editions are mutually exclusive. Do not rename it.
- **DPI rules (Tauri/tao)**: position windows with `PhysicalPosition` (panel against primary work area, menu against the cursor's monitor) but size them with `LogicalSize` — giving physical sizes gets double-scaled on `WM_DPICHANGED`. See SPEC section 20.
- **Portable mode**: an empty `portable.dat` next to the exe redirects `settings.json` to the exe directory instead of `%APPDATA%\KimiPlanbarTray\`.
- **Win11 corners**: `lib.rs::disable_dwm_corner_rounding` opts every window out of DWM auto-rounding; the CSS paints its own radius. Keep this when adding windows.

## Security considerations

- The OAuth token / API key is **read-only** from local Kimi Code CLI files and is sent only to `https://api.kimi.com/coding/v1/usages` as a Bearer token. It must never be logged, persisted elsewhere, or sent to any other endpoint.
- No admin rights: autostart writes only `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run` (key `KimiPlanbarTray`); nothing touches HKLM or Program Files.
- The app is unsigned; SmartScreen warnings are expected and documented in the README.
- Icon assets are the official Kimi logo (copyright Moonshot AI); this is an unofficial community tool — keep the non-affiliation notice in the README. Keep `LICENSE` (MIT © Shawn Qi) and `NOTICE` (portions © baigong-ai / kimi-planbar) attribution intact.

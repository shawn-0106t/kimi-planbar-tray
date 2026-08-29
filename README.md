# Kimi Planbar Tray

[中文](README_CN.md)

A lightweight Windows tray app that keeps your [Kimi Code](https://www.kimi.com/code/) plan quota one click away — 5-hour window and weekly usage, with reset countdowns, right from the system tray.

| Moonlit (light) | Moondark (dark) |
|---|---|
| ![light](docs/screenshot-light.png) | ![dark](docs/screenshot-dark.png) |

## Features

- **Tray resident** — left-click the tray icon to pop up the floating panel (native Windows slide animation); auto-hides on focus loss; right-click menu: Open / Refresh / Settings / Skills / Exit
- **Quota at a glance** — 5-hour and weekly usage cards with progress bars and reset countdowns; data comes from the same endpoint as the CLI's `/usage`
- **Light / dark themes** — "Moonlit" and "Moondark" palettes that follow the Windows system theme in real time (or pin one in Settings); accent color `#1A88FF`
- **Resilient refresh** — auto-refresh on a configurable interval (1/5/10/30 min); on failure the last good values stay visible and a fast retry kicks in after 30 s
- **CLI version check** — shows your local `kimi --version`; an orange badge appears when a newer release exists on [kimi-code Releases](https://github.com/MoonshotAI/kimi-code/releases) (click the row to open the page). Version info comes from the official changelog (with GitHub API fallback), so it works even when GitHub is unreachable
- **Hover-to-fresh** — hovering the tray icon prefetches quota in the background (10s throttle), so the tooltip and popup always show fresh numbers
- **Extra Usage card** — shows your booster wallet balance (¥) and monthly charge usage/limit; gracefully shows "not activated / no data" when the wallet has never been topped up
- **Skills at a glance** (Rust edition) — right-click menu → Skills opens a read-only list of your local Kimi Code skills, grouped by source (`~/.kimi-code/skills`, `~/.agents/skills`, managed plugins); scanned once on open and cached, zero background polling
- **Portable & UAC-free** — single exe, per-user only (HKCU autostart, no admin rights, nothing written to HKLM or Program Files); drop an empty `portable.dat` next to the exe to store settings beside it instead of `%APPDATA%`
- **Small footprint** — ~260 KB single-file build, ~80 MB working set at runtime, no background polling beyond the refresh timer

## Download

> **The WPF edition is no longer maintained** (frozen at v1.5.0) — new features land in the Rust edition only. The `wpf/` source is kept for reference.

Get the latest exe from [Releases](../../releases):

| Build | Size | Requirement | Working set (measured) |
|---|---|---|---|
| `KimiPlanbarTray-rust.exe` | ~5.6 MB | Nothing — uses the system WebView2 | ~317 MB |
| `KimiPlanbarTray-wpf.exe` (unmaintained) | ~260 KB | [.NET 8 Desktop Runtime](https://dotnet.microsoft.com/download/dotnet/8.0) installed | ~69 MB |
| `KimiPlanbarTray-wpf-selfcontained.exe` (unmaintained) | ~65 MB | Nothing — runtime bundled | ~69 MB |

Both editions share the same UI/UX (see `docs/SPEC.md`) and the same settings file.

> Windows SmartScreen may warn on first launch because the exe is not code-signed. Click "More info" → "Run anyway" — this is expected for unsigned personal builds.

## Requirements

- Windows 10 / 11
- [Kimi Code](https://www.kimi.com/code/) CLI installed and signed in, with a **Kimi For Coding** plan (the app reads the CLI's local OAuth token from `~/.kimi-code/credentials/kimi-code.json`, falling back to a plain `api_key` in `~/.kimi-code/config.toml`)
- Network access to `api.kimi.com`

No credentials are stored or sent anywhere except the official `api.kimi.com/coding/v1/usages` endpoint.

## Usage

- **Left-click tray icon** — show/hide the panel
- **Right-click tray icon** — menu: Open / Refresh / Settings / Skills / Exit
- **Hover the tray icon** — quick `5h X% · week Y%` tooltip
- **Settings** — theme (System default / Moonlit / Moondark), refresh interval, launch at login
- **Console button** — opens the [Kimi Code console](https://www.kimi.com/code/console) in your browser
- **CLI version row** — click to open the Releases page

## Build from source

This repo hosts two editions: `wpf/` (original .NET 8 / WPF, **unmaintained since v1.6.0**, kept for reference) and `rust/` (Tauri 2 / Rust rewrite, actively developed). Shared UI/UX spec lives in `docs/SPEC.md`.

WPF edition (unmaintained) — requires .NET 8 SDK (Windows):

```bash
cd wpf
dotnet publish -c Release -r win-x64 --self-contained false -p:PublishSingleFile=true -o publish
# self-contained variant:
dotnet publish -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:EnableCompressionInSingleFile=true -o publish-sc
```

Rust edition — requires Rust (stable, MSVC), Node.js 18+ and the WebView2 Runtime:

```bash
cd rust
npm install
npx tauri build   # single-file exe at src-tauri/target/release/
```

Headless self-checks (useful in CI or after changes):

```bash
KimiPlanbarTray.exe --test-fetch   # fetch quota once, print JSON, exit
KimiPlanbarTray.exe --test-ui      # construct all 4 windows, print OK lines, exit (~6 s)
```

## Tech notes

- Rust edition: Tauri 2 backend + vanilla HTML/CSS/TS frontend (no framework); WPF edition (frozen): .NET 8 / WPF, zero third-party NuGet dependencies
- UI design and layout adapted from [KimiCodeBar](https://github.com/xifandev/KimiCodeBar) (MIT) by [@xifandev](https://github.com/xifandev)
- Quota logic adapted from [kimi-planbar](https://github.com/baigong-ai/kimi-planbar) (MIT) — same token sources, endpoint, and cache/retry strategy
- Tray icon is the official Kimi Code logo embedded as a PNG-compressed ICO; logo copyright belongs to **Moonshot AI** — this is an unofficial community tool, not affiliated with Moonshot AI

## License

[MIT](LICENSE) © 2026 Shawn Qi (shawn-0106t), with portions © baigong-ai (kimi-planbar), © xifandev (KimiCodeBar)

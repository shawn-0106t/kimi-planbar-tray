# Kimi Planbar Tray

[中文](README_CN.md)

A lightweight Windows tray app that keeps your [Kimi Code](https://www.kimi.com/code/) plan quota one click away — 5-hour window and weekly usage, with reset countdowns, right from the system tray.

| Moonlit (light) | Moondark (dark) |
|---|---|
| ![light](docs/screenshot-light.png) | ![dark](docs/screenshot-dark.png) |

## Features

- **Tray resident** — left-click the tray icon to pop up the floating panel (native Windows slide animation); auto-hides on focus loss; right-click menu: Open / Refresh / Settings / Exit
- **Quota at a glance** — 5-hour and weekly usage cards with progress bars and reset countdowns; data comes from the same endpoint as the CLI's `/usage`
- **Light / dark themes** — "Moonlit" and "Moondark" palettes that follow the Windows system theme in real time (or pin one in Settings); accent color `#1A88FF`
- **Resilient refresh** — auto-refresh on a configurable interval (1/5/10/30 min); on failure the last good values stay visible and a fast retry kicks in after 30 s
- **CLI version check** — shows your local `kimi --version`; an orange badge appears when a newer release exists on [kimi-code Releases](https://github.com/MoonshotAI/kimi-code/releases) (click the row to open the page); fails silently when GitHub is unreachable
- **Portable & UAC-free** — single exe, per-user only (HKCU autostart, no admin rights, nothing written to HKLM or Program Files); drop an empty `portable.dat` next to the exe to store settings beside it instead of `%APPDATA%`
- **Small footprint** — ~195 KB single-file build, ~80 MB working set at runtime, no background polling beyond the refresh timer

## Download

Get the latest exe from [Releases](../../releases):

| Build | Size | Requirement |
|---|---|---|
| `KimiPlanbarTray.exe` | ~195 KB | [.NET 8 Desktop Runtime](https://dotnet.microsoft.com/download/dotnet/8.0) installed |
| `KimiPlanbarTray-selfcontained.exe` | ~65 MB | Nothing — runtime bundled |

> Windows SmartScreen may warn on first launch because the exe is not code-signed. Click "More info" → "Run anyway" — this is expected for unsigned personal builds.

## Requirements

- Windows 10 / 11
- [Kimi Code](https://www.kimi.com/code/) CLI installed and signed in, with a **Kimi For Coding** plan (the app reads the CLI's local OAuth token from `~/.kimi-code/credentials/kimi-code.json`, falling back to a plain `api_key` in `~/.kimi-code/config.toml`)
- Network access to `api.kimi.com`

No credentials are stored or sent anywhere except the official `api.kimi.com/coding/v1/usages` endpoint.

## Usage

- **Left-click tray icon** — show/hide the panel
- **Right-click tray icon** — menu: Open / Refresh / Settings / Exit
- **Hover the tray icon** — quick `5h X% · week Y%` tooltip
- **Settings** — theme (follow system / Moonlit / Moondark), refresh interval, launch at login
- **CLI version row** — click to open the Releases page

## Build from source

Requires .NET 8 SDK (Windows):

```bash
dotnet publish -c Release -r win-x64 --self-contained false -p:PublishSingleFile=true -o publish
# self-contained variant:
dotnet publish -c Release -r win-x64 --self-contained true -p:PublishSingleFile=true -p:EnableCompressionInSingleFile=true -o publish-sc
```

Headless self-checks (useful in CI or after changes):

```bash
KimiPlanbarTray.exe --test-fetch   # fetch quota once, print JSON, exit
KimiPlanbarTray.exe --test-ui      # construct both windows, verify resources/XAML
```

## Tech notes

- .NET 8 / WPF, zero third-party NuGet dependencies
- Quota logic adapted from [kimi-planbar](https://github.com/baigong-ai/kimi-planbar) (MIT) — same token sources, endpoint, and cache/retry strategy
- Tray icon is the official Kimi Code logo embedded as a PNG-compressed ICO; logo copyright belongs to **Moonshot AI** — this is an unofficial community tool, not affiliated with Moonshot AI

## License

[MIT](LICENSE) © 2026 Shawn Qi (shawn-0106t), with portions © baigong-ai (kimi-planbar)

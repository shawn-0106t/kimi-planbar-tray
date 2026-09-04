# Kimi Planbar Tray — Project Technical Specification (SPEC)

> This document is the project's **single source of truth**, in two parts:
> - **Part 1 — Project Specification** (chapters 1–9): goals, architecture, data flow, security, build & release, maintenance boundaries
> - **Part 2 — UI & Behavior Specification** (chapters 10–21): every numeric detail of windows, colors, layout, animations, API parsing, persistence, etc. (merged from the original `docs/UI-SPEC.md`; the standalone file has been deleted)
>
> `SPEC x.y` references in code comments point to chapter numbers in this document; before changing any behavior you must consult the corresponding chapter in Part 2, and any behavior change must be reflected in the corresponding chapter.

---

# Part 1 — Project Specification

> English version of this spec. Chinese original: SPEC.md (chapter numbering is identical across both).

## 1. Overview

### 1.1 Goal

A resident Windows system tray app that puts Kimi Code plan quota one click away: usage cards for the 5-hour window and weekly usage, reset countdowns, Extra Usage (booster wallet) balance and monthly usage, plus Kimi Code CLI version update hints, a read-only Skills overview, and browser jumps to Console / Releases.

### 1.2 Feature scope

- Read the local Kimi Code CLI credentials and call `GET https://api.kimi.com/coding/v1/usages` to fetch quota
- Main panel (tray left-click), settings window, tray right-click menu window, read-only Skills window
- Theme follows the system (Moonlit light / Moondark dark), configurable refresh interval, launch at Windows startup (HKCU), portable mode (portable.dat)
- CLI version check (changelog first, GitHub API fallback)
- Headless self-check commands (`--test-fetch` / `--test-update` / `--test-ui`)

### 1.3 Non-goals

- **Unofficial community tool**, not affiliated with Moonshot AI (logo copyright belongs to Moonshot AI)
- Does not modify any Kimi Code CLI files (credentials are read-only)
- No telemetry, no data reporting, no third-party analytics
- Windows 10/11 only; no macOS / Linux
- `wpf/` legacy edition frozen at v1.5.0, no new features

## 2. Terminology

| Term | Meaning |
|---|---|
| Moonlit / Moondark | The light / dark dual themes (color values in chapter 11) |
| Extra Usage / booster wallet | The Extra Usage (booster wallet); see 16.3 for the balance unit trap |
| portable mode | When an empty `portable.dat` exists next to the exe, the settings file is stored in the exe directory instead (see 18.1) |
| `SPEC x.y` | A code comment's reference to a chapter number in this document |

## 3. System Architecture

### 3.1 Repository layout (monorepo, two editions)

- `rust/` — **the only actively developed line**: Tauri 2 + Rust backend + vanilla HTML/CSS/TS frontend (Vite multi-page build, no framework)
- `wpf/` — original .NET 8 / WPF, frozen at v1.5.0, read-only reference, do not delete or modify
- `docs/` — this spec, screenshot baselines, archived history
- Root scripts: `make_release_zip.py` (release packaging), `make_screenshots.py` (README screenshot generation), `verify_icons.py` (byte-compares icons against the library), plus several one-off diagnostic scripts

### 3.2 Process and window model

Single process. All 4 WebView windows are created at startup (hidden) and are all **singletons, reused** (closing hides them, they are never destroyed); there are also a tray icon, the quota polling timer, and a system-theme registry watcher. Single instance is guaranteed by a named mutex (see 20.1).

| Window | Frontend page | Logical size |
|---|---|---|
| main (main panel) | `index.html` + `main.ts` | 424×520 |
| settings | `settings.html` + `settings.ts` | 404×464 |
| skills (overview) | `skills.html` + `skills.ts` | 404×520 |
| menu (tray menu) | `menu.html` + `menu.ts` | 188×160 |

### 3.3 Backend modules (`rust/src-tauri/src/`)

| Module | Responsibility |
|---|---|
| `main.rs` | Entry; `--test-*` self-checks run before the single-instance check; named mutex `KimiPlanbarTray.SingleInstance` |
| `lib.rs` | Tauri builder, all IPC commands, window event routing (collapse on focus loss / hide-on-close), DWM corner rounding disabled |
| `credentials.rs` | Credential chain (credentials json → config.toml fallback), read-only |
| `quota.rs` | usages API fetch + defensive JSON parsing |
| `polling.rs` | Refresh scheduling: first refresh 2 s after launch, 30 s fast retry on failure, back to the normal interval on success, keep last good data |
| `tray.rs` | Tray icon, left-click toggle, right-click menu, tooltip, hover prefetch |
| `panel.rs` | Panel/menu positioning (physical pixels), focus behavior, 300 ms re-entry guard |
| `settings.rs` | settings.json persistence, portable.dat detection, HKCU autostart |
| `skills.rs` | Read-only scan of local skills (scanned once on first open and cached) |
| `update.rs` | `kimi --version` + changelog Range request + GitHub API fallback |
| `theme_watch.rs` | Registry watch for system light/dark theme changes |
| `state.rs` | AppState shared state (RwLock, caches, scheduling notification) |

### 3.4 Frontend/backend boundary (Tauri IPC)

Commands (registered in `lib.rs`, called by the frontend via `invoke`):

| Command | Purpose |
|---|---|
| `get_state` / `refresh_now` | Read full state / refresh now |
| `open_settings` / `close_settings` | Show/hide the settings window |
| `open_skills` / `close_skills` / `get_skills` | Show/hide the Skills window and its data (`refresh=true` forces a rescan) |
| `save_settings` | Save settings and apply (theme/autostart/reschedule refresh) |
| `open_releases` / `open_console` | Open GitHub Releases / Kimi Code Console in the browser |
| `finish_hide_panel` | Actually hide the panel after the collapse animation ends |
| `quit_app` / `menu_action` / `menu_height` | Quit / menu click / menu height auto-fit |
| `start_drag` | Drag a frameless window |

Events (emitted by the backend, frontend `listen`): `quota-updated`, `update-status`, `panel-show`, `panel-hide`, `settings-show`, `skills-show`.

### 3.5 Frontend

All 4 pages share `common.ts` (DTO types + formatting helpers + theme init) and `theme.css` (the single source of all theme colors). External data (skill names/descriptions, etc.) is always rendered with `textContent`; `innerHTML` is forbidden.

## 4. Tech Stack and Key Dependencies

- **Tauri 2** (with opener / single-instance plugins) — windows, tray, IPC
- **tokio / reqwest / serde** — async runtime, HTTP, JSON
- **windows 0.61 / winreg** — Win32 (work area, DPI, DWM, mutex) and registry
- **regex / chrono** — version number parsing, reset countdown calculation
- **Vite + TypeScript** — frontend build; no runtime framework

## 5. Data Flow

```
~/.kimi-code credentials (read-only)
   -> credentials.rs takes the token
   -> quota.rs fetch + parse (traps in 16.3)
   -> AppState (keeps last good values)
   -> emit quota-updated
   -> frontend renders cards / tray tooltip
```

- Settings: frontend → `save_settings` → `settings.rs` writes JSON (path in 18.1) → apply theme/autostart/reschedule timer
- Skills: frontend opens the window → `get_skills` → first open scans the local directories and caches in AppState; never writes back to any file
- Version check: async in the background; the result is pushed to the version row via the `update-status` event

## 6. Security and Privacy

- The OAuth token / api_key is only **read** from local Kimi Code CLI files and is sent only as a Bearer token to `https://api.kimi.com/coding/v1/usages`; it is never logged, never persisted elsewhere, never sent to any other endpoint
- No admin rights needed: autostart only writes `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run` (key `KimiPlanbarTray`); nothing touches HKLM / Program Files
- The exe is not code-signed; SmartScreen warnings are expected (noted in the README)
- The icons are the official Kimi logo (copyright Moonshot AI); keep the `LICENSE` (MIT © Shawn Qi) and `NOTICE` (portions © baigong-ai / kimi-planbar) attributions — they must not be removed

## 7. Build, Test, and Release

### 7.1 Build

Prerequisites: Windows + Rust stable (MSVC) + Node.js 18+ + WebView2 Runtime.

```bash
cd rust
npm install
npm run dev          # frontend only; for the full app use npx tauri dev
npx tauri build      # release exe -> src-tauri/target/release/kimi-planbar-tray.exe
```

Note: a plain `cargo build` debug exe does **not** embed the frontend (its windows point at the Vite devUrl); running it without the dev server shows a WebView2 "localhost ERR_CONNECTION_REFUSED" page, and debug builds come with a console window. The `--test-*` self-checks never load web content, so the debug exe works for them.

### 7.2 Testing

There is no unit-test suite beyond the `skills.rs` frontmatter-parser tests (`cargo test` runs only those). Verification methods (details in chapter 19):

- `--test-fetch` / `--test-update` / `--test-ui` headless self-checks (run before the mutex check, so they can coexist with a running instance)
- Visual verification: `PYTHONUTF8=1 python make_screenshots.py` (headless Chrome renders dist, regenerates `docs/screenshot-*.png`) or compare against the `docs/*.png` baselines
- Before delivery, per the user's global rules, dispatch an independent subagent for code review

### 7.3 Release

1. Bump the version in all four places: `rust/package.json`, `rust/src-tauri/Cargo.toml`, `rust/src-tauri/tauri.conf.json`, and `VERSION` in `make_release_zip.py`
2. `npx tauri build` produces the release exe
3. `python make_release_zip.py` packages a zip of the source snapshot + binaries and generates `SHA256SUMS.txt`
4. The zip and checksums are gitignored; upload them to GitHub Releases manually; **do not commit binaries to the repo**
5. If you plan to release back to the upstream repo (shawn-0106t/kimi-planbar-tray), confirm with the user first whether to open a PR or fork a separate repo

## 8. Runtime Requirements

- Windows 10 / 11 (with WebView2 Runtime, bundled with Win11)
- Kimi Code CLI installed and logged in, on a Kimi For Coding plan
- Network access to `api.kimi.com`

## 9. Maintenance Boundaries and Documentation Map

- New features go only into `rust/`; `wpf/` is frozen as a read-only reference
- When behavior is ambiguous: Part 2 is the contract, `wpf/` is the reference implementation

| Document | Role |
|---|---|
| `README.md` / `README_CN.md` | User-facing: features, download, usage, build |
| `docs/SPEC.md` (the Chinese original of this document) | The single authoritative spec: project-level + UI/behavior details |
| `AGENTS.md` | Onboarding index for AI coding agents (structure, commands, trap summary) |
| `docs/screenshot-*.png` | Visual baselines (generated by `make_screenshots.py`) |
| `docs/archive/HANDOFF.md` | Archived WPF→Rust rewrite handoff manual (historical, no longer updated) |

---

# Part 2 — UI & Behavior Specification

> This part was merged from the original `docs/UI-SPEC.md` (chapter numbers 1–12 → 10–21). All coordinates are in DIP (logical pixels); the DPI awareness mode is system-level (SystemAware).
> The numeric baselines were originally extracted from the WPF reference implementation (`wpf/`, frozen at v1.5.0); the current implementation is `rust/`, and any behavior change must be reflected in this part.

---

## 10. Window Specifications

All three windows are: **frameless** (`WindowStyle=None`), **transparent background + AllowsTransparency** (self-drawn rounded windows), **topmost** (`Topmost=True`), **not shown in the taskbar** (`ShowInTaskbar=False`), **non-resizable** (`ResizeMode=NoResize`). The window's own background is `Transparent`; the actual visible appearance is provided by an inner `Border` (corner radius + background color + shadow), with Margin on all sides as shadow space.

### 10.1 Main usage panel (MainWindow)
- Size: `Width=380, Height=476`
- Root Border: `CornerRadius=14`, `Margin=6` (shadow space), background `WindowBgBrush`
- Shadow: `DropShadowEffect BlurRadius=24, ShadowDepth=2, Opacity=0.25, Color=#000000`
- Content Grid: `Margin=16`, 5 rows (Auto / * / Auto / Auto / Auto)
- Positioning logic (`ShowNearTray`): bottom-right aligned to the work area
  ```
  Left = WorkArea.Right  - Width  - 12
  Top  = WorkArea.Bottom - Height - 12
  ```
  (`WorkArea` = the screen work area excluding the taskbar; i.e. 12 px from the right and bottom screen edges. WPF does not handle taskbars docked on other edges; a port may keep the same behavior.)
- Focus-loss behavior: automatically `HideAnimated()` on `Deactivated`; suppressed via the `_suppressDeactivate` flag while the settings window is open.
- Window reuse: singleton `_popup`; triggering it again while already visible = collapse (toggle).

### 10.2 Settings window (SettingsWindow)
- Size: `Width=360, Height=420`
- Startup position: `CenterScreen`
- Root Border: `CornerRadius=14`, `Margin=6`, background `WindowBgBrush`, same shadow as the main panel (BlurRadius=24, Depth=2, Opacity=0.25)
- Content Grid: `Margin=16`, 2 rows (Auto / *)
- Custom title bar: `DragMove()` on `MouseLeftButtonDown` anywhere on the row makes it draggable
- Behavior: does not auto-close on focus loss; only the ✕ button or "Save" closes it. Singleton, reused.

### 10.3 Tray right-click menu (TrayMenuWindow)
- Size: `Width=150`, height `SizeToContent=Height` (auto-fits 4 items)
- Root Border: `CornerRadius=12`, `Margin=5`, `Padding=4`, background `WindowBgBrush`
- Shadow: `DropShadowEffect BlurRadius=20, ShadowDepth=2, Opacity=0.3, Color=#000000`
- Positioning (`ShowAtCursor`):
  - Take the cursor's physical-pixel coordinates `Cursor.Position`, convert to DIP with `GetDpiForWindow(hwnd)/96.0` (treat `scale<=0` as 1)
  - `Left = min(cursorX_dip, WorkArea.Right - ActualWidth - 8)`
  - Vertically: if `cursorY + ActualHeight + 24 > WorkArea.Bottom` (cursor near the bottom edge), pop upward with `Top = cursorY - h - 8`; otherwise downward with `Top = cursorY + 8`
  - After `Show()`, you must `Activate()` + P/Invoke `SetForegroundWindow(hwnd)` to grab the foreground, otherwise the menu never gets focus and is immediately closed by Deactivated
- Close on focus loss: `Deactivated` → `Dispatcher.BeginInvoke` deferred check `if (IsVisible) Close()` (deferred to avoid a synchronous re-entrant crash from Deactivated during Close)
- Each right-click creates a new instance (close the old menu first, then new)

---

## 11. Color Scheme

### 11.1 Brush key table (ARGB hex, verbatim from source)

| Brush key | Light (Moonlit) | Dark (Moondark) |
|---|---|---|
| `AccentBrush` (accent color / progress bar) | `#FF1A88FF` | `#FF1A88FF` |
| `WindowBgBrush` (window background) | `#FFF3F4F6` | `#FF17191E` |
| `CardBgBrush` (card background) | `#FFFFFFFF` | `#FF23262D` |
| `TextPrimaryBrush` (primary text) | `#FF1F2329` | `#FFF2F3F5` |
| `TextSecondaryBrush` (secondary text) | `#FF6B7280` | `#FF9AA0A8` |
| `ProgressTrackBrush` (progress bar track) | `#FFE5E7EB` | `#FF3A3E47` |
| `ButtonBgBrush` (button background) | `#FFE9ECF0` | `#FF2C3039` |
| `ButtonHoverBrush` (button hover) | `#FFDCE2E9` | `#FF3A404B` |
| `BadgeBgBrush` (new-version badge background) | `#FFFFF0E0` | `#FF3D2E1A` |
| `BadgeFgBrush` (new-version badge text) | `#FFE06D00` | `#FFF0A040` |

- The accent color (Moonshot blue) is the same in both themes: `#1A88FF`.
- The text color of the selected radio pill / checkbox check is fixed to `White`, independent of the theme.
- Tray icon fallback blue ball: `RGB(0x1A, 0x88, 0xFF)` + highlight `rgba(255,255,255,90/255)`.

### 11.2 Progress bar colors
- **No usage-range color-change logic**: the progress bar fill is always `AccentBrush` (`#1A88FF`), the track always `ProgressTrackBrush`. There is no code that switches green/yellow/red by percentage.
- The fill width is implemented with two columns `GridLength(p, Star)` / `GridLength(100-p, Star)`, where `p = clamp(percent, 0, 100)`.

---

## 12. Panel UI Structure (MainWindow)

Overall: `Grid Margin=16`, 5 rows. All copy is in English (terminology aligned with the Kimi console: Weekly usage / 5-hour usage / Extra Usage).

### 12.1 Header (Row 0)
- `DockPanel Margin=2,0,2,16`
- Left: logo image `kimi-logo.png`, `20x20`
- Title: `"Kimi Planbar Tray"`, `FontSize=17`, `FontWeight=SemiBold`, `TextPrimaryBrush`, `Margin=10,0,0,0`, vertically centered
- Right side (Dock Right): `LastUpdated`, `FontSize=11`, `TextSecondaryBrush`, `Margin=12,0,0,0`
  - No data: empty string
  - Error: `"Update failed"`
  - Normal: `"Updated HH:mm"` (`FetchedAt` local time, 24-hour clock)

### 12.2 Usage card area (Row 1)
- `UniformGrid Columns=2`, two cards: left card `Margin=0,0,6,0`, right card `Margin=6,0,0,0` (12 px between cards)
- Card style: `CardBgBrush`, `CornerRadius=12`, `Padding=16`
- Left card "Weekly usage" (week) / right card "5-hour usage" (5h), same structure:
  - Title: `"Weekly usage"` / `"5-hour usage"`, `FontSize=13`, `TextSecondaryBrush`
  - Big percentage: default `"--"`, `FontSize=32`, `FontWeight=Bold`, `Margin=0,10,0,10`, `TextPrimaryBrush`; after data arrives, `$"{Percent:0}%"` (the display uses the raw, unclamped Percent; the progress bar uses the clamped value)
  - Progress bar: `Grid Height=6`, two columns (fill column starts at `0*` / remainder column starts at `100*`); bottom layer `Border CornerRadius=3` spanning both columns with `ProgressTrackBrush`, top layer fill `Border CornerRadius=3 AccentBrush` (a capsule bar 6 px high with radius 3)
  - Reset countdown: `FontSize=11`, `Margin=0,12,0,0`, `TextSecondaryBrush`

### 12.3 Reset countdown format (`FormatReset`, `span = at - now`)
- `span < 0`: `"Resets soon"`
- `>= 1 day`: `"Resets in {int(TotalDays)}d {Hours}h"` (e.g. `Resets in 4d 3h`)
- `>= 1 hour`: `"Resets in {int(TotalHours)}h {Minutes}m"`
- `< 1 hour`: `"Resets in {max(1, Minutes)}m"` (shows at least 1 minute)
- No `resetTime`: empty string

### 12.4 Extra Usage card (Row 2)
- `Margin=0,12,0,0`, `Padding=16,12`, `CornerRadius=12`, `CardBgBrush`
- First row DockPanel: left `"Extra Usage"` (`FontSize=13`, `TextSecondaryBrush`), right `ExtraBalance` (default `"--"`, `FontSize=18`, `FontWeight=Bold`, `TextPrimaryBrush`, right-aligned)
- The balance text has three states (`ExtraState`):
  - `Ready`: shows `FmtYuan` if `BalanceCents` exists (see 12.5), otherwise `"--"`
  - `NoData`: `"No data"`
  - Otherwise (`NotActivated`): `"Not activated"`
- Monthly sub-panel `ExtraMonthlyPanel` (`Margin=0,8,0,0`, default `Collapsed`): shown only when `MonthlyEnabled && MonthlyLimitCents > 0 && MonthlyUsedCents.HasValue`
  - Same style of progress bar (height 6, radius 3), `p = clamp(used/limit*100, 0, 100)`
  - Text: `"Used {FmtYuan(used)} this month / {FmtYuan(limit)} limit"`, `FontSize=11`, `Margin=0,8,0,0`, `TextSecondaryBrush`

### 12.5 Currency formatting (`FmtYuan`, unit: cents → yuan)
- Negative: `"-" + FmtYuan(-cents)`
- `¥{cents/100}`, appending `.{frac:00}` when the remainder > 0; decimals omitted for whole yuan. E.g. `1234 → "¥12.34"`, `10000 → "¥100"`

### 12.6 Version row (Row 3)
- The whole row is a clickable card: `Margin=0,12,0,12`, `Padding=14,10`, `CornerRadius=12`, `Cursor=Hand`, `ToolTip="View Kimi Code releases"`
- Clicking opens the browser: `https://github.com/MoonshotAI/kimi-code/releases` (exceptions silently swallowed)
- DockPanel: left `"Kimi Code CLI"` (`FontSize=13`, `TextPrimaryBrush`); right side arranged horizontally:
  - `CliVersion`: default `"--"`, shows the local version number or `"Not detected"`, `FontSize=13`, `TextSecondaryBrush`
  - New-version badge `NewVersionBadge`: default `Collapsed`; `CornerRadius=8`, `Padding=8,2`, `Margin=8,0,0,0`, background `BadgeBgBrush`, text `"Update available"` `FontSize=11` `BadgeFgBrush`

### 12.7 Bottom buttons (Row 4)
- 4 buttons evenly split one row, styled as `ActionButton` (see 13.3) but **with the background changed to `CardBgBrush`** (same color as the cards, to reduce the visual weight of the bottom buttons and emphasize the info area above; hover remains `ButtonHoverBrush`); each button is a two-line layout with **icon on top, text below**: 16 px icon (inline SVG, fill style, 24×24 grid, `currentColor` follows the theme text color), 2 px gap between icon and text, button `Padding=0,5,0,6`, text `FontSize=12`
- Icon sources: Console=`Browser`, Refresh=`Refresh`, Settings=`Setting` (all from the kimi-widget icon library); Exit=hand-drawn power glyph (a ring with an opening at the top + a rounded vertical bar on top, same fill style)
  - `"Console"` `Margin=0,0,6,0` → opens the browser at `https://www.kimi.com/code/console?from=kfc_overview_topbar` (exceptions silently swallowed), `ToolTip="Open Kimi Code Console"`
  - `"Refresh"` `Margin=3,0` → `SafeRefresh()` + `CheckAsync()`
  - `"Settings"` `Margin=3,0` → opens the settings window (focus-loss collapse is suppressed while it is open)
  - `"Exit"` `Margin=6,0,0,0` → `Application.Current.Shutdown()`
- The Rust edition debounces manual refreshes by **2 seconds**: both the panel Refresh button and the tray menu "Refresh" go through `refresh_now`, and repeated triggers less than 2 s after the last manual refresh are silently ignored.

---

## 13. Settings Window UI Structure (SettingsWindow)

### 13.1 Title bar (Row 0)
- `DockPanel Margin=2,0,2,16`, the whole row is draggable
- logo `18x18` + title `"Kimi Planbar Tray Settings"` (`FontSize=15`, `SemiBold`, `TextPrimaryBrush`, `Margin=10,0,0,0`)
- Close button `"✕"` on the right, styled as `ChromeCloseButton`

### 13.2 Settings items (Row 1, `StackPanel Margin=6,0,4,0`)
| Setting | Control | Options | Default |
|---|---|---|---|
| "Theme" subtitle (`FontSize=13 SemiBold TextSecondaryBrush`) | — | — | — |
| Theme radio | `RadioButton` x3, `GroupName="theme"`, style `ThemeRadio`; spacing `Margin=0,10,0,0` / `0,8,0,0` / `0,8,0,0` | `"System default"`=system, `"Moonlit (light)"`=light, `"Moondark (dark)"`=dark | `"system"` |
| "Refresh interval" subtitle (`Margin=0,20,0,0`) | — | — | — |
| Refresh interval | horizontal `StackPanel Margin=0,10,0,0`, `RadioButton` x4, `GroupName="interval"`, style `PillRadio`, first three `Margin=0,0,6,0` | `"1 min"`(Tag=1), `"5 min"`(Tag=5), `"10 min"`(Tag=10), `"30 min"`(Tag=30) | `5` (in XAML, 5 min has `IsChecked=True`) |
| Launch at startup | `CheckBox "Launch at Windows startup"`, style `ThemeCheckBox`, `Margin=0,22,0,0` | bool | `false` |
| Save | `Button "Save"`, style `ActionButton`, `Margin=0,26,0,0` | — | — |

- Save action: write `settings.json` → `ApplyAutoStart()` → `Theme.Apply(theme)` → `Quota.Reschedule()` → `Close()`.
- On open, the window back-fills the checked states from the current settings; `RefreshMinutes` is matched via the Tag string.

### 13.3 Shared control styles (`Themes/Shared.xaml`)
- **ActionButton**: `Padding=0,10`, `FontSize=13`, foreground `TextPrimaryBrush`, background `ButtonBgBrush`, no border, hand cursor; template `Border CornerRadius=10`, hover background `ButtonHoverBrush`
- **ThemeRadio**: `FontSize=13`, `Cursor=Hand`; `18x18` circular outline (`Stroke=TextSecondaryBrush, StrokeThickness=1.5`, transparent fill); when selected, an inner `8x8` dot is shown (`Margin=3`, `Fill=AccentBrush`); text offset from the circle `Margin=8,0,0,0`; text turns `AccentBrush` on hover
- **PillRadio** (segmented pill): `FontSize=12`, `Padding=12,6`, template `Border CornerRadius=9`; unselected: background `ButtonBgBrush`, text `TextPrimaryBrush`; selected: background `AccentBrush`, text `White`; hover background `ButtonHoverBrush` (selected + hover stays `AccentBrush`)
- **ThemeCheckBox**: `FontSize=13`; `18x18` square `CornerRadius=5`, border `TextSecondaryBrush 1.5`, transparent background; when selected, background and border become `AccentBrush` and a white `"✓"` is shown (`FontSize=12 Bold`); text offset from the box `Margin=8,0,0,0`
- **ChromeCloseButton**: `Width=28`, `Padding=8,2`, `FontSize=14`, right-aligned, foreground `TextSecondaryBrush`, transparent background, template `Border CornerRadius=6`; hover: background `ButtonHoverBrush`, text `TextPrimaryBrush`
- **MenuItemButton** (tray menu item): `Padding=14,9`, `FontSize=13`, `TextPrimaryBrush`, transparent background, left-aligned, template `Border CornerRadius=8`; hover background `ButtonHoverBrush`

---

## 14. Tray Behavior (TrayManager)

- Implementation: `System.Windows.Forms.NotifyIcon`; the icon is static and never changes, refreshes only update the tooltip text.
- **Hover (MouseMove) → hover-to-refresh**: throttled to **10 seconds** (skipped if the last hover refresh was <10 s ago), triggers `Quota.SafeRefresh()` (async, not awaited).
- **Left click (MouseUp, Left)**: toggles the main panel. Re-entry guard: left clicks within **300 milliseconds** after the panel was auto-hidden by focus loss are ignored (the `_lastHide` check, so the same click does not trigger focus-loss hide and then immediately pop back). Panel already visible → `HideAnimated()`; not visible → singleton reuse `ShowNearTray()`.
- **Right click (MouseUp, Right)**: closes the old menu instance, creates a new `TrayMenuWindow`, and `ShowAtCursor()`.
  - Menu items: "Open" → close menu + `TogglePopup()`; "Refresh" → close menu + `SafeRefresh()` + `CheckAsync()`; "Settings" → close menu + `ShowSettings()`; "Exit" → `Shutdown()`.
  - Uses `MouseUp` instead of `MouseClick` (more reliable for right-click without a ContextMenuStrip).
- **Tooltip (NotifyIcon.Text)**:
  - No data: `"Kimi Planbar Tray"`
  - With data: `"Kimi Planbar Tray  5h {x}% · week {y}%"` (separated by two spaces; a missing segment shows `"?"`; `Percent:0` format)
  - On error, appends `" (update failed)"` at the end; on failure the retained last-good data is still shown
- Icon loading: prefers the embedded `kimi-logo.png` manually wrapped as an ICO (ICONDIR: reserved=0, type=1, count=1; entry width=0/height=0 meaning 256, 32bpp, planes=1, payload offset=22), preserving alpha; on missing resource or exception, falls back to a programmatically drawn 32x32 blue ball (circle `(1,1,30,30)` filled `#1A88FF`, highlight ellipse `(7,5,10,7)` filled `rgba(255,255,255,90)`).

---

## 15. Animations

### 15.1 Panel slide-in (`ShowNearTray`)
- The comment in the code is explicit: `AnimateWindow` is unreliable on AllowsTransparency layered windows, so WPF animations (GPU-composited) are used
- Initial state: `RootBorder.RenderTransform = TranslateTransform(0, 16)`, `Window.Opacity = 0`
- Fade in: `Opacity 0 → 1`, **160 ms**, linear
- Slide in: `TranslateTransform.Y 16 → 0` (16 px upward from below), **220 ms**, `CubicEase EaseOut`
- Both animations start at the same time

### 15.2 Panel slide-out (`HideAnimated`)
- Re-entry guard flag `_hiding`
- Fade out: `Opacity → 0`, **130 ms**, linear
- Slide out: `TranslateTransform.Y → 12` (12 px downward), **160 ms**, `CubicEase EaseIn`
- After the fade-out completes: `Hide()`, `Opacity = 1` (reset), notify `Tray.NotifyPopupHidden()` (records `_lastHide` for the 300 ms re-entry guard)

### 15.3 Others
- **Hover glow** (Rust edition): clickable controls (ActionButton, tray menu items, the version-row card, the Skills Refresh mini-button) fade in an accent-colored outer glow on hover — a 1 px accent ring at 35% alpha plus an 8 px halo at 18% alpha (dark theme: 45% / 22%, 10 px), with the background-color change transitioned in sync; all **140 ms ease**. The glow is pre-painted on an `::after` pseudo-element and only its `opacity` is transitioned (the box-shadow itself never animates — compositor-only, no per-frame repaint).
- All other controls (theme radios, interval pills, the checkbox, the title-bar ✕) switch their hover state instantly. Progress bar width changes are not animated (the width is set directly).

---

## 16. Data and API (QuotaService)

### 16.1 Request
- URL: `GET https://api.kimi.com/coding/v1/usages`
- Headers: `Authorization: Bearer {token}`, `Accept: application/json`
- HTTP timeout **10 seconds**; non-2xx → failure path

### 16.2 Credential read priority chain (`LoadToken`, aligned with quota-status.py)
- `<kimi_home>` defaults to `%USERPROFILE%/.kimi-code/` and can be overridden by the `KIMI_CODE_HOME` environment variable (same convention as 21.2).
1. **`<kimi_home>/credentials/kimi-code.json`**:
   - Read `access_token` (string)
   - Validate `expires_at` (Unix seconds, number) > current UTC time + **30 seconds** margin; if expired, treat it as invalid and continue to the next step
   - Parse exceptions are silently swallowed
2. **Fallback `<kimi_home>/config.toml`** (hand-written line-by-line parsing, not a full TOML parser):
   - Split into `[section]`s; extract key-values with the regex `^(base_url|api_key)\s*=\s*"([^"]*)"`
   - Match condition (`MatchProvider`): the section name starts with `"providers."` **and** `base_url` contains `"api.kimi.com/coding"` **and** `api_key` is non-empty → return that `api_key`
   - When a new section starts, settle the previous section first; settle the last section at end of file
3. Neither found → return `QuotaResult{ Error = "no-token" }`

### 16.3 Response JSON parsing
- **5-hour segment**: `root.limits` (array; take the `detail` object of element 0) → `ParseSegment`
- **Weekly segment**: `root.usage` (object) → `ParseSegment`
- `ParseSegment`: `percent = used/limit*100` (`used`, `limit` tolerate numbers or numeric strings, missing treated as 0; `limit<=0` treated as 1 to avoid division by zero); `resetTime` (string, `DateTimeOffset.TryParse`) → `ResetAt`
- **Extra Usage**: `root.boosterWallet` (object):
  - Not an object / missing → `State = NotActivated` ("Not activated")
  - `isEnabled == false` → `NotActivated` (defense: when the booster is not enabled, `amountLeft` is an estimate of "monthly limit minus used" rather than a real balance, and must be treated as not activated — borrowed from the KimiCodeBar v1.1.1 bug)
  - `balance.amountLeft` (numeric string, numeric type tolerated) parseable → `State = Ready`; the unit is **1e-8 yuan**, converted to cents: `BalanceCents = (raw + 500000) / 1000000` (with rounding)
  - Otherwise → `State = NoData` ("No data")
  - `monthlyChargeLimitEnabled == true` → `MonthlyEnabled=true`, `monthlyUsed.priceInCents` → `MonthlyUsedCents`, `monthlyChargeLimit.priceInCents` → `MonthlyLimitCents` (unit cents, numeric strings)
- **Note: all JSON numbers from the server are modeled as strings**, with numeric fallback tolerated during parsing.

### 16.4 Data model (equivalent Rust struct)
```
QuotaSegment { percent: f64, reset_at: Option<DateTimeOffset> }
ExtraState   { NotActivated, NoData, Ready }
ExtraInfo    { state, balance_cents: Option<i64>, monthly_enabled: bool,
               monthly_used_cents: Option<i64>, monthly_limit_cents: Option<i64> }
QuotaResult  { five_hour: Option<QuotaSegment>, week: Option<QuotaSegment>,
               extra: Option<ExtraInfo>, fetched_at: DateTimeOffset, error: Option<String> }
```
(On error, `Error` = exception type name, e.g. `"HttpRequestException"` / `"TaskCanceledException"` / `"no-token"`)

### 16.5 Refresh scheduling and failure retry (`SafeRefresh` / `Reschedule`)
- `Reschedule()`: period = `max(1, RefreshMinutes)` minutes; the timer's first delay is **2 seconds** (first refresh 2 s after launch), then it follows the period
- `SafeRefresh()`:
  1. `FetchAsync()`
  2. **Keep last good data on failure**: if `r.Error != null && Last != null`, use `Last` to fill the null fields of `FiveHour`/`Week`/`Extra` (the UI is not cleared; only the status row shows it)
  3. **Fast retry 30 seconds after failure**: the timer's next fire becomes `Error != null ? 30_000ms : periodMs`; the period itself is unchanged (on success it returns to the normal period); race exceptions from an already-Disposed timer are swallowed
  4. Raise the `Updated` event (on the UI thread)

---

## 17. CLI Version Check (UpdateService)

### 17.1 Local version (`DetectLocalVersion`)
- Spawn a subprocess: `kimi --version` (`UseShellExecute=false, CreateNoWindow=true`, stdout/stderr both redirected)
- **5000 ms** timeout waiting for exit; on timeout `Kill()` and return null
- `WaitForExit` first, then read the output (the output is a single line and will not fill the pipe buffer)
- Regex the combined stdout+stderr text for the first `\d+\.\d+\.\d+`
- Any exception → null (the panel shows `"Not detected"`)

### 17.2 Latest version (two-level fallback)
1. **Official docs site changelog** (preferred; the English version is the most up to date; bypasses GitHub API rate limiting and hosts blocking):
   - `GET https://moonshotai.github.io/kimi-code/en/release-notes/changelog.md`
   - Request header `Range: bytes=0-4095` (only fetch the first 4 KB; GitHub Pages may ignore Range and return a full 200 — both response shapes are handled)
   - Regex `^## (\d+\.\d+\.\d+)` (Multiline); the first match is the latest version
2. **GitHub Releases API fallback**:
   - `GET https://api.github.com/repos/MoonshotAI/kimi-code/releases/latest`
   - Header `User-Agent: KimiPlanbarTray` (required, otherwise GitHub rejects the request)
   - Take `tag_name` (e.g. `"@moonshot-ai/kimi-code@0.31.1"`), extract `\d+\.\d+\.\d+` with regex
- HTTP timeout 10 seconds; if both fail → `LatestVersion = null`

### 17.3 Comparison and status
- `UpdateAvailable = latest != null && both parse as semantic versions && latest > local`
- `CheckFailed = latest == null` (silently degrades when the network is unreachable; the UI shows nothing)
- Raises the `Updated` event when done
- Trigger points: in the background at startup; the panel "Refresh" button and the tray menu "Refresh" trigger it too

### 17.4 UI presentation
- The version row shows `LocalVersion ?? "Not detected"`
- When `UpdateAvailable == true`, show the orange badge "Update available" (colors: BadgeBg/BadgeFg in 11.1)
- Clicking anywhere on the row jumps to `https://github.com/MoonshotAI/kimi-code/releases`

---

## 18. Settings Persistence (SettingsService)

### 18.1 Config file path (portable mode logic)
- **Portable mode**: a `portable.dat` file exists next to the exe (any content; only existence is checked) → config directory = the exe's directory
- **Otherwise**: `%APPDATA%\KimiPlanbarTray\`
- Config file: `<ConfigDir>\settings.json`

### 18.2 JSON schema (`SettingsData`, serialized with indentation)
```json
{
  "Theme": "system",
  "RefreshMinutes": 5,
  "AutoStart": false
}
```
- `Theme`: `"system" | "light" | "dark"`, default `"system"`
- `RefreshMinutes`: int, allowed values 1/5/10/30, default 5
- `AutoStart`: bool, default false
- Load: file missing or deserialization fails → fall back to all defaults (exceptions silently swallowed)
- Save: `CreateDirectory` first, then overwrite the file as a whole (exceptions silently swallowed)

### 18.3 Launch at Windows startup (`ApplyAutoStart`)
- Registry: `HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run` (per-user, does not trigger UAC)
- Key name: `KimiPlanbarTray`
- `AutoStart=true` → value = `"{full exe path}"` (quoted)
- `AutoStart=false` → delete the value (no error if it does not exist)
- Exceptions silently swallowed

---

## 19. Test/Self-Check Commands (App.OnStartup command-line args)

All self-check modes run **before the single-instance mutex check** (so they work even while a GUI instance is running), print to stdout, and exit. Subprocess calls are moved onto the thread pool, off the UI thread, to avoid deadlock.

| Arg | Behavior | Output |
|---|---|---|
| `--test-fetch` | Fetch quota once, print JSON | JSON of `QuotaResult` (indented, non-ASCII not escaped) |
| `--test-update` | Run one version check | Single line: `local={x} latest={y} updateAvailable={bool} checkFailed={bool}` |
| `--test-ui` | After applying the current theme, construct each window in turn to verify resource resolution and page loading | One line per window: `MainWindow OK` / `SettingsWindow OK` / `TrayMenuWindow OK` (the Rust edition additionally prints `SkillsWindow OK`); on exception `UI-FAIL: {exception type}: {message}` + InnerException message |
| `--screenshot <path> [--dark] [--mock]` (WPF edition only) | Render the main panel as PNG with real (or mocked) quota data + the specified theme (for the README) | `saved: <path>` |

- `--screenshot` (WPF edition only; the Rust edition has no such arg — see 7.2 for screenshot verification): light theme by default, `--dark` switches to dark; `--mock` injects fixed data (5h=42% resetting in 3.5 hours, week=68% resetting in 4 days, Extra: balance 1234 cents, monthly used 4567 / limit 10000 cents); otherwise it fetches real data. Renders as a 96 DPI Pbgra32 PNG and auto-creates the output directory.

---

## 20. Other Implementation Details

- **Single instance**: named mutex `"KimiPlanbarTray.SingleInstance"`; exits immediately if the mutex is not acquired; released on exit (exceptions swallowed). Closing windows does not exit the process (`ShutdownMode=OnExplicitShutdown`).
- **Startup order**: load settings → construct Theme/Quota/Update services → (self-check branch) → single-instance check → apply theme → hook the system theme event → create the tray icon → start auto-refresh (first refresh after 2 s) → background version check.
- **Focus-loss collapse**: only on the main panel; suppressed with `_suppressDeactivate` while the settings window is open (the settings window itself does not collapse on focus loss).
- **Real-time system theme following**: read the registry value `AppsUseLightTheme` under `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` (DWORD, 0=dark, 1=light, default 1 if missing); listen via the `SystemEvents.UserPreferenceChanged` event, re-applying only when the setting is `"system"` (switch back to the UI thread). A Tauri port can use the `dark-light` crate or read the registry itself + listen for `WM_SETTINGCHANGE`.
- **Theme switching implementation**: clear the app-level resource dictionary, then add Shared + Light/Dark in order. All colors go through dynamic resource references, so switching takes effect immediately without rebuilding windows. The Shared style dictionary is loaded only once.
- **DPI**: locked to system-level DPI awareness (SystemAware). Tray menu positioning relies on `Cursor.Position` (physical pixels) ÷ `GetDpiForWindow/96` converted to DIP; without this conversion the menu would be positioned off-screen on high-DPI displays.
- **DPI trap (Tauri/tao port)**: tao creates all HWNDs (hidden) at app startup with position `CW_USEDEFAULT` — Windows places new windows on the **launcher's screen** (e.g. double-clicking the exe from an Explorer window on a secondary monitor puts the hidden windows on that monitor, carrying that monitor's scale). If you later use `LogicalPosition`/`LogicalSize` when positioning across screens, the conversion uses the scale of the screen the window is currently on rather than the target screen, so with mixed-DPI multi-monitor setups the first open is guaranteed to be off. **Rule: always use `PhysicalPosition` for cross-screen placement (the panel against the primary screen's work area in physical pixels, the menu against the cursor's screen in physical pixels); keep sizes in `LogicalSize`** — on `WM_DPICHANGED`, tao recomputes the physical size as logical size × the new scale, so physical sizes would be double-scaled. The WPF edition has no such problem: a `Window`'s HWND is created at `Show()` time, after `Left`/`Top` have already been set, so conversion naturally uses the target screen's DPI.
- **Icon resources**: `kimi-logo.png` embedded in the assembly, used for: the panel logo (20x20), the settings window logo (18x18), and the tray icon (manual PNG→ICO wrapping, natively supported on Vista+ and preserves alpha). A Tauri port needs to embed the same PNG and provide an ICO (pregenerate it with the `ico` crate, or embed the PNG directly into an ICO container — the same approach as the reference implementation).
- **Event subscription lifecycle**: the main panel subscribes to `Quota.Updated`/`Updates.Updated` at construction and unsubscribes on close (the window is actually only hidden, never closed — singleton reuse); the tray subscribes to `Quota.Updated` to update the tooltip, and unsubscribes and destroys the tray icon on exit.
- **Exception handling baseline**: all IO, registry, external process, and HTTP calls are wrapped in try/catch and silently swallowed; failure paths are expressed via UI text ("Update failed"/"Not detected") or state fields — never pop up an error dialog.
- **Currency unit trap**: `amountLeft` is in 1e-8 yuan (convert to cents with `(raw + 500000) / 1000000`, with rounding); `priceInCents` is already cents; all JSON numbers are strings.
- **`isEnabled=false` trap**: when the booster is not enabled, `amountLeft` is not a real balance; the whole card must be judged as "Not activated".
- **Exit cleanup**: hide and destroy the tray icon → stop the refresh timer → release the single-instance mutex.

---

## 21. Skills Read-Only Window (Rust edition only, new in v1.6)

Borrowed from the `/api/skills` idea of [kimi-code-dashboard](https://github.com/perinchiang/kimi-code-dashboard), trimmed down to purely read-only display.

### 21.1 Window

- Fully reuses the settings-window paradigm: frameless transparent, `margin: 28px` shadow space, custom title bar (logo 18×18 + title "Kimi Skills" + ✕ close, draggable), centered, topmost, skip taskbar, non-resizable.
- Size: 404×520 (visible card area 348×464).
- Singleton reuse: closing only hides; on open, pin the size (same DPI guard as show_panel) and emit `skills-show` so the frontend back-fills.
- While open, `skills_open` is set, which — like `settings_open` — suppresses the main panel's focus-loss auto-hide.

### 21.2 Data source and performance

- Three root directories: `<kimi_home>/skills` (label "Kimi Code"), `~/.agents/skills` ("Agents"), `<kimi_home>/plugins/managed/<plugin>/skills` ("Plugin: <name>"); `<kimi_home>` can be overridden by `KIMI_CODE_HOME`.
- For each `<dir>/<id>/SKILL.md`, only the first 4 KiB are read to parse the YAML frontmatter's `name` / `description` (hand-written line parsing, stripping leading/trailing quotes, falling back to the directory name when absent; no YAML dependency). After reading the bytes, decode with `from_utf8_lossy`, tolerating multi-byte characters cut at the 4 KiB boundary and mixed GBK bytes, and strip the UTF-8 BOM before `---`.
- The frontend is purely event-driven: the skills page does **not** load data at startup (while the window is hidden); scanning is triggered entirely by the `skills-show` event emitted by `open_skills` (the listener is registered before `initTheme` to prevent a race).
- There is no enabled/disabled state to display: Kimi Code does not persist a per-skill disabled state (nothing related is found in the kimi.exe binary); `~/.agents/.skill-lock.json` is lark-cli's installer lock file (`{version, skills, dismissed}`, no `disabled` key) and is never read.
- **Zero background cost**: no polling, no file watching; scans once on first window open and caches into `AppState`; `get_skills(refresh=false)` returns the cache directly; the Refresh button on the window passes `refresh=true` to force a rescan.

### 21.3 Presentation

- Top summary row: `N skills` + Refresh button.
- The list is grouped by source (sorted case-insensitively by name within each group) and scrollable; each item is a card: name (SemiBold, single-line ellipsis) + description (12 px, 2-line clamp, full description in the card tooltip).
- All colors come from `theme.css` variables, automatically following Moonlit/Moondark.
- The frontend renders everything with `textContent` (skill descriptions are external input; no innerHTML).
- Entry point: a new "Skills" item in the tray right-click menu (between Settings and Exit); the menu height is auto-reported by the frontend content, no change to the positioning logic.

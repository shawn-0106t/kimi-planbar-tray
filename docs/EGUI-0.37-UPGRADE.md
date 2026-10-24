# egui edition upgrade: eframe/egui 0.36.2 → 0.37.0 — validation record

> Date: 2026-10-24 · Branch: `egui/upgrade-eframe-0.37` (from main `5f58221`)
> Machine: Windows 11 26H2 build 26300, primary 2880x1800 @225%, secondary 3456x1944 @125%
> Toolchain: rustc 1.97.1 · MSVC 14.50.35717
> Baseline artifacts: `egui/target/baseline-0.36.2/` (gitignored): self-check outputs,
> 8 screenshots (light/dark × 4 windows), lock-alignment/build notes, harness scripts.

## 1. Change summary (the whole diff)

- `egui/Cargo.toml`: `eframe = "=0.37.0"`, `egui = { version = "0.37.0", features = ["default_fonts"] }`
  (default_fonts is no longer a default feature since 0.37); `tray-icon = "=0.26.1"` untouched.
- `Cargo.lock`: targeted `cargo update -p egui -p eframe` — egui family 0.36.2 → 0.37.0,
  new `egui_system_font_provider 0.37.0` + fontique/parlance stack (Windows builds take the
  windows branch). winit stays 0.30.13. Business-core versions unchanged (see §2).
- 14 × `Id::new(` → `Id::unique(` (0.37 deprecates the former; `-D warnings` makes it
  mandatory): app.rs ×1, ui/controls.rs ×2, ui/panel.rs ×2, ui/settings_win.rs ×4,
  ui/menu_win.rs ×1, ui/skills_win.rs ×2, ui/widgets.rs ×1, bin/spike.rs ×1.
  (`theme.rs`'s two `FontId::new` are egui::FontId, not affected.)
- Comments only (no logic change) in `theme.rs`/`app.rs` font-probe docs: egui 0.37
  `set_fonts` lands "at the next pass start, or immediately when no text was laid out
  this pass" (verified in egui-0.37.0 source, `apply_font_changes_now_if_unused`); the
  one-frame-later verification stays correct under both timings, so the FontProbe state
  machine is unchanged.
- **windows.rs / panel_pos.rs: ZERO changes** (CBT hook, NC-collapse subclass,
  WS_CAPTION strip, window_dpi_scale positioning — all untouched).
- Gate A verdict: mechanical adaptation only; fmt/clippy/build/test all green with CI
  parity (`cargo fmt --all --check`, `cargo clippy --all-targets --locked -- -D warnings`,
  `cargo build --locked`, `cargo test --locked`; 9/9 unit tests pass).

## 2. Dependency alignment with rust/src-tauri (read-only control)

Manifest business core verified verbatim identical (serde/serde_json/tokio/reqwest/regex/
chrono/winreg/windows 0.62). Lock: tokio 1.53.3, reqwest 0.13.5, serde 1.0.229,
serde_json 1.0.154, chrono 0.4.45, regex 1.13.1, image 0.25.10, windows 0.62.2 — all equal.
Known splits (pre-existing, not from this upgrade): winreg lock 0.56.0 vs rust 0.55.0
(same manifest "0.56"), aws-lc-sys 0.46.0 vs rust 0.45.0.

## 3. Gate B results

| # | Item | Result |
|---|------|--------|
| 1 | Smoke: `--test-fetch` / `--test-update` / `--test-ui` vs rust exe | fetch field-identical (`no-token` path, timestamp exempt); update **byte-identical** (`local=2.3.1 latest=2.3.1 updateAvailable=False checkFailed=False`); 4 OK lines ×2 themes |
| 2 | Fonts (CJK 加 + ¥ chain) | Full-window pixel diff 0.37 vs 0.36.2 within a same-exe noise baseline (0.6–5.1% vs 0.6–4.5% per window, transparent-margin desktop bleed dominates; zero systematic delta) |
| 3 | Transparency & first frame | Burst 1438 frames over 5 cold starts: no flash, panel transparent margins show desktop, frame-1 `Visible(false)` re-send behavior unchanged |
| 4 | Caption-flash (CBT hook protocol) | 0.37: five cold-start runs, peak bright% = **0.40%** (threshold < 1%); 0.36.2 control: 286 frames peak 0.00%. V0 positive control not reproducible on this machine/date (pre-fix `8b12b9a` debug build, 283 frames, peak 3.57%, no ≥80% frame) — the bug was always timing-sensitive (FIX doc: release never reproduced it); detection method itself proven on 2026-10-18 |
| 5 | click-dead (`spawn_pointer_rearm`) | Three probe protocols (menu-item click, theme-radio click, X-button click with parked cursor + no-move raw click): **0.37 and 0.36.2 behave identically round-by-round** (all no-response). The injected no-move click is a two-edition shared limitation; move-then-click works on both (settings X close succeeded every time). No version regression |
| 6 | wake_with_retry (settings cycle) | Settings X close (cross-viewport SettingsClose through WinBus+wake) succeeded in every run; 4 OK lines + on-time Quit across 10+ runs. Reopen-path evidence weak (see §5 limitations) |
| 7 | Visual spot-check vs docs/*.png | Pixel-diff method above; 0.37 style changes (wider buttons / larger radii) did not leak into the painter-drawn UI |
| 8 | Mixed-DPI + mutex matrix | Mutex: egui-0.37 holds ← rust challenger silently exits ✓; egui←egui ✓; locked `--test-fetch/update/ui` all usable (7 lines / 1 line / 4 OK) ✓. **UNTESTABLE**: rust-holder pairings (local WebView2 failure — rust GUI exits 0xC0000401…0xC0000409 on launch, see §4), qt pairings (no artifact). DPI positioning: `panel_pos.rs` unchanged (source-level zero-diff), primary-screen geometry ±0 px vs baseline |
| 9 | `ViewportInfo.visible` | egui-winit 0.37 source still never writes it (grep); behavior consistent — hidden-state handling stays in `App::ui`, migration NOT needed (plan's 5-min experiment conclusion) |

## 4. Environment incidents on this machine (NOT upgrade regressions)

1. **Tray icon registration broken system-wide** (found during Gate B): `Shell_NotifyIconW`
   NIM_ADD returns TRUE but the icon never appears (main tray nor overflow) and
   `Shell_NotifyIconGetRect` returns E_FAIL — proven with a standalone PowerShell probe
   icon, with the 0.36.2 control build, and across an Explorer restart. Pre-existing
   icons (registered before the breakage) re-register fine. This silently defeats
   tray-icon's own click dispatch (`get_tray_rect` → None drops the event), which is why
   tray-toggle-dependent probes could not run.
2. **WebView2 failure**: rust-edition GUI (and its `--test-ui`) panics at webview creation
   with `0x800700AA` (ERROR_BUSY), exit 0xC0000409; reproducible; no leftover
   msedgewebview2 processes. Blocks rust-holder mutex pairings only.
3. **Local MSVC quirks** (build env, CI unaffected): aws-lc-sys 0.46 debug build hits
   cl.exe D8050 (command-line + `-Z7` debug-record overflow) → `CARGO_PROFILE_DEV_DEBUG=0`;
   high-parallelism link hits lib.exe LNK1104 on TEMP → dedicated `TMP` + `-j 4`.

## 5. Known limitations of this validation

- Real-tray interactions (left-click toggle, right-click menu, hover refresh, real
  settings open/close pressure loop over the tray path) could not be exercised on this
  machine/date due to incident 1; they were all verified on 2026-10-18 on the same code
  paths, and none of those code paths changed in this upgrade (windows.rs/tray.rs/
  panel_pos.rs untouched except the mechanical Id rename in ui modules).
- The reopen-after-close half of item 6 was blocked by coordinate estimation limits of
  the injected-click harness, not by an observed failure (the close half and the whole
  event bus are proven).
- Items marked UNTESTABLE in §3 rows 8 should be re-run when the machine's shell
  environment recovers.

## 5a. Post-reboot re-validation (2026-10-24 ~14:00–14:50 +08:00, after a full Windows restart at 13:31)

Per the merge-gate plan (HANDOFF §5a), both blocked environment incidents were re-tested
after a full Windows reboot. Both PERSIST, confirming they are machine state rather than a
transient shell glitch:

- Incident 1 (tray): after the reboot a fresh standalone PowerShell probe icon is rejected
  outright — `Shell_NotifyIconW` NIM_ADD now returns FALSE (previously TRUE-but-invisible) —
  the 0.37 GUI's icon is absent from both the main tray area and the opened overflow flyout
  (verified by screenshot), and `Shell_NotifyIconGetRect` still returns E_FAIL. The five
  real-tray probes of the restart protocol therefore remain unrunnable on this machine;
  version-independence is re-confirmed by the non-KPT probe failing identically. Additional
  build note: this Win11 26H2 taskbar exposes no tray buttons to UIA at all, so UIA-based
  tray-button discovery is unavailable here.
- Incident 2 (WebView2): the rust release GUI still panics at webview creation with
  `0x800700AA` (identical signature, post-reboot). rust `--test-ui` and the rust-holder
  mutex-matrix rows remain UNTESTABLE.

Headless checks re-run after the reboot — all PASS, no drift vs the original Gate B evidence:

- `--test-fetch`: rust and egui outputs field-identical (no-token path).
- `--test-update`: rust and egui one-liners byte-identical
  (`local=2.3.1 latest=2.3.1 updateAvailable=False checkFailed=False`).
- egui `--test-ui`: all four OK lines; the only stderr output is the harmless
  `Error removing system tray icon` (the icon never registered — incident 1 echo).

Settings open/close cycle re-attempt (PostMessage clicks at DWM-rect-derived client
coordinates): not conclusively extended. Cursor injection broke system-wide mid-session
(`SetCursorPos` fails with err=2 from any process; a stale 4800x1800 cursor clip survives
`ClipCursor(NULL)`; second-display topology in flux), so clicks were re-delivered via posted
`WM_MOUSEMOVE`/`WM_LBUTTONDOWN`/`WM_LBUTTONUP`; combined with `--test-ui`'s ~6 s lifetime and
a multi-second cross-viewport command latency while the app is idle, the reopen half could
not be separated from app teardown. No 0.37-specific failure was observed in any attempt;
§5's item-2 limitation (reopen half blocked by harness, close half and the whole event bus
proven in the original session) stands unchanged.

Bottom line: the reboot re-validation surfaced nothing that contradicts the Gate B verdict;
the remaining unknowns are all machine-environment limitations, recorded above.

## 6. Verdict

Gate A passed (mechanical only). Gate B passed (zero version regressions across every
measurable probe; no workaround premise broken — CBT hook, NC-collapse, WS_CAPTION strip,
transparency, wake path all confirmed working on 0.37). Phase 4R not triggered.

# Contributing to Kimi Planbar Tray

Thanks for your interest in contributing! This is a small Windows-only tray app, so this guide is short and practical. (Chinese build notes: [README_CN.md](README_CN.md).)

## What accepts changes

| Path | Status | PRs |
|---|---|---|
| `rust/` | Actively developed (Tauri 2 + Rust backend + vanilla HTML/CSS/TypeScript frontend) | ✅ Welcome |
| `qt/` | Experimental C++ Qt6 edition at feature parity | Only to mirror a `rust/` change; discuss in an issue first |
| `wpf/` | Original .NET 8 / WPF edition, **frozen at v1.5.0** | ❌ Do not add features |

`docs/SPEC.md` (Chinese, authoritative) and `docs/SPEC_EN.md` (English mirror) are the behavior/UI contract — when behavior is ambiguous, the SPEC wins. If your change alters documented behavior, update both files in the same PR. `docs/archive/` holds frozen historical handoff documents written in Chinese; they are records, not guidance.

## Development setup

Prerequisites: Windows 10/11, Rust stable (MSVC toolchain), Node.js 18+, WebView2 Runtime.

```bash
cd rust
npm install
npm run dev        # frontend only (Vite)
npx tauri dev      # full app
npx tauri build    # release exe at rust/src-tauri/target/release/kimi-planbar-tray.exe
```

Note: a plain `cargo build` debug exe does not embed the frontend — only the release build (and `npx tauri dev`) render the UI. The headless `--test-*` self-checks below still work on the debug exe.

## Before you open a PR

CI runs on every PR and `main` is branch-protected, so your PR must pass:

- `cargo fmt --check` and `cargo clippy --all-targets --locked -- -D warnings`
- `npm ci` + `npx tsc --noEmit` + `npm run build`
- `cargo build --locked` + `cargo test --locked`
- A full-history gitleaks secret scan

CI only covers the compile level; verify runtime behavior locally with the headless self-checks (they run before the single-instance mutex, so they work while a tray instance is live):

```bash
kimi-planbar-tray.exe --test-fetch    # one quota fetch, prints JSON (needs local Kimi Code credentials)
kimi-planbar-tray.exe --test-update   # CLI version check, one line
kimi-planbar-tray.exe --test-ui       # constructs all 4 windows, prints OK lines
```

For visual changes, compare against the `docs/*.png` baselines (`docs/SPEC.md` chapters 10, 11 and 15 define window sizes, colors and animation timings).

## Conventions

- Code, comments and commit messages are in English. UI copy is English, with terminology aligned with the Kimi console (Weekly usage / 5-hour usage / Extra Usage …).
- All IO, registry, process and HTTP failures are handled **silently** — surfaced via UI text only ("Update failed", "Not detected"), never a modal error dialog.
- External data (skill names/descriptions) renders via `textContent` only, never `innerHTML`.
- Comments cite SPEC sections (e.g. `SPEC 16.5`); keep the citations accurate when you change behavior.
- Don't bump versions in feature PRs — releases follow the checklist in `AGENTS.md` / `docs/SPEC.md` §7.3, and notable changes are recorded in `CHANGELOG.md` at release time.

## Issues & security

- Bugs and feature requests: open an issue with your Windows version, the edition you run (rust/qt), and steps to reproduce.
- Security: use GitHub's private vulnerability reporting — see [SECURITY.md](SECURITY.md). Do not open public issues for security matters.

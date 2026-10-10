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
- Be kind: this project follows the Contributor Covenant — see [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

---

# 参与贡献 Kimi Planbar Tray（中文）

感谢你有意参与贡献！这是一个小型 Windows 托盘应用，本指南短而务实。

## 哪些部分接受改动

| 路径 | 状态 | PR |
|---|---|---|
| `rust/` | 活跃开发版（Tauri 2 + Rust 后端 + vanilla HTML/CSS/TypeScript 前端） | ✅ 欢迎 |
| `qt/` | 实验性 C++ Qt6 版，功能对齐 | 仅镜像 `rust/` 的改动；请先在 issue 讨论 |
| `wpf/` | 原 .NET 8 / WPF 版，**冻结于 v1.5.0** | ❌ 勿加功能 |

`docs/SPEC.md`（中文，权威）与 `docs/SPEC_EN.md`（英文镜像）是行为/UI 契约——行为有歧义时以 SPEC 为准。改动涉及已文档化的行为时，请在同一 PR 内同步两个文件。`docs/archive/` 是冻结的中文历史交接文档，仅为记录，不是指引。

## 开发环境

前提：Windows 10/11、Rust stable（MSVC 工具链）、Node.js 18+、WebView2 Runtime。

```bash
cd rust
npm install
npm run dev        # 仅前端（Vite）
npx tauri dev      # 完整应用
npx tauri build    # release exe 位于 rust/src-tauri/target/release/kimi-planbar-tray.exe
```

注意：普通 `cargo build` 的 debug exe 不内嵌前端——只有 release 构建（和 `npx tauri dev`）能渲染 UI。下面的 `--test-*` 无头自检在 debug exe 上仍可用。

## 开 PR 之前

CI 在每个 PR 上运行且 `main` 受分支保护，PR 必须通过：

- `cargo fmt --check` 与 `cargo clippy --all-targets --locked -- -D warnings`
- `npm ci` + `npx tsc --noEmit` + `npm run build`
- `cargo build --locked` + `cargo test --locked`
- 全历史 gitleaks 密钥扫描

CI 只覆盖编译级；请用无头自检在本地验证运行时行为（先于单实例互斥锁执行，可与运行中的托盘实例并存）：

```bash
kimi-planbar-tray.exe --test-fetch    # 拉取一次额度，打印 JSON（需本机 Kimi Code 凭据）
kimi-planbar-tray.exe --test-update   # CLI 版本检测，单行输出
kimi-planbar-tray.exe --test-ui       # 构造全部 4 个窗口，打印 OK
```

视觉改动请与 `docs/*.png` 基准对比（`docs/SPEC.md` 第 10、11、15 章定义窗口尺寸、配色与动画时序）。

## 约定

- 代码、注释与提交信息使用英文。UI 文案为英文，术语对齐 Kimi 控制台（Weekly usage / 5-hour usage / Extra Usage …）。
- 所有 IO、注册表、进程与 HTTP 失败均**静默处理**——仅通过 UI 文本呈现（"Update failed"、"Not detected"），绝不弹模态错误框。
- 外部数据（skill 名称/描述）只用 `textContent` 渲染，绝不用 `innerHTML`。
- 注释引用 SPEC 章节（如 `SPEC 16.5`）；改动行为时保持引用准确。
- 勿在功能 PR 里 bump 版本——发版遵循 `AGENTS.md` / `docs/SPEC.md` §7.3 的清单，重要变更在发版时记入 `CHANGELOG.md`。

## Issue 与安全

- Bug 与功能请求：开 issue 并附 Windows 版本、使用的版本（rust/qt）与复现步骤。
- 安全问题：使用 GitHub 私密漏洞报告——见 [SECURITY.md](SECURITY.md)。勿为安全问题开公开 issue。
- 保持友善：本项目遵循 Contributor Covenant——见 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。

# HANDOFF — egui 版迁移（进行中，暂停于 Phase 4 中途）

> 暂停时间：2026-10-14 晚（用户要求）。恢复入口：本文档 + `docs/EGUI-MIGRATION.md`（完整计划、各 Phase 状态与全部 gotchas）。
> 恢复方式：新会话读这两份文档即可继续，无需依赖本次会话上下文。

## 1. 一句话状态

egui 版（第四版本，纯 Rust UI：eframe 0.36.2 + tray-icon，无 WebView/Node）Phase 0–3 已完成并提交；Phase 4（parity 全面审计）执行中途暂停，其中发现的一个真实 parity bug 已修复入库（见 §4）；Phase 5（打包与文档收尾）未开始。

## 2. 提交线（分支 `egui-edition`，本地未 push）

| commit | 内容 |
|---|---|
| `f3e89f0` | Phase 0 spike（GO 结论：eframe-glow 全项通过，透明子 viewport 在 0.36.2 已修 #3632） |
| `52c3be2` | Phase 1 无头核心（8 业务模块去 Tauri 化，--test-* 与 rust 版逐字段一致） |
| `d6f3698` | Phase 2 托盘 + 主面板（SPEC 10–15 parity，动画/光晕/双主题） |
| `59943d7` | Phase 3 设置/Skills/菜单三窗（四窗全 parity，--test-ui 四行 OK） |
| 本次 | Phase 4 WIP：混合 DPI 定位真实修复（panel_pos.rs 经 `GetDpiForWindow` 换算，替代主屏 scale 假设）+ 本 HANDOFF |

## 3. 恢复操作序列

1. **环境自检**：tasklist 确认无 kimi-planbar 进程占用 mutex；`%APPDATA%\KimiPlanbarTray\settings.json` 应为 `{"Theme": "dark", "RefreshMinutes": 1, "AutoStart": true}`（用户真身，任何测试改写后须逐字节还原）；HKCU Run 的 `KimiPlanbarTray` 值应指向用户安装版路径（`...\trae_projects\kimi\KimiPlanbarTray-rust.exe`）；确认屏幕拓扑（是否多屏）。
2. **重新委派 Phase 4**：coder subagent（**默认 flash 模型——用户明确要求，勿擅自改**）。任务书要点在 §4"未完成"清单 + `docs/EGUI-MIGRATION.md` §3 Phase 计划；原 prompt 精髓：对抗式逐章审计、只用 repo 构建的 exe 做互斥测试、结构化（非像素级）截图对比、真实 bug 修+静态链复跑、结果写入 EGUI-MIGRATION §6。
3. **Phase 4 gate**（主会话亲跑）：静态链（fmt / clippy --all-targets -D warnings / build / test）+ --test-* 回归 + --test-ui 四行 + 抽验 checklist 结论与性能数字 → commit。
4. **code review 委派**：Phase 4 通过后、正式交付前，**必须先 AskUserQuestion 确认审查模型与思考强度**（用户全局规范），再委派 code-reviewer agent（只读、证伪导向、假设至少 2 处错误），完整报告转交用户。
5. **Phase 5**：见 §6 清单。

## 4. Phase 4 已完成 / 未完成

**已完成**：
- 真实 parity bug 修复：混合 DPI 多屏定位（`egui/src/panel_pos.rs` 新增 `window_dpi_scale`——egui `OuterPosition` 是逻辑点且乘窗口自身 scale，须按 `GetDpiForWindow` 换算，即 SPEC 20 启动器屏陷阱家族）。已过静态链 + --test-ui/--test-update 抽验，随本次 commit 入库。
- 部分 harness：`egui/target/phase4-shots/`（menu_mixed_dpi.ps1、user-config-backup 等）。
- 环境零残留（进程/设置/注册表均已还原）。

**未完成（重启 Phase 4 的任务书）**：
- SPEC 10–21 逐章对抗式 checklist（每条款 PASS / DEVIATION / UNTESTABLE + 证据；16–18 可引用 Phase 1 逐字节拷贝结论 + 抽查三个陷阱值）
- 跨版本互斥锁实测：只用 `rust/src-tauri/target/release/kimi-planbar-tray.exe`（--test-* 在持锁实例存活时仍可用也须验证）；qt/dist 若有 exe 一并测
- DPI/多屏审计：先探测拓扑（phase4-shots 里的 menu_mixed_dpi.ps1 暗示做过混合 DPI 测试——重启时先跑它/确认是否有第二屏，避免重复劳动）；单屏则如实标 UNTESTABLE（Qt 先例）
- 性能正式测量：冷启动（spawn→托盘就绪，3 次取中位）+ 空闲 CPU/RSS，与 rust 版同机对照
- 双主题截图与 `docs/screenshot-{light,dark}.png` 结构化对比（参照物是 mock 数据、本机无 token：几何 ±2px、SPEC 11 色值采样、数据驱动子状态用代码/单测佐证）
- 差异清单定稿 + `docs/EGUI-MIGRATION.md` §6 Phase 4 状态

## 5. 环境与流程注意事项（重要）

- **绝不启动用户安装版**（`C:\Users\rexxa\Documents\trae_projects\kimi\KimiPlanbar-tray-rust.exe`，注意大小写/路径以 reg query 实测为准）；互斥测试只用 repo 构建产物；用户已手动退出自己的实例（mutex 空闲）。
- **测试 harness 位置**：`egui/target/{phase2,phase3,phase4}-shots/`（gitignored，**cargo clean 会丢**）：tray_click.ps1（真实托盘点击）、shoot_main.ps1（按 exe 写的 kpt-windows.json 截窗）、measure_idle.ps1、sample_px.py、geom_compare.py、focus_then_blur.ps1 等。若已 clean，从 git 历史 Phase 2/3 commit 的 EGUI-MIGRATION §6 描述可重建。
- **commit 规程**：`SENSITIVE_SCAN=0 git commit`（Cargo.lock 的 SHA-256 checksum 会命中手机号/身份证正则误报，已在 Phase 0 验证为纯误报）；提交信息英文；每 phase 一 commit。
- **额度教训**：Phase 2 期间 flash 池 429（5 小时窗口耗尽）打死过一个 subagent；subagent 可跨会话 resume（当时 agent id 已不可考，重启新 agent 即可，状态全在磁盘/文档）。额度重置窗口以 429 信息为准。
- **spike.rs**：`egui/src/bin/spike.rs` 是 Phase 0 参考实现，曾被根 `.gitignore` 的 `bin/` 规则误伤（已 force-add 纳管）；勿再改动（已清理过 lint 与格式）。

## 6. Phase 5 剩余清单（未开始）

- release 单 exe 发布形态确认（`#![windows_subsystem = "windows"]` 已就位；glow 单后端；预期 6–12MB——以 Phase 4 实测为准）
- README.md / README_CN.md：第四版本段落（镜像 qt 写法）、editions 列表、构建说明
- CONTRIBUTING.md 目录表 + `.github/PULL_REQUEST_TEMPLATE.md` checklist 补 egui/（双语两处）
- SECURITY.md 核查（若 token 数据流表述需去 WebView/CSP 化则同步）
- CHANGELOG.md Unreleased 条目定稿；SPEC 7.3 版本清单与 §9 文档地图终核
- `docs/EGUI-MIGRATION.md` §6 记完结；**是否移入 docs/archive/ 与用户确认时机**
- 发版回上游（shawn-0106t/kimi-planbar-tray）前与用户确认 PR / 独立仓库（既有约定）

## 7. 关键技术速查

- 版本 pin：eframe/egui 全家 `=0.36.2`、tray-icon `=0.26.1`、windows 0.62、业务依赖与 rust/ 同版本
- egui 版全部 gotchas：`docs/EGUI-MIGRATION.md` §6 各 Phase 状态 + `AGENTS.md` egui bullet（frame-1 强制显示、OuterPosition 逻辑点、字体次帧生效、per-viewport pass 模型、ViewportInfo.visible 缺失等）
- 自检基线：`--test-ui` 四行 OK；`cargo test` 9 通过；`--test-fetch/--test-update` 与 rust 版同机一致（本机无 token 属真实状态）
- 构建入口：`cd egui && cargo build --release` → `target/release/kimi-planbar-tray-egui.exe`

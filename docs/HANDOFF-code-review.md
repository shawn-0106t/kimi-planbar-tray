# Handoff — kimi-planbar-tray 代码评审与后续修复

> 写给下一位接手的 agent。本文不重复 review 发现的完整细节（见下方"评审结果存档"），只记录上下文、状态与下一步。

## 当前状态

1. 用户要求将 `https://github.com/shawn-0106t/kimi-planbar-tray` clone 到工作目录下同名文件夹，已完成：
   - 路径：`C:/Users/shaqi1/OneDrive - Publicis Groupe/Documents/自动化/github/kimi-planbar-tray`
2. 已读取并需遵守该仓库的 `kimi-planbar-tray/AGENTS.md`（关键约定摘录）：
   - 代码、注释、commit message 用英文；与用户对话用中文
   - `rust/` 是唯一活跃版本（Tauri 2 + Rust + 无框架 TS 前端）；`wpf/` 冻结只读，不得改动
   - `docs/SPEC.md`（中文）是行为契约，Rust 注释引用 SPEC 章节号需保持准确
   - 错误一律静默吞掉、只经 UI 文本呈现，绝不弹错误对话框
   - 前端渲染外部数据只用 `textContent`，禁止 `innerHTML`
   - 无单元测试；验证用 `--test-fetch` / `--test-update` / `--test-ui` 自检参数 + 截图对比 `docs/*.png`
   - 改动后：`cd rust/src-tauri && cargo build` + `cd rust && npm run build`
   - 版本号四处同步：`rust/package.json`、`rust/src-tauri/Cargo.toml`、`rust/src-tauri/tauri.conf.json`、`make_release_zip.py`（当前实际版本 1.7.0，AGENTS.md 写的 1.6.0 已过时）
   - 未经用户确认不得对上游仓库开 PR 或 fork；不得自行 git commit/push
3. 已用 AgentSwarm 并行 4 个 subagent 完成全仓 code review（排除 `wpf/`），四个 scope：
   - Rust 安全核心（main/credentials/quota/update/lib.rs）
   - Rust 应用逻辑（polling/tray/panel/settings/skills/state/theme_watch + 配置）
   - 前端（4 个 html + src/*.ts/*.css + vite.config.ts）
   - 根目录脚本（4 个 .py + 3 个 .ps1 + README×2）

## 评审结果存档

完整发现已在上一轮对话中以结构化表格交付给用户（含文件:行号、级别、修法示意）。要点浓缩（供修复时定位）：

**确凿问题（建议修复优先级从高到低）**
- P-A `rust/src-tauri/src/lib.rs:189` `save_settings` 对 IPC 的 `refresh_minutes` 零校验 → `polling.rs:30,69` `mins * 60` u64 溢出。修法：command 入口 clamp 到 1..=30 + `saturating_mul`
- P-B `rust/src-tauri/src/polling.rs:46-56` reschedule 与滞留 retime permit 竞态，2s 首刷约 50% 概率被旧周期覆盖。修法：reschedule 分支里 `retime_delay.lock().unwrap().take()`
- P-C `rust/src-tauri/src/quota.rs:235` `(raw + 500_000) / 1_000_000` i64 溢出。修法：`saturating_add`
- P-D `rust/src/skills.ts:79-91`（及 `main.ts:121-124` 同类）`*-show` listener 在 `await initTheme()` 后注册，与 SPEC 21.2 明文矛盾。修法：listen 提到所有 await 之前
- P-E `rust/src/main.ts:97-105` `playHide` 170ms 定时器无代际守卫，hide→show→hide 截断动画。修法：hideGen 计数
- P-F `rust/src-tauri/capabilities/default.json` 删除未使用的 `opener:default`；`tauri.conf.json:74` `csp: null` 建议补一条最小 CSP
- P-G `rust/src/settings.ts:17-24` 回填用 CSS selector 插值，非法 theme 值导致全部控件不回填。修法：遍历匹配 value
- P-H `rust/src-tauri/src/quota.rs:198` percent 可为 inf/NaN；`lib.rs:107-116` `refresh_now` 无防抖；`settings.rs:52` 建议 `#[serde(default)]`
- P-I `measure_run.ps1:78,41-57` 按进程名无差别 `Stop-Process -Force` + 同名实例污染基准。修法：以 `$launched.Id` 为根遍历进程树
- P-J `make_release_zip.py:30-33` 排除清单与 .gitignore 不同步，建议改用 `git ls-files`
- P-K 文档修正：README WPF 冻结版本自相矛盾（v1.5.0 vs v1.6.0）、README 性能数字是 WPF 旧数据、AGENTS.md 版本号 1.6.0→1.7.0、`verify_icons.py` 硬编码他人机器路径

**存疑待确认（修复前需先验证）**
- Q-1 `credentials.rs:34` 未 honor `KIMI_CODE_HOME`（skills.rs 有），SPEC 16.2 与 AGENTS.md 表述矛盾
- Q-2 `skills.rs:40` 读 `.skill-lock.json` 顶层 `disabled` 对象，但本机真实文件（version 3）无该键——Skills 窗口启用/禁用列可能整体失效。验证法：在 kimi-code CLI 禁用一个 skill 后 diff 该文件

**评审中确认无问题、勿再返工的方向**：token 链路（单消费点、硬编码 URL、无日志）、锁使用（无死锁/读锁升级）、前端 XSS 面（零 innerHTML）、subprocess 注入面、CSS↔窗口尺寸契约、动画参数与 SPEC 吻合。

## 用户环境约束（全局 AGENTS.md）

- 中文回复，技术术语保留英文；每条回复末尾单独一行 `-from Kimi Code`
- 修改现有文件前先询问直接改还是建副本
- Windows 中文机（GBK/936）：Python 一律 `PYTHONUTF8=1` 前缀，文本 IO 显式 UTF-8；Bash 是 Git Bash 用 POSIX 语法
- 交付级脚本须由未参与编写的 subagent 独立验证（证伪导向）
- 未经确认不做 git mutation

## 下一步建议

用户尚未明确下一步，最可能是：挑选上述发现进行修复。开始前先与用户确认：
1. 修哪些（全部确凿问题 or 仅高优先级 P-A~P-F）
2. 是否先验证 Q-2（skill-lock 格式），它决定 skills.rs 是否需要改而不仅是 SPEC 文档
3. 修复后按仓库 AGENTS.md 跑 `cargo build` + `npm run build` + `--test-fetch`/`--test-ui` 自检
4. 是否 bump 版本（若修，按四处同步规则应为 1.7.1）

## Suggested skills

下一位 agent 按需调用 Skill 工具：

- `code-review` — 修复完成后对 diff 做复审
- `kimi-cu` — 若需操作真实 GUI 验证托盘应用行为（截图、点击托盘窗口）
- `update-config` — 仅当涉及 kimi-code 自身配置问题时（本任务大概率不需要）
- 仓库相关的 lark-*/xlsx/pptx 等 skill 与本任务无关，不需要

## 敏感信息说明

本对话未涉及任何 API key/密码/PII。仓库本身读取本机 Kimi Code OAuth token，但评审确认其仅发往官方端点；接力时不要在文档或日志中粘贴任何 token。

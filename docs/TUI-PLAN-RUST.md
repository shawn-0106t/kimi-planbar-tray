# Kimi Planbar Tray — Rust TUI 版实施方案（ratatui）

> 状态：**已实施**（2026-09-19），产物在独立仓库 [kimi-planbar-tui](https://github.com/shawn-0106t/kimi-planbar-tui)（v0.1.0），而非本文 §3 所述的本仓库 `rust-tui/` 目录。实施后的主要偏差：crate 位于独立仓库根目录、命名统一为 `kimi-planbar-tui`；dashboard 后续按用户决策简化为 kimi CLI `/usage` 风格的纯文本线框（圆角卡片方案已废弃）；新增启动时最小窗口（72×13，独占控制台守卫）与 exe VERSIONINFO 资源（`build.rs` + `winresource`）。行为契约以该仓库的 `docs/SPEC.md` 为准。
> 本文档是纯 TUI 版的两套候选方案之一（另一套：`TUI-PLAN-TS-BUN.md`，未实施）。
> 行为契约以 `SPEC.md` 为准；本文只定义 TUI 版的**范围裁剪、模块映射与落地步骤**。

## 1. 目标与范围

做一个无托盘的终端常驻仪表盘，数据与行为契约复用 SPEC 第二篇，**窗口/托盘/动画相关章节整体不适用**：

| SPEC 章节 | TUI 版处理 |
|---|---|
| 10 窗口规格 / 14 托盘行为 / 15 动画 | **不适用**（无窗口、无托盘、无动画） |
| 11 配色 | 色值映射为终端 truecolor（见 §4.2） |
| 12 面板 UI 结构 | **保留**：卡片内容、文案、`FormatReset`（12.3）、`FmtYuan`（12.5）、Extra 三态（12.4）、手动刷新 2s 防抖（12.7） |
| 13 设置窗 | 转译为终端设置表单（选项与默认值不变） |
| 16 数据与 API / 17 版本检查 / 18 设置持久化 | **原样保留**（含全部陷阱） |
| 19 自检命令 | 保留 `--test-fetch` / `--test-update` |
| 21 Skills 只读 | 保留（终端滚动列表） |

## 2. 技术栈

- Rust stable（MSVC，本机已有）+ **ratatui**（TUI 框架）+ **crossterm**（跨平台终端后端/事件）
- `tokio` + `reqwest` + `serde(_json)` + `winreg` + `regex` + `chrono` —— 与 `rust/src-tauri/Cargo.toml` 同源，零新增生态风险
- 不引 Tauri、不引 WebView

## 3. 代码复用与模块映射（核心优势）

`rust/src-tauri/src/` 的后端模块去 Tauri 化后几乎原样可搬：

| 源文件（rust/src-tauri/src） | 目标（rust-tui/src） | 改造点 |
|---|---|---|
| `credentials.rs` | `credentials.rs` | 原样搬运，无 Tauri 依赖（SPEC 16.2 凭证链：`kimi-code.json` 过期 30s 余量 → `config.toml` 手写解析兜底，`KIMI_CODE_HOME` 覆盖） |
| `quota.rs` | `quota.rs` | 原样搬运（SPEC 16.1/16.3/16.4：10s 超时、字符串建模数字、`amountLeft` 1e-8 元→`(raw+500000)/1000000` 分、`isEnabled=false` → NotActivated） |
| `polling.rs` | `polling.rs` | 去掉 `AppHandle` 事件，改 `tokio::sync::mpsc` 通知 UI；调度语义不变（2s 首刷、失败 30s 快重试、keep-last-good，SPEC 16.5） |
| `settings.rs` | `settings.rs` | 原样搬运（settings.json schema、portable.dat、HKCU Run 自启，SPEC 18） |
| `skills.rs` | `skills.rs` | 原样搬运；**连同其两个 frontmatter 单测一起搬**（SPEC 21.2） |
| `update.rs` | `update.rs` | 原样搬运（`kimi --version` 5s kill、changelog Range 4KB、GitHub API 兜底，SPEC 17） |
| `theme_watch.rs` | `theme.rs` | 简化：`winreg` 读 `AppsUseLightTheme`，启动读一次 + 30s 轮询（crossterm 无系统事件源）；仅 `theme=system` 时生效 |
| `main.rs`（自检段） | `main.rs` | 保留 `--test-fetch` / `--test-update` 打印后退出；**不需要**命名互斥锁（TUI 非常驻托盘，允许多实例） |
| —（新增） | `ui/{dashboard,settings_view,skills_view}.rs` + `app.rs` | ratatui 视图层，唯一全新代码 |

新增 crate：`rust-tui/`（不动 `rust/` `qt/` `wpf/`），版本号独立起步 `0.1.0`。

## 4. UI 设计

### 4.1 布局（ratatui `Layout` 纵向约束）

```
┌──────────────────────────────────────────────┐
│ Kimi Planbar Tray            Updated HH:mm   │  标题行（3 行高）
├────────────────────┬─────────────────────────┤
│ Weekly usage       │ 5-hour usage            │  双卡（各 7 行高）
│ 68%  [██████░░░]   │ 42%  [████░░░░░░]       │  百分比 + Gauge + 倒计时
│ Resets in 4d 3h    │ Resets in 3h 28m        │
├────────────────────┴─────────────────────────┤
│ Extra Usage                         ¥12.34   │  Extra 卡（3–5 行，三态：
│ Used ¥45.67 this month / ¥100 limit          │  Ready/No data/Not activated）
├──────────────────────────────────────────────┤
│ Kimi Code CLI  0.31.1  [Update available]    │  版本行（3 行）
├──────────────────────────────────────────────┤
│ r Refresh · s Settings · k Skills · q Quit   │  footer（1 行）
└──────────────────────────────────────────────┘
```

### 4.2 主题映射（SPEC 11.1 → ratatui `Color::Rgb`）

Moonlit / Moondark 两套调色板直接取 SPEC 11.1 十色（`#1A88FF` accent 两主题相同、`#F3F4F6`/`#17191E` 窗口底、`#FFFFFF`/`#23262D` 卡片底等），以 `theme=dark|light` 切换；`theme=system` 时由 `theme.rs` 轮询注册表决定。进度条填充恒 accent、无区间变色（SPEC 11.2）。

### 4.3 交互

- 按键：`r` 刷新（**2s 防抖**，SPEC 12.7）、`s` 设置、`k` skills、`c` 打开 Console、`g` 打开 Releases、`q` 退出；`↑/↓` 在列表/表单内移动，`Enter` 确认，`Esc` 返回
- 设置表单：主题三选一 / 刷新间隔 1·5·10·30 / 开机自启 checkbox / Save——选项与默认值同 SPEC 13.2，保存动作同 SPEC（写 JSON → 自启 → 主题 → 重排定时器）
- Skills 视图：顶部 `N skills` + 手动重扫键，按来源分组（组内名称不区分大小写排序），滚动只读（SPEC 21.3）

## 5. 事件循环架构

```
tokio::select! {
    crossterm EventStream（键盘/resize） → App 状态机 → ratatui 重绘
    polling mpsc（配额结果/失败）        → 更新状态 → 重绘 + footer 时间戳
    update mpsc（版本检查结果）          → 版本行徽标
    theme 轮询 tick（30s）               → system 模式下换调色板
}
```

倒计时文案随每次重绘重算（`FormatReset` 输入为 `reset_at - now`），无需独立秒级定时器，重绘节流 250ms。

## 6. 测试与自检

- `cargo test`：搬迁来的 skills frontmatter 两测试 + **新增 quota 解析单测**（SPEC 16.3：字符串/数字混排、`isEnabled=false`、单位四舍五入、除零）——本仓库首个真正的解析测试套件
- `--test-fetch` / `--test-update`：与 rust 版同机背靠背跑，JSON 逐字段 diff 验证一致性
- 终端兼容：Windows Terminal / VS Code 终端为基准；legacy conhost 需 `chcp 65001`，README 注明

## 7. 构建与分发

```bash
cd rust-tui
cargo build --release   # 单 exe（~3–5 MB，静态链接，免运行时）
```

分发物即单个 exe，可纳入 `make_release_zip.py`（届时再定）；不提交二进制进仓库。

## 8. 里程碑

1. **M1 骨架与 core 搬运**：`rust-tui/` crate + §3 六个模块去 Tauri 化 + 单测绿
2. **M2 dashboard**：ratatui 主视图双主题，mock 数据渲染对齐 SPEC 12 文案与格式
3. **M3 交互**：按键路由、设置表单、skills 视图、2s 防抖、后台版本检查
4. **M4 收尾**：自检 diff、`SPEC.md` 增 TUI 章节（声明 10/14/15 不适用）、`AGENTS.md` 增补、派独立 code-reviewer subagent 审查

## 9. 风险

- **ratatui × tokio 集成**：crossterm `EventStream` + mpsc 是 ratatui 官方推荐的异步模式，成熟无悬念
- **注册表轮询替代事件**：30s 轮询对主题跟随足够（托盘版的实时 WM_SETTINGCHANGE 在 TUI 场景无对应物），已按偏差写明
- **工作量**：视图层全新编写是唯一大头；core 搬运省掉约 60% 逻辑量

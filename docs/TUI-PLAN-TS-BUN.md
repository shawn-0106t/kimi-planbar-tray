# Kimi Planbar Tray — TS + Bun TUI 版实施方案（OpenTUI）

> 状态：待实施。本文档是纯 TUI 版的两套候选方案之一（另一套：`TUI-PLAN-RUST.md`）。
> 行为契约以 `SPEC.md` 为准；本文只定义 TUI 版的**范围裁剪、模块设计与落地步骤**。

## 1. 目标与范围

与 Rust TUI 版完全相同的范围裁剪（见 `TUI-PLAN-RUST.md` §1）：SPEC 第 10/14/15 章不适用；12/13/16/17/18/19/21 章行为契约保留；SPEC 16.3 的解析陷阱（字符串建模数字、`amountLeft` 1e-8 元→分、`isEnabled=false` → NotActivated）必须逐条移植并由测试钉死。

## 2. 技术栈与前提

- **Bun**（本机未装，需先安装：`npm i -g bun` 或官方脚本，免管理员）——完整 TS 原生直跑、`bun:test` 内置测试、`bun build --compile` 一键单 exe
- **TypeScript**（完整语法，无 erasable 限制）
- **OpenTUI**（`@opentui/core` 命令式 API，不引 React）：Zig 内核 + `bun:ffi`，Bun 专属；opencode 生产环境验证，Windows 可用
- 零其他运行时依赖：HTTP 用内置 `fetch` + `AbortSignal.timeout(10_000)`；子进程用 `Bun.spawn`

## 3. 目录与模块设计

新建 `ts/`（不动 `rust/` `qt/` `wpf/`），单包，版本独立起步 `0.1.0`：

```
ts/
├── package.json
├── src/
│   ├── core/               # UI 无关，逐模块对应 SPEC
│   │   ├── credentials.ts  # 16.2 凭证链：kimi-code.json（expires_at > now+30s）→ config.toml 手写逐行解析；KIMI_CODE_HOME 覆盖
│   │   ├── quota.ts        # 16.1/16.3/16.4：fetch + 防御性解析；interface 上 JSON 数字一律 string 建模、容忍数字兜底
│   │   ├── polling.ts      # 16.5：2s 首刷 / 失败 30s 快重试 / keep-last-good 补齐
│   │   ├── settings.ts     # 18：settings.json + portable.dat + HKCU Run 自启（Bun.spawn 调 reg.exe，输出按 GBK 解码）
│   │   ├── skills.ts       # 21.2：三目录扫描 + SKILL.md 前 4KiB frontmatter 手写解析 + 首开缓存
│   │   ├── update.ts       # 17：kimi --version（5s kill）+ changelog Range 4KB + GitHub API 兜底
│   │   ├── format.ts       # 12.3/12.5：搬运 rust/src/common.ts 的 formatReset/fmtYuan/clampPercent + 全部 DTO 类型
│   │   └── theme.ts        # reg query AppsUseLightTheme：启动读一次 + 30s 轮询（TUI 无系统事件源）
│   ├── tui/
│   │   ├── app.ts          # OpenTUI 装配：布局、双主题调色板、键盘路由
│   │   ├── dashboard.ts    # 主视图（布局同 TUI-PLAN-RUST.md §4.1）
│   │   ├── settingsView.ts # 设置表单（选项/默认值同 SPEC 13.2）
│   │   └── skillsView.ts   # 分组可滚动只读列表（SPEC 21.3）
│   └── main.ts             # 入口；--test-fetch / --test-update 自检先于一切初始化
└── test/                   # bun:test：16.3 单位换算/混排兜底/isEnabled、16.2 过期回退、格式化边界
```

## 4. 关键实现要点

- **主题**：SPEC 11.1 十色映射 OpenTUI truecolor（`#RRGGBB` 直给）；`theme=system` 由 `theme.ts` 轮询注册表决定
- **reg.exe GBK 陷阱**：中文系统 reg 输出为 GBK——spawn 结果按 GBK 解码；自启只做写/删不做读回校验，规避乱码面
- **进度条**：块状字符 gauge（`█`/`░`），填充恒 accent、无区间变色（SPEC 11.2）
- **防抖**：手动刷新 2s 防抖（SPEC 12.7）
- **core 与 tui 严格分层**：core 不 import 任何 OpenTUI 符号——OpenTUI 年轻，渲染层可整体替换为手写 ANSI 而不动 core

## 5. 测试与自检

- `bun test`：SPEC 16.3/16.2/12.3/12.5 的边界用例（本仓库首个解析测试套件，与 Rust 版方案同构）
- `--test-fetch` / `--test-update`：与 rust 版同机背靠背 JSON 逐字段 diff

## 6. 构建与分发

```bash
cd ts
bun install
bun run src/main.ts                          # 开发
bun test                                     # 测试
bun build --compile ./src/main.ts --outfile kpt-tui.exe   # 单 exe（~90 MB，内嵌 Bun 运行时）
```

分发二选一：单 exe（体积大但免运行时）或 README 要求用户装 Bun。不提交二进制进仓库。

## 7. 里程碑

1. **M1 环境与 core**：安装 Bun（需用户确认）→ `ts/` 骨架 → core 八模块 + `bun:test` 单测绿
2. **M2 dashboard**：OpenTUI 主视图双主题，mock 数据对齐 SPEC 12 文案与格式
3. **M3 交互**：键盘路由、设置表单、skills 视图、2s 防抖、后台版本检查
4. **M4 收尾**：自检 diff、`SPEC.md` 增 TUI 章节、`AGENTS.md` 增补、派独立 code-reviewer subagent 审查

## 8. 风险

- **OpenTUI 年轻**：官方自称尚未 production-ready（opencode 已在生产使用）；对策为 §4 的 core/tui 分层
- **分发体积**：`--compile` ~90 MB，远大于 Rust 版 3–5 MB；接受或改为要求装 Bun
- **Bun Windows 边角**：TTY/spawn 的 Node 兼容边角偶有坑；本项目 API 面窄（stdin raw mode、spawn、fetch），风险可控
- **注册表只能 spawn reg.exe**：无原生 winreg 等价物；写自启/读主题已验证可行，GBK 解码按 §4 处理

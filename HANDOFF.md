# HANDOFF — kimi-planbar-tray Rust 化重构接力手册

> 写给接力设备上的 Kimi Code 会话：读完本文件 + `docs/UI-SPEC.md` 即可无缝开工。
> 上一份会话日期：2026-08-07。暂停原因：原设备（Windows，无管理员权限）无法安装 MSVC 构建工具，Rust 工具链无法落地。
> 状态更新（2026-08-08）：**重写已完成并通过验证**（双端含阴影边距修复）；仓库已重组为 `wpf/` + `rust/` 双版本 monorepo。
> 状态更新（2026-08-22）：**WPF 版停止维护**（停留在 v1.5.0，代码保留仅供参考）；自 v1.6.0 起新功能（如 Skills 只读窗口，见 UI-SPEC 第 12 章）只进 Rust 版。

## 1. 目标

把 [shawn-0106t/kimi-planbar-tray](https://github.com/shawn-0106t/kimi-planbar-tray)（WPF / .NET 8）重构为 **Rust 技术栈**的 Windows 托盘应用，**1:1 保留现有 UI/UX**，功能与现有版本完全对齐（不加新功能）。

## 2. 已确认的技术决策（用户已拍板，勿再变更）

- **框架**：Tauri 2（不用纯 Rust 手搓 webview，不用 React）
- **后端**：Rust（reqwest + tokio + serde + windows-rs 系）
- **前端**：vanilla HTML + CSS + TS（UI 只有两张用量卡 + 设置窗，不上框架）
- **功能范围**：对齐现有 planbar-tray —— 5h/周用量卡、重置倒计时、Extra Usage（加油包）、双主题跟随系统、刷新间隔（1/5/10/30，默认 5）、CLI 版本检查、开机自启（HKCU）、便携模式（portable.dat）、hover-to-fresh、失败 30s 快速重试、`--test-fetch` / `--test-ui` 自检
- **复刻基准**：`docs/UI-SPEC.md`（343 行，全部数值已从 WPF 源码提取：窗口尺寸 380×468、圆角 14、强调色 #1A88FF、动画参数、API 细节、Extra Usage 的 1e-8 元单位陷阱等）

## 3. 接力方式与仓库结构

接力载体：**GitHub 仓库 [shawn-0106t/kimi-planbar-tray](https://github.com/shawn-0106t/kimi-planbar-tray) 的 `rust-rewrite` 分支**（从 main 切出，WPF 源码原样保留作只读参考）。

分支上的目录结构（2026-08-08 重组后）：

```
kimi-planbar-tray/                 # rust-rewrite 分支
├── HANDOFF.md                     # 本文件（仓库根目录）
├── docs/UI-SPEC.md                # UI/UX + 行为完整规格（343 行，双端共享）
├── docs/*.png                     # 原版界面截图（作视觉对比基准）
├── wpf/                           # WPF / .NET 8 原版（继续维护）
├── rust/                          # Tauri 2 / Rust 重写版（含 src-tauri/ + vanilla TS 前端）
├── analyze_wpf_shadow.py          # 阴影 alpha 通道验证脚本
└── measure_run.ps1                # 进程树性能测量脚本
```

新设备上手：

```bash
git clone https://github.com/shawn-0106t/kimi-planbar-tray.git
cd kimi-planbar-tray
git checkout rust-rewrite
```

然后对 Kimi Code 说：「读 HANDOFF.md 继续开发」。

## 4. 新设备环境准备（一次性）

前提：新设备为 Windows 10/11，有管理员权限（仅装 VS Build Tools 时需要一次 UAC）。

```powershell
# 1) VS Build Tools（C++ 工作负载，含 MSVC + Windows SDK）—— 会弹 UAC
winget install --id Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended --passive"

# 2) Rust 工具链（免管理员）
winget install --id Rustlang.Rustup
rustup default stable-x86_64-pc-windows-msvc

# 3) 验证
rustc --version && cargo --version && node --version
```

Node.js 需要 18+（Tauri CLI 用），若无则 `winget install OpenJS.NodeJS.LTS`。

## 5. 实施步骤（任务拆解）

1. **脚手架**：在仓库根目录下新建 `rust/` 子目录放 Tauri 项目（与 WPF 源码并存，便于对照参考；功能验证完成后由用户决定是否提到根目录/清理旧代码）：`cd rust && npm create tauri-app@latest . -- --template vanilla-ts`（或手工按 Tauri 2 文档建 `src-tauri/` + 静态前端）；`tauri.conf.json` 配置：无边框透明窗口 380×468、置顶、跳过任务栏、托盘图标。
2. **Rust 后端**（按 UI-SPEC 第 5–9 章实现）：
   - `credentials.rs` — 凭证链：`~/.kimi-code/credentials/kimi-code.json` 的 `access_token`（校验 `expires_at` > now+30s）→ `config.toml` 兜底 api_key
   - `quota.rs` — `GET https://api.kimi.com/coding/v1/usages`，Bearer，10s 超时；响应防御性解析；**Extra Usage 的 `amountLeft` 单位是 1e-8 元**（详见 SPEC 第 7 章）
   - `polling.rs` — 周期刷新（默认 5min）+ 失败 30s 快速重试 + 启动 2s 首刷 + hover 10s 节流预取；失败保留上次数据
   - `tray.rs` — 左键 toggle / 右键菜单 / tooltip `Kimi Planbar Tray  5h X% · week Y%`
   - `panel.rs` — 面板定位（工作区右下各留 12px）、滑入滑出动画（淡入 160ms + 上移 16px/220ms CubicEaseOut；反向收起）、失焦收起（300ms 内再点忽略）、设置窗打开时抑制失焦
   - `settings.rs` — JSON 持久化，portable.dat 探测，HKCU Run 键自启，单实例 Mutex `KimiPlanbarTray.SingleInstance`
   - `update.rs` — `kimi --version`（5s 超时）→ changelog Range 请求 → GitHub API fallback
3. **前端**（按 UI-SPEC 第 1–4 章 1:1 复刻）：CSS 变量实现 Moonlit/Moondark 双主题，Rust 监听系统主题切换后通知前端。
4. **自检命令**：`--test-fetch`（拉一次配额打 JSON 退出）、`--test-ui`（构造两窗体验证资源）。
5. **验证**：`npm run tauri dev` 人工核对 UI 与 WPF 版截图（main 分支 `docs/*.png`，本分支同路径已有）逐像素对比；`npm run tauri build` 产出 exe 实测。
6. **交付前**：按用户全局规范，必须派 subagent 对全部改动做 code review，确认无问题后再交付。

## 6. 验收标准

- [ ] 视觉与交互对照 UI-SPEC 逐项一致（含动画时长/缓动、tooltip 格式、菜单定位边缘情况）
- [ ] 断网/凭证失效时保留上次数据 + 30s 重试，不崩溃
- [ ] Extra Usage 未开通时正确显示 "Not activated"（`isEnabled=false` 陷阱）
- [ ] portable.dat 便携模式、HKCU 自启、单实例均生效
- [ ] `--test-fetch` / `--test-ui` 通过
- [ ] 产物为单 exe（约 3 MB 量级），运行内存 ~10–20 MB，无 .NET 依赖

## 7. 注意事项

- 参考源码 = `wpf/` 目录（已停止维护，勿删，仅作只读参考）；Rust 版为唯一活跃开发线。
- 图标资产：WPF 版用 `wpf/kimi-logo.png`，Rust 版用 `rust/src-tauri/icons/`（Kimi 官方 logo，版权属 Moonshot AI，README 需保留非官方声明）。
- **许可证与归属**：保留 `LICENSE`（MIT © Shawn Qi）与 `NOTICE`（portions © baigong-ai / kimi-planbar）——Rust 重写属衍生作品，attribution 不可移除。
- 代码/注释/提交信息跟随原项目英文风格；对话用中文。
- 若打算发版回原仓库：先与用户确认是提 PR 还是另开仓库。

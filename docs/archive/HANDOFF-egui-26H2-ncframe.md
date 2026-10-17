# HANDOFF — egui 版 Win11 26H2 窗口外框/闪烁修复（进行中）

> **状态：IN PROGRESS（2026-10-17 深夜检查点）**。与 archive 内其他冻结交接不同，本文档随工作推进更新，完结后再标冻结。
> 分支：`docs/egui-dwm-hairline-limitation`。本次 commit 是**调试中途快照**：bug #1（激活闪烁）的修复代码已入库但**未完成端到端验证**；bug #2（窗口状态脱钩）定位中。

## 1. 一句话状态

用户实测发现 egui 版窗口在打开/焦点切换瞬间闪出白色原生标题栏（1 帧）。已实锤根因并实现修复（`WM_NCACTIVATE` 转发期间临时剥 `WS_CAPTION`）；但回归测试途中发现**第二个既有 bug**（settings 窗口关不掉/面板状态机与 OS 窗口脱钩，排除法证实非本次改动引入），它卡住了 bug #1 修复的验证路径。当前停在 bug #2 的根因定位。

## 2. Bug #1：激活变化瞬间闪白色原生标题栏

### 现象（用户原话）

- 点击时闪出白色大外框然后消失
- 主面板点 Settings → 主面板闪；右键托盘 → 菜单闪；打开主面板/settings/skills → 对应窗口闪

### 根因（实锤）

高速连拍（`burst_capture.ps1`，~40fps）抓到闪帧：窗口顶部出现一整条浅色原生 caption（含标题文字 "Kimi Planbar Tray Main Panel" 与右上角 X 按钮），**正好 1 帧**（bright% 从基线 ~0.06-0.28 跳到 ~2.5）。

机制链：

1. 四个窗口都带 `WS_CAPTION`（winit 为 aero-snap/undecorated-shadow 保留）。
2. 上一会话已设 `DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED`（修 26H2 持续白框）——**DWM 不再画非客户区**。
3. 于是 `DefWindowProc` 在 `WM_NCACTIVATE`（窗口得焦/失焦）时**自己用 GDI 直接画 caption**——不经 `WM_NCPAINT`（所以子类化吞掉 `WM_NCPAINT` 拦不住，已实测）。
4. GDI 画进窗口 DC（GL surface 的重定向位图），下一次 GL present 才覆盖 → 单帧白闪。
5. winit 0.30.13 必须收到 `WM_NCACTIVATE`（它是唯一设置 `is_active` 的消息；没有它 `has_active_focus` 永假 → egui 收不到 `Focused(true)` → 失焦自动隐藏的 `panel_was_focused` latch 失效），且其 handler 以 `ProcResult::DefWindowProc` 结尾（`winit-0.30.13/src/platform_impl/windows/event_loop.rs:2142`，已核实源码）——不能简单吞掉该消息。

### 修复（已入库，未验证完）

`egui/src/windows.rs::nc_collapse_proc`：拦截 `WM_NCACTIVATE`，**转发给 winit 原 wndproc 期间临时剥掉 `WS_CAPTION`**（`SetWindowLongPtrW(GWL_STYLE, style & !WS_CAPTION)`），转发返回后立即恢复。无 `SWP_FRAMECHANGED` → 不触发 NC 重算、无视觉副作用；winit 不处理 `WM_STYLECHANGING/WM_STYLECHANGED`（已核实）；`apply_diff` 反正每次可见性切换都整体重写样式。DefWindowProc 找不到 caption 可画 → 不闪。

**验证状态**：连拍回归做过一轮但**作废**——当时 settings 点击实际未生效（bug #2 已发作），没有发生激活切换，"无闪帧"是空转。bug #2 修复后须重测全部场景：settings 点击、右键菜单、菜单项开各窗口、关 settings 焦点回落。

## 3. Bug #2：窗口状态脱钩（既有 bug，本次改动无关）

### 复现序列

`开 settings → X 关 settings → 面板失焦自动隐藏 → 托盘重开面板`，第一轮全部正常；**从第二轮起稳定发作**：

1. settings 窗口**一直可见但完全无响应**（X 点了不关；trace 显示 `WinAction::SettingsClose` 再也收不到 → settings 子 viewport 的 egui pass 死了）。应用侧 `settings_open=false`（认为已关）→ 与 OS 可见性脱钩。
2. 面板托盘重开：`toggle_panel → show_panel` 正常执行、状态机走到 `Shown`，`App::ui` 持续每 500ms 运行，但 **OS 层面窗口保持隐藏**（root viewport 的 `Visible(true)` 命令不再生效）。期间面板还会出现对鼠标输入无响应（PostMessage 直发 `WM_LBUTTONDOWN` 也无反应）→ root viewport 的事件处理也停了。
3. root HWND 不变（trace 里恒为同一句柄）→ **排除窗口销毁重建**。

### 排除法记录（别再重查）

- **与我本次 `WM_NCACTIVATE` 改动无关**：把 style-strip 临时禁用重建后，同一序列照样发作。
- **不是 viewport GC/重建**：HWND 不变。
- **不是 UIA 探测假象**：`IsWindowVisible` 直查 + 截图双重确认。
- 上一轮会话还排除过：winit `apply_diff` 竞态（有 NCRP+子类化在不闪）、窗口类背景刷（两个窗口类 hbrBackground=0）、"四个 HWND 全消失"假象（当时是我误杀进程 + PS EnumWindows 回调内 pid 过滤有怪癖——**别在 EnumWindows 回调里做 pid 过滤**，用 `enum_all_windows.ps1` 的全量收集后置过滤）。

### 当前假设（待验证）

eframe 0.36.2 的 glow_integration：`on_window_event` 以 `glutin.viewport_from_window.get(&window_id)` 路由事件、`handle_viewport_output` 执行 viewport 命令；`builder.patch()` 返回 `recreate=true` 时会把 `viewport.window/egui_winit/gl_surface` 全部置 None（glow_integration.rs:1561-1570）。疑似某个时序下 deferred viewport（settings）的 pass 停摆 → 它的命令队列（含 `Visible(false)`）不再执行；root 的 `Visible(true)` 同样不落地。下一步：在 `register_viewports`/`handle_viewport_output` 层面加打点或直接读 eframe 源码确认 pass 停摆条件（重点：`ViewportBuilder::patch` 的 recreate 触发条件、deferred viewport 的 RedrawRequested 唤醒链、occlusion 判定）。

## 4. 当前代码树状态（本次 commit）

- `egui/src/windows.rs`：上一会话的 NCRP_DISABLED + NC 折叠子类化（已验证，修持续白框）+ 本次的 NCACTIVATE style-strip（待验证）+ **临时 trace 计数器**（`TRACE_*` atomics，零成本）。
- `egui/src/app.rs`：`KPT_TRACE=1` 环境变量门控的打点——每 500ms 向 `%TEMP%\kpt_trace.log` 写一行（panel_state / focused / was_focused / pointer / root hwnd / 消息计数），并在 `show_panel`/`start_hide`/`finish_hide`/`open_settings`/`close_settings`/`execute_win_action` 打 `>>>` 事件行。**定位完 bug #2 后全部移除。**
- `egui/src/main.rs`、`AGENTS.md`、`CHANGELOG.md`：上一会话持续白框修复的配套改动（watcher 提前到 HWND 出现即样式化等），已完整。
- `scripts/diagnostics/` 新增本 arc 的全部测试脚本（见 §5）。
- 已通过：`cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`。

## 5. 测试工具链（scripts/diagnostics/，均可复用）

- `burst_capture.ps1` — 区域高速连拍 + 定时合成点击（`-Left/-Top/-Width/-Height/-DurationMs/-OutDir/-ClickX/-ClickY/-ClickAtMs/-Button/-Prefix`）。
- `burst_analyze.py <dir> <prefix>` — 每帧 mean 亮度 + bright%(≥200)；闪帧特征 bright% 单帧跳到 ~2.5。
- `timed_repro.ps1 -AppPid <pid>` — 带时间戳的六步复现序列（T1 托盘开面板 → T2 点 Settings → T3 X 关 → T4 点桌面 → T5 托盘重开 → T6 再点 Settings）。
- `reset_state.ps1 -AppPid <pid>` — 关 settings/skills/menu、藏面板，回到干净态。
- `open_panel_click_settings.ps1` / `bisect_sequence.ps1` / `scenario_settings.ps1` — 早期版本，功能已被 timed_repro 覆盖。
- `click_check.ps1 -AppPid -X -Y` / `click_tray_check.ps1` — 合成点击 + UIA 可见性报告。
- `post_click.ps1 -AppPid -Cx -Cy` — PostMessage 直发点击到面板 HWND（绕开 OS 命中测试，用于区分 app 侧/OS 侧输入死亡）。
- `enum_all_windows.ps1 -AppPid` — 全量枚举进程窗口（含隐藏），style/exstyle/rect。
- `dump_styles.ps1` / `find_tray_icon.ps1` / `list_kimi_procs.ps1` / `check_focus.ps1`。
- `dump_app_windows.ps1` — **pid 过滤有 bug，别用**（用 enum_all_windows.ps1）。
- `verify_autohide.ps1` / `verify_autohide2.ps1` — auto-hide 回归；注意合成点击的 focus steal 常失败（GetForegroundWindow 停在任务栏），更可靠的是先点面板空白处聚焦再点别处。

环境坐标（225% DPI，2880x1800）：托盘图标中心约 (2050,1746)（用 find_tray_icon.ps1 现查）；面板 rect (1949,545,954,1170) 物理像素；Settings 按钮 = 面板左上 + (0.603, 0.868)×(宽,高)；settings 窗 X = 窗口右上内缩 (134, -122)... 即 `(X + Width - 134, Y + 122)`。

## 6. 恢复操作序列（下一会话从这里继续）

1. `git log` 确认本 commit；当前无运行中的 egui 实例（提交前已杀）。
2. 重建运行：`cd egui && cargo build --release && (KPT_TRACE=1 ./target/release/kimi-planbar-tray-egui.exe &)`，拿 PID（`list_kimi_procs.ps1`）。
3. 复现 bug #2：`timed_repro.ps1` 连跑 2 轮（第 2 轮起稳定发作），读 `%TEMP%\kpt_trace.log`。
4. 定位：按 §3 假设查 eframe `handle_viewport_output` / `patch(recreate)` / deferred viewport 唤醒链。
5. 修完 bug #2 → 移除全部 KPT_TRACE 打点 → 重跑 bug #1 的闪帧连拍回归（四个场景）→ auto-hide 回归 → `--test-ui` 四行 OK → fmt/clippy/test。
6. 交付前：AskUserQuestion 问模型与思考强度 → 委派 code-reviewer（证伪导向）→ 用户实机确认（**确认测的是哪个构建/PID**）。
7. 文档收尾：AGENTS.md 的 egui known-trap 段补"激活变化 GDI caption 闪烁"条目（机制 + 修复方式）；CHANGELOG.md 补对应 Fixed 条目（中英双写）；SPEC.md/SPEC_EN.md 不用动（无行为变更）；本 HANDOFF 标冻结。

## 7. 已知测试环境陷阱

- `mouse_event` 合成的焦点窃取常被 shell 拒绝（前台停在任务栏），auto-hide 测试要手动先聚焦面板。
- 托盘右键菜单窗口展开后会**盖住托盘图标**（菜单底缘 y=1771 > 图标中心 1746），菜单开着时点"托盘图标"实际点在菜单上。
- PowerShell 单行 `-Command` 里 `$_` 会被 bash 吃掉——一律写 .ps1 文件用 `-File` 跑。
- `FindWindowW(null, title)` 在 PowerShell P/Invoke 里 `$null` 会被 marshal 成空字符串导致找不到——用 UIA 或 EnumWindows。
- debug 构建的托盘图标会被壳收进溢出区（图标不可见），find_tray_icon 只对 release 路径有效。

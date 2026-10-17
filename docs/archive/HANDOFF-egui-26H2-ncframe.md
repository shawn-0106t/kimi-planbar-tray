# HANDOFF — egui 版 Win11 26H2 窗口外框/闪烁修复（已完结，冻结）

> **状态：FROZEN（2026-10-17 完结）**。bug #1（激活闪烁）与 bug #2（窗口状态脱钩）均已根因修复并通过回归；egui 版经用户决策**不再继续维护**，残留已知问题接受现状（见 §8）。
> 分支：`docs/egui-dwm-hairline-limitation`（PR #25）。

## 1. 一句话状态

用户实测发现 egui 版窗口在打开/焦点切换瞬间闪出白色原生标题栏（1 帧）。根因实锤并修复（`WM_NCACTIVATE` 转发期间临时剥 `WS_CAPTION`）；回归途中发现的第二个既有 bug（settings 关不掉/面板状态脱钩）亦已根因修复（eframe 唤醒丢失重试 + winit 指针位置重新武装，见 §3）。2026-10-17 完成全部回归（§6 结果），egui 版自此冻结。

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

### 修复（已入库，回归验证通过）

`egui/src/windows.rs::nc_collapse_proc`：拦截 `WM_NCACTIVATE`，**转发给 winit 原 wndproc 期间临时剥掉 `WS_CAPTION`**（`SetWindowLongPtrW(GWL_STYLE, style & !WS_CAPTION)`），转发返回后立即恢复。无 `SWP_FRAMECHANGED` → 不触发 NC 重算、无视觉副作用；winit 不处理 `WM_STYLECHANGING/WM_STYLECHANGED`（已核实）；`apply_diff` 反正每次可见性切换都整体重写样式。DefWindowProc 找不到 caption 可画 → 不闪。

**验证状态（2026-10-17 最终回归，release 构建 + kimi-cu 真实输入 + 高速连拍）**：四个激活场景全部无闪帧——打开 settings（bright% 峰值 0.19，无跳变）、托盘右键菜单（菜单绘制帧 0.59-1.09 为壁纸/文字正常亮度，无 caption）、菜单项开窗口（Settings 重开 + Skills 本实例首开，峰值 0.22-0.24）、关 settings 焦点回落（峰值 0.26，焦点正确回落面板）。四窗稳态截图全部干净：无白色标题栏、无顶部灰条。唯一异常见 §8（一次未复现的首秀 caption 残留）。

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

### 根因（已实锤并修复，2026-10-17）

假设方向（eframe 唤醒链）证实，机制比预想更窄，两个叠加原因：

1. **eframe 0.36.2 丢弃"过期"唤醒**：目标 viewport 在 repaint 回调与事件处理之间完成了两次 pass 时，`UserEvent::RequestRepaint` 被判 "outdated" 丢弃（run.rs，egui issue #7776 时代的调度）。恰在 pass 突发结束时发出的唤醒可能是**最后一个唤醒源**——被丢后队列里的命令（TogglePanel、SettingsClose）无人 drain，settings 子 viewport 的 pass 停摆、root 的 `Visible(true)` 不落地，直到某个无关事件到来。修复：`app.rs::wake_with_retry`——每次跨线程入队在 120ms 后补发一次唤醒；egui 的 repaint-delay 门控在 pass 正常流动时抑制补发，常态零成本。
2. **winit 指针位置陈旧导致 click-dead**：egui-winit 只从 `CursorMoved` 学习指针位置，位置未知时丢弃全部鼠标按键输入；而 winit 在 隐藏→显示 周期间保留 `mouse.last_position`——窗口在光标原地不动时重新显示不产生 `CursorMoved`，点击被忽略直到光标移到新位置。修复：`windows.rs::spawn_pointer_rearm`——每次 show post 两条合成 WM_MOUSEMOVE（第二条偏移 1px 打破平局），`panel_pos.rs` 三条 show 路径全部接入。

修复后验证：带埋点中间构建上原复现序列连跑 10 轮全过（修复前第 2 轮起必发）；最终清理构建（KPT_TRACE 已移除）上 `timed_repro.ps1` 通过。注意：埋点本身的 500ms 文件 I/O 会改变时序、掩盖竞态——**验证必须在无埋点构建上复跑**。

## 4. 最终代码树状态

- `egui/src/windows.rs`：NCRP_DISABLED + NC 折叠子类化（修持续白框）+ NCACTIVATE style-strip（修激活闪帧）+ `spawn_pointer_rearm`（修 click-dead）。**KPT_TRACE 临时计数器已全部移除。**
- `egui/src/app.rs`：`wake_with_retry` 唤醒重试；**KPT_TRACE 打点已全部移除。**
- `egui/src/panel_pos.rs`：三条 show 路径（panel / centered / menu）均接入 `spawn_pointer_rearm`。
- `scripts/diagnostics/` 新增本 arc 的全部测试脚本（见 §5），外加回归期新增的 `wait_window.ps1`（UIA 轮询等窗口可见）与 `ensure_window.ps1`（IsWindowVisible 真值 + 托盘坐标可传参的重试打开）。
- 已通过：`cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test`（9 个）、`--test-ui` 四行 OK。

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

## 6. 最终回归结果（2026-10-17，release 构建，无埋点）

全部通过（第 5 步的 code-reviewer 环节经用户决策跳过）：

1. `cargo fmt --check` / `cargo clippy --all-targets --locked -- -D warnings` / `cargo test`（9 个）✓
2. `cargo build --release` 最终清理后源码重建 ✓；`--test-ui` 四行 OK ✓
3. bug #2 复现序列 `timed_repro.ps1` 通过（Settings 点击、X 关、托盘重开全通）✓
4. bug #1 闪帧连拍回归四场景全部无闪帧（峰值 bright% 0.19-0.26，无 ~2.5 跳变），四窗稳态截图干净 ✓
5. auto-hide 回归：面板聚焦后点击他处 ~2s 内自动隐藏 ✓

回归方法备注：后半程改用 **kimi-cu 真实输入 + `IsWindowVisible` ground truth**——UIA `BoundingRectangle` 对隐藏的单例窗口报陈旧矩形（假阳性），`SetCursorPos`+`mouse_event` 合成点击对 egui 窗口不可靠（输入事件可能被排队延迟处理）；kimi-cu 无法操作托盘图标（Shell_TrayWnd 排除在发现之外），托盘点击仍需 PS 脚本，且**托盘图标位置会漂移**（本次回归中左移了 63px），用前必须 `find_tray_icon.ps1` 现查。

## 7. 已知测试环境陷阱

- `mouse_event` 合成的焦点窃取常被 shell 拒绝（前台停在任务栏），auto-hide 测试要手动先聚焦面板。
- 托盘右键菜单窗口展开后会**盖住托盘图标**（菜单底缘 y=1771 > 图标中心 1746），菜单开着时点"托盘图标"实际点在菜单上。
- PowerShell 单行 `-Command` 里 `$_` 会被 bash 吃掉——一律写 .ps1 文件用 `-File` 跑。
- `FindWindowW(null, title)` 在 PowerShell P/Invoke 里 `$null` 会被 marshal 成空字符串导致找不到——用 UIA 或 EnumWindows。
- debug 构建的托盘图标会被壳收进溢出区（图标不可见），find_tray_icon 只对 release 路径有效。

## 8. 残留已知问题（接受现状，egui 版已冻结）

- **首秀 caption 残留（未复现）**：回归中一个实例的**首次**面板展示时，原生标题栏持续出现约 3.5-7 秒后自行消失（连拍 bright% 恒 1.48 铁证）；此后两个实例首秀及所有后续 show 均未再现（1/3 实例首秀命中率，无法按需复现）。疑似首次 show 时 hwnd-watcher 样式化与 winit `apply_diff` 的竞态残留。
- **合成输入延迟 drain（测试环境限定）**：`SetCursorPos`+`mouse_event` 合成点击的输入事件曾被排队、数分钟后随无关事件延迟 drain（表现为"窗口自己打开了"）；kimi-cu 真实输入与带正确坐标的托盘点击在最终构建上零失败。真实用户场景（物理鼠标持续产生事件）不受影响。
- 用户决策（2026-10-17）：egui 版作为 experimental 版**不再继续维护**，上述残留接受现状；代码保留可构建与 CI 门禁。

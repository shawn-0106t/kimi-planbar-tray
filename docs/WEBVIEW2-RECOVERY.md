# WebView2 故障恢复与自动重建建议

日期：2026-11-08
状态：已实现（v1.9.0，`rust/src-tauri/src/recovery.rs`；落地记录见文末）

## 1. 背景与故障模型

Rust 版使用 Tauri 2 + Wry + WebView2。四个窗口在启动时创建，之后只隐藏、不销毁。

本次异常中，Rust 主进程仍“正在响应”，主面板和菜单窗口仍为可见状态，但属于应用的 `msedgewebview2.exe` 进程树已经消失，窗口只剩原生外壳。现有代码没有 WebView2 失败检测和恢复逻辑，因此界面无法自行恢复。

当前相关位置：

- `rust/src-tauri/src/lib.rs`：窗口事件只处理失焦、关闭和隐藏。
- `rust/src-tauri/src/panel.rs`：展示窗口时只执行定位、`show()` 和 `set_focus()`。
- `rust/src-tauri/tauri.conf.json`：四个窗口均为透明、无边框窗口，启动时统一创建。

目标不是掩盖故障，而是：

1. 记录 WebView2 退出的直接原因。
2. 避免原生窗口残留在不可交互状态。
3. 在不丢失配额、设置和主题状态的前提下自动恢复。
4. 防止连续重建形成崩溃循环。

## 2. 先补诊断与测试隔离

### 2.1 订阅 WebView2 ProcessFailed

通过 Tauri `WebviewWindow::with_webview` 取得 Windows 原生 WebView2 对象，在初始化阶段订阅 `CoreWebView2.ProcessFailed`。Tauri 已提供原生对象入口，但 `webview2-com` 会随 Tauri minor 版本变化，实现时应锁定并核对当前版本。

至少记录：

- 窗口 label。
- `ProcessFailedKind` 与 `Reason`。
- 应用 PID、窗口 HWND、WebView2 Runtime 版本。
- 故障发生时间、窗口是否可见、应用运行时长。
- 最近一次显示/隐藏、DPI/显示器变化、睡眠/唤醒事件。

日志不得包含 OAuth token、API key、完整凭据路径或配额响应正文。

### 2.2 隔离自检的数据目录

正式 GUI 与 `--test-ui` 不应共用同一个 WebView2 user-data folder。为 `--test-ui` 设置独立临时目录，并在进程正常退出后清理；异常退出时保留目录用于诊断，下次启动再按规则清理陈旧测试目录。

历史记录中出现过 `0x800700AA (ERROR_BUSY)`。虽然它不是本次进程退出的已证实原因，但测试隔离可以排除一类干扰，避免自检影响正在使用的正式实例。

## 3. 自动恢复策略

### 3.1 建立单一恢复协调器

在 Rust 后端增加统一 recovery coordinator，不要让各窗口各自重建：

- 使用 atomic flag 或异步 Mutex 保证 single-flight，同一时刻只有一次恢复。
- 使用 generation counter 丢弃已经过期的完成回调。
- 按失败类型确定影响范围。
- 恢复期间继续保留托盘、轮询和 `AppState`，不得重复启动这些后台任务。

### 3.2 按 ProcessFailedKind 分级处理

建议初始策略：

- `RenderProcessExited`、`RenderProcessUnresponsive`：先对受影响页面执行一次受控 reload；失败或重复发生时重建该窗口。
- `BrowserProcessExited`：所有 WebView 都可能失效，统一重建四个窗口。
- `FrameRenderProcessExited`：记录并 reload 受影响页面；持续失败再重建。
- `GPUProcessExited`：先记录，优先依赖 WebView2 自身恢复；若伴随持续黑屏或后续失败，再升级为主面板重建。

不要只依赖进程存在性轮询。ProcessFailed 是主信号，打开窗口时的健康检查是补充信号。

### 3.3 重建窗口

优先研究进程内重建，而不是立即重启整个应用：

1. 隐藏受影响的原生窗口，阻止继续接收交互。
2. 记录每个窗口此前的可见状态；设置和 Skills 的打开状态同步更新，避免失焦逻辑误判。
3. 销毁已失效的 WebView 窗口，再用相同 label、URL、尺寸和窗口配置创建新实例。
4. 重新应用 DWM 圆角设置、透明配置、定位规则和事件处理。
5. 页面加载完成后重新注册 ProcessFailed，再恢复此前可见的窗口。
6. 通过现有 `get_state` 和事件机制回填配额、版本、主题与设置，不重新读取或保存凭据。
7. 对主面板重新发送 `panel-show`；此前隐藏的窗口保持隐藏。

重建逻辑应复用现有窗口配置和 `AppState`，不要再次执行完整 `run()`，也不要创建第二个托盘图标或第二组轮询任务。

如果 Tauri 当前版本无法安全销毁并用相同 label 重建，再评估受控整进程重启。整进程重启必须处理 single-instance 竞态：新进程需要等待旧进程退出，不能在旧实例仍持有互斥锁时直接启动。

### 3.4 限流与最终降级

- 建议指数退避，例如 1 秒、2 秒、5 秒。
- 建议 30 秒内最多恢复 3 次；达到上限后停止自动重建。
- 达到上限后保留可用托盘和最后已知状态，按项目现有约定静默降级，不弹错误对话框。
- 不把反复失败视为需要清空 `EBWebView` 的理由；不得自动删除用户数据目录。

## 4. 验证方案

所有故障注入都使用独立测试数据目录，不对正式实例做进程终止测试。

必测场景：

1. 终止测试实例的 renderer 进程：受影响页面能 reload 或重建，其他窗口不重复创建。
2. 终止测试实例的 browser 进程：四个窗口统一恢复，托盘仍只有一个，轮询仍只有一组。
3. 恢复前主面板可见、设置窗隐藏：恢复后可见状态保持一致，配额与主题正确回填。
4. 连续注入故障：single-flight 生效，退避和次数上限生效，无重建风暴。
5. 睡眠/唤醒、显示器切换和 DPI 变化后，窗口可渲染且位置符合 SPEC。
6. 正式实例运行期间执行 `--test-ui`：两者 WebView2 进程树互不影响。
7. 恢复后完成真实链路：托盘打开面板、手动刷新、打开/关闭设置、打开 Skills、退出。

验收时同时检查：

- 窗口截图不是全黑或透明空壳。
- UIA/前端交互可用，不能只看主进程 `Responding=True`。
- WebView2 子进程属于新实例，窗口 label 不重复。
- 日志包含失败类型与恢复结果，但不包含敏感信息。
- `cargo test`、前端 `tsc --noEmit`、构建和 `--test-ui` 均通过。

## 5. 推荐实施顺序

1. 先加入 ProcessFailed 日志与 `--test-ui` 数据目录隔离。
2. 再实现 single-flight recovery coordinator 和分级恢复。
3. 最后加入进程内重建、限流及完整故障注入验证。
4. 若进程内重建受 Tauri 生命周期限制，再单独评审受控整进程重启方案。

## 6. 落地记录（v1.9.0）

实现位置：`rust/src-tauri/src/recovery.rs`（新增模块）、`state.rs` / `lib.rs` / `panel.rs` / `polling.rs` 接线；规范条目见 `docs/SPEC.md` 第 22 章（英文同步：`docs/SPEC_EN.md` 第 22 章）。

与本文建议的对应关系：

- **2.1 诊断**：每个 webview 订阅 `CoreWebView2.ProcessFailed`，回调只做 COM 读取与非阻塞入队；日志写到 `config_dir()/webview.log`（便携模式跟随 exe），单行含 kind/reason/exit code/process description/PID/HWND/可见性/DPI/Runtime 版本/运行时长/最近 8 条 UI 事件；渲染进程之外的 signal（GPU/utility/PPAPI/sandbox/unknown）只记录。日志不含 token、API key、凭据路径或配额正文。
- **2.2 自检隔离**：`--test-ui` 在 build 前把 `app_directories_override` 指向 `%TEMP%\kimi-planbar-tray-testui-<pid>`（`AppDirectoriesOverride::Root`），WebView2 用户数据目录随之隔离；正常退出尝试删除本轮目录（WebView2 释放文件可能延迟约 10s，删除带重试），失败或异常退出时保留用于诊断，下次启动清扫超过 10 分钟的陈旧目录。6 秒退出改用 `AppHandle::exit(0)` 以让清理执行，四条 OK 行与 SPEC 19 一致。
- **3.1 协调器**：`AppState.recovery` 持有事件 channel、generation、reload/rebuild 记录、pending show 与 UI 事件环形缓冲；单个 tokio 任务消费事件，`recovering` 保证 single-flight，generation 丢弃旧 webview 的迟到事件；托盘、轮询与 `AppState` 不重建。
- **3.2 分级策略**：按本文建议实现——renderer 类先 reload、120 s 内重复失败升级为重建；`BrowserProcessExited` 重建全部四窗；`GPUProcessExited` 只记录。
- **3.3 进程内重建**：快照可见性（用 show/hide 时维护的 `last_visible`，避免与失焦隐藏竞态）→ hide → bump generation → `destroy()` → 轮询 label 从 manager 移除（≤3 s）→ `WebviewWindowBuilder::from_config` 用同一 `tauri.conf.json` WindowConfig 重建 → 重新应用 DWM 无圆角 → 重新订阅 ProcessFailed → 等页面加载（≤2 s）→ 按快照与 `pending_show` 恢复可见窗口（main 走 `show_panel`，settings/skills 走原打开路径，menu 始终隐藏），并按实际展示结果回写 `settings_open` / `skills_open` 与可见性缓存。`destroy` 不触发 `CloseRequested`；重建在后台线程执行（主线程 build 会自锁）。
  - 实测发现：`BrowserProcessExited` 一次性销毁全部四窗时，Tauri 会在最后一个窗口销毁后请求隐式退出。`run()` 的 `RunEvent::ExitRequested` 必须对 `code == None` 调用 `api.prevent_exit()`，否则进程在重建前退出；`app.exit(code)`（`code = Some`）不受影响。
  - 托盘菜单是 Settings / Skills / Exit 的唯一入口：菜单窗缺失时同样触发一次重建（重建后不自动弹出，用户再次右键即可）。
- **3.4 限流**：1 s / 2 s / 5 s 退避，滑动 30 s 内最多 3 次；超限置 `disabled`，保留托盘与最后已知数据，静默降级，不弹窗、不删用户数据目录。达到上限时额外安装原生 quit-only 托盘菜单（`kpt-fallback-quit`），保证 webview 菜单失效时仍能退出。
- **打开窗口健康检查**：`recovery::gate_show` 在窗口缺失时记录 `pending_show` 并在无恢复进行时入队一次重建请求；ProcessFailed 仍是主信号。

与原建议的差异：

- 原建议的“记录睡眠/唤醒”没有做 WM_POWERBROADCAST 订阅，而是用刷新循环的墙钟跳变（实际等待 > 预期间隔 + 60 s）近似，只用于日志。
- 菜单窗不做可见性恢复（瞬态窗口，失焦即关）。
- 未实现整进程重启兜底：进程内重建已覆盖 `BrowserProcessExited`，整进程重启会引入单实例互斥竞态且丢失状态。
- 验证脚本：`scripts/diagnostics/kill_webview2_process.ps1 -TargetPid <pid> -Kind browser|renderer|gpu`（按 user-data-dir 过滤，避免误杀其他 WebView2 应用）。

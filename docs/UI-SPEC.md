# Kimi Planbar Tray — 技术规格（WPF 参考实现 1:1 复刻用）

> 来源：`kimi-planbar-tray-wpf-ref`（WPF / .NET 8 / C#），版本 1.3.0。
> 本规格为 Tauri 2 (Rust) 复刻提供精确数值与行为定义。所有坐标单位均为 DIP（逻辑像素），DPI 感知模式为系统级（SystemAware）。
> 源代码映射：`MainWindow.xaml(.cs)`、`SettingsWindow.xaml(.cs)`、`TrayMenuWindow.xaml(.cs)`、`Themes/{Shared,Light,Dark}.xaml`、`App.xaml.cs`、`TrayManager.cs`、`Services/{QuotaService,SettingsService,UpdateService,ThemeService}.cs`。

---

## 1. 窗口规格

三个窗口均为：**无边框**（`WindowStyle=None`）、**透明背景 + AllowsTransparency**（自绘圆角窗口）、**置顶**（`Topmost=True`）、**不显示在任务栏**（`ShowInTaskbar=False`）、**禁止缩放**（`ResizeMode=NoResize`）。窗口本体背景为 `Transparent`，实际可视外观由内部 `Border`（圆角 + 背景色 + 阴影）承担，四周 Margin 为阴影空间。

### 1.1 主用量面板（MainWindow）
- 尺寸：`Width=380, Height=468`
- 根 Border：`CornerRadius=14`，`Margin=6`（阴影空间），背景 `WindowBgBrush`
- 阴影：`DropShadowEffect BlurRadius=24, ShadowDepth=2, Opacity=0.25, Color=#000000`
- 内容 Grid：`Margin=16`，5 行（Auto / * / Auto / Auto / Auto）
- 位置逻辑（`ShowNearTray`）：右下对齐工作区
  ```
  Left = WorkArea.Right  - Width  - 12
  Top  = WorkArea.Bottom - Height - 12
  ```
  （`WorkArea` = 排除任务栏的屏幕工作区；即距屏幕右/下边缘各 12px。WPF 未处理任务栏在其他边的情况，复刻时可保持同样行为。）
- 失焦行为：`Deactivated` 时自动 `HideAnimated()` 收起；设置窗打开期间通过 `_suppressDeactivate` 标志抑制。
- 窗口复用：单例 `_popup`，已可见时再次触发 = 收起（toggle）。

### 1.2 设置窗口（SettingsWindow）
- 尺寸：`Width=360, Height=420`
- 启动位置：`CenterScreen`
- 根 Border：`CornerRadius=14`，`Margin=6`，背景 `WindowBgBrush`，阴影同主面板（BlurRadius=24, Depth=2, Opacity=0.25）
- 内容 Grid：`Margin=16`，2 行（Auto / *）
- 自定义标题栏：整行 `MouseLeftButtonDown` 时 `DragMove()` 可拖动
- 行为：失焦不自动关闭；仅 ✕ 按钮或 "Save" 关闭。单例复用。

### 1.3 托盘右键菜单（TrayMenuWindow）
- 尺寸：`Width=150`，高度 `SizeToContent=Height`（4 项自适应）
- 根 Border：`CornerRadius=12`，`Margin=5`，`Padding=4`，背景 `WindowBgBrush`
- 阴影：`DropShadowEffect BlurRadius=20, ShadowDepth=2, Opacity=0.3, Color=#000000`
- 定位（`ShowAtCursor`）：
  - 取光标物理像素坐标 `Cursor.Position`，用 `GetDpiForWindow(hwnd)/96.0` 换算成 DIP（`scale<=0` 时按 1 处理）
  - `Left = min(cursorX_dip, WorkArea.Right - ActualWidth - 8)`
  - 垂直方向：若 `cursorY + ActualHeight + 24 > WorkArea.Bottom`（光标贴近底部）则向上弹出 `Top = cursorY - h - 8`，否则向下 `Top = cursorY + 8`
  - `Show()` 后必须 `Activate()` + P/Invoke `SetForegroundWindow(hwnd)` 抢前台，否则菜单拿不到焦点会立刻被 Deactivated 关闭
- 失焦关闭：`Deactivated` → `Dispatcher.BeginInvoke` 延迟判断 `if (IsVisible) Close()`（延迟是为避免 Close 期间 Deactivated 同步重入崩溃）
- 每次右键新建实例（先关旧菜单再 new）

---

## 2. 配色方案

### 2.1 画刷键值表（ARGB hex，源码原文）

| 画刷 Key | Light（Moonlit） | Dark（Moondark） |
|---|---|---|
| `AccentBrush`（强调色/进度条） | `#FF1A88FF` | `#FF1A88FF` |
| `WindowBgBrush`（窗口底） | `#FFF3F4F6` | `#FF17191E` |
| `CardBgBrush`（卡片底） | `#FFFFFFFF` | `#FF23262D` |
| `TextPrimaryBrush`（主文字） | `#FF1F2329` | `#FFF2F3F5` |
| `TextSecondaryBrush`（次文字） | `#FF6B7280` | `#FF9AA0A8` |
| `ProgressTrackBrush`（进度条轨道） | `#FFE5E7EB` | `#FF3A3E47` |
| `ButtonBgBrush`（按钮底） | `#FFE9ECF0` | `#FF2C3039` |
| `ButtonHoverBrush`（按钮悬停） | `#FFDCE2E9` | `#FF3A404B` |
| `BadgeBgBrush`（新版本徽标底） | `#FFFFF0E0` | `#FF3D2E1A` |
| `BadgeFgBrush`（新版本徽标字） | `#FFE06D00` | `#FFF0A040` |

- 强调色（Moonshot 蓝）两主题相同：`#1A88FF`。
- 选中态单选丸 / 复选框勾的文字色固定为 `White`，不随主题变。
- 托盘图标兜底蓝球：`RGB(0x1A, 0x88, 0xFF)` + 高光 `rgba(255,255,255,90/255)`。

### 2.2 进度条颜色
- **无用量区间变色逻辑**：进度条填充恒为 `AccentBrush`（`#1A88FF`），轨道恒为 `ProgressTrackBrush`。不存在按百分比切换绿/黄/红的代码。
- 填充宽度通过两列 `GridLength(p, Star)` / `GridLength(100-p, Star)` 实现，`p = clamp(percent, 0, 100)`。

---

## 3. 面板 UI 结构（MainWindow）

整体：`Grid Margin=16`，5 行。所有文案为英文（术语对齐 Kimi console：Weekly usage / 5-hour usage / Extra Usage）。

### 3.1 头部（Row 0）
- `DockPanel Margin=2,0,2,16`
- 左：logo 图片 `kimi-logo.png`，`20x20`
- 标题：`"Kimi Planbar Tray"`，`FontSize=17`，`FontWeight=SemiBold`，`TextPrimaryBrush`，`Margin=10,0,0,0`，垂直居中
- 右侧（Dock Right）：`LastUpdated`，`FontSize=11`，`TextSecondaryBrush`，`Margin=12,0,0,0`
  - 无数据：空字符串
  - 有错误：`"Update failed"`
  - 正常：`"Updated HH:mm"`（`FetchedAt` 本地时间，24 小时制）

### 3.2 用量卡片区（Row 1）
- `UniformGrid Columns=2`，两张卡片：左卡 `Margin=0,0,6,0`，右卡 `Margin=6,0,0,0`（卡间距 12）
- 卡片样式：`CardBgBrush`，`CornerRadius=12`，`Padding=16`
- 左卡「Weekly usage」（week）/ 右卡「5-hour usage」（5h），结构相同：
  - 标题：`"Weekly usage"` / `"5-hour usage"`，`FontSize=13`，`TextSecondaryBrush`
  - 百分比大字：默认 `"--"`，`FontSize=32`，`FontWeight=Bold`，`Margin=0,10,0,10`，`TextPrimaryBrush`；数据到达后为 `$"{Percent:0}%"`（显示用原始 Percent 未 clamp，进度条用 clamp 后值）
  - 进度条：`Grid Height=6`，两列（填充列起始 `0*` / 剩余列起始 `100*`）；底层 `Border CornerRadius=3` 跨两列 `ProgressTrackBrush`，上层填充 `Border CornerRadius=3 AccentBrush`（高 6、圆角 3 的胶囊条）
  - 重置倒计时：`FontSize=11`，`Margin=0,12,0,0`，`TextSecondaryBrush`

### 3.3 重置倒计时格式（`FormatReset`，`span = at - now`）
- `span < 0`：`"Resets soon"`
- `>= 1 天`：`"Resets in {int(TotalDays)}d {Hours}h"`（例：`Resets in 4d 3h`）
- `>= 1 小时`：`"Resets in {int(TotalHours)}h {Minutes}m"`
- `< 1 小时`：`"Resets in {max(1, Minutes)}m"`（至少显示 1 分钟）
- 无 `resetTime`：空字符串

### 3.4 Extra Usage 卡片（Row 2）
- `Margin=0,12,0,0`，`Padding=16,12`，`CornerRadius=12`，`CardBgBrush`
- 第一行 DockPanel：左 `"Extra Usage"`（`FontSize=13`，`TextSecondaryBrush`），右 `ExtraBalance`（默认 `"--"`，`FontSize=18`，`FontWeight=Bold`，`TextPrimaryBrush`，右对齐）
- 余额文案三态（`ExtraState`）：
  - `Ready`：有 `BalanceCents` 显示 `FmtYuan`（见 3.5），否则 `"--"`
  - `NoData`：`"No data"`
  - 其他（`NotActivated`）：`"Not activated"`
- 月度子面板 `ExtraMonthlyPanel`（`Margin=0,8,0,0`，默认 `Collapsed`）：仅当 `MonthlyEnabled && MonthlyLimitCents > 0 && MonthlyUsedCents.HasValue` 时显示
  - 同款进度条（高 6、圆角 3），`p = clamp(used/limit*100, 0, 100)`
  - 文本：`"Used {FmtYuan(used)} this month / {FmtYuan(limit)} limit"`，`FontSize=11`，`Margin=0,8,0,0`，`TextSecondaryBrush`

### 3.5 金额格式化（`FmtYuan`，单位：分 → 元）
- 负数：`"-" + FmtYuan(-cents)`
- `¥{cents/100}`，余数 > 0 时追加 `.{frac:00}`；整元省略小数。例：`1234 → "¥12.34"`，`10000 → "¥100"`

### 3.6 版本行（Row 3）
- 整行可点击卡片：`Margin=0,12,0,12`，`Padding=14,10`，`CornerRadius=12`，`Cursor=Hand`，`ToolTip="View Kimi Code releases"`
- 点击打开浏览器：`https://github.com/MoonshotAI/kimi-code/releases`（异常静默吞掉）
- DockPanel：左 `"Kimi Code CLI"`（`FontSize=13`，`TextPrimaryBrush`）；右侧水平排列：
  - `CliVersion`：默认 `"--"`，显示本地版本号或 `"Not detected"`，`FontSize=13`，`TextSecondaryBrush`
  - 新版本徽标 `NewVersionBadge`：默认 `Collapsed`；`CornerRadius=8`，`Padding=8,2`，`Margin=8,0,0,0`，底 `BadgeBgBrush`，文字 `"Update available"` `FontSize=11` `BadgeFgBrush`

### 3.7 底部按钮（Row 4）
- `UniformGrid Columns=3`，样式 `ActionButton`（见 4.3）：
  - `"⟳  Refresh"`（U+27F3 + 两个空格）`Margin=0,0,6,0` → `SafeRefresh()` + `CheckAsync()`
  - `"⚙  Settings"` `Margin=3,0` → 打开设置窗（期间抑制失焦收起）
  - `"⏻  Exit"` `Margin=6,0,0,0` → `Application.Current.Shutdown()`

---

## 4. 设置窗 UI 结构（SettingsWindow）

### 4.1 标题栏（Row 0）
- `DockPanel Margin=2,0,2,16`，整行可拖动
- logo `18x18` + 标题 `"Kimi Planbar Tray Settings"`（`FontSize=15`，`SemiBold`，`TextPrimaryBrush`，`Margin=10,0,0,0`）
- 右侧关闭按钮 `"✕"`，样式 `ChromeCloseButton`

### 4.2 设置项（Row 1，`StackPanel Margin=6,0,4,0`）
| 设置项 | 控件 | 选项 | 默认值 |
|---|---|---|---|
| 「Theme」小标题（`FontSize=13 SemiBold TextSecondaryBrush`） | — | — | — |
| 主题单选 | `RadioButton` x3，`GroupName="theme"`，样式 `ThemeRadio`；间距 `Margin=0,10,0,0` / `0,8,0,0` / `0,8,0,0` | `"System default"`=system、`"Moonlit (light)"`=light、`"Moondark (dark)"`=dark | `"system"` |
| 「Refresh interval」小标题（`Margin=0,20,0,0`） | — | — | — |
| 刷新间隔 | 水平 `StackPanel Margin=0,10,0,0`，`RadioButton` x4，`GroupName="interval"`，样式 `PillRadio`，前三个 `Margin=0,0,6,0` | `"1 min"`(Tag=1)、`"5 min"`(Tag=5)、`"10 min"`(Tag=10)、`"30 min"`(Tag=30) | `5`（XAML 中 5 min `IsChecked=True`） |
| 开机自启 | `CheckBox "Launch at Windows startup"`，样式 `ThemeCheckBox`，`Margin=0,22,0,0` | bool | `false` |
| 保存 | `Button "Save"`，样式 `ActionButton`，`Margin=0,26,0,0` | — | — |

- 保存动作：写 `settings.json` → `ApplyAutoStart()` → `Theme.Apply(theme)` → `Quota.Reschedule()` → `Close()`。
- 打开窗口时按当前设置回填勾选状态；`RefreshMinutes` 通过 Tag 字符串匹配。

### 4.3 共享控件样式（`Themes/Shared.xaml`）
- **ActionButton**：`Padding=0,10`，`FontSize=13`，前景 `TextPrimaryBrush`，背景 `ButtonBgBrush`，无边框，手型光标；模板 `Border CornerRadius=10`，悬停背景 `ButtonHoverBrush`
- **ThemeRadio**：`FontSize=13`，`Cursor=Hand`；`18x18` 圆形外框（`Stroke=TextSecondaryBrush, StrokeThickness=1.5`，透明填充），选中时内显 `8x8` 圆点（`Margin=3`，`Fill=AccentBrush`）；文字距圆 `Margin=8,0,0,0`；悬停时文字变 `AccentBrush`
- **PillRadio**（分段丸）：`FontSize=12`，`Padding=12,6`，模板 `Border CornerRadius=9`；未选中底 `ButtonBgBrush` 字 `TextPrimaryBrush`；选中底 `AccentBrush` 字 `White`；悬停底 `ButtonHoverBrush`（选中+悬停时保持 `AccentBrush`）
- **ThemeCheckBox**：`FontSize=13`；`18x18` 方块 `CornerRadius=5`，边框 `TextSecondaryBrush 1.5`，透明底；选中时底与边框变 `AccentBrush`，显示白色 `"✓"`（`FontSize=12 Bold`）；文字距框 `Margin=8,0,0,0`
- **ChromeCloseButton**：`Width=28`，`Padding=8,2`，`FontSize=14`，右对齐，前景 `TextSecondaryBrush`，透明底，模板 `Border CornerRadius=6`；悬停底 `ButtonHoverBrush` 字 `TextPrimaryBrush`
- **MenuItemButton**（托盘菜单项）：`Padding=14,9`，`FontSize=13`，`TextPrimaryBrush`，透明底，左对齐，模板 `Border CornerRadius=8`；悬停底 `ButtonHoverBrush`

---

## 5. 托盘行为（TrayManager）

- 实现：`System.Windows.Forms.NotifyIcon`；图标静态不变，刷新只更新 tooltip 文字。
- **悬停（MouseMove）→ hover-to-refresh**：节流 **10 秒**（距上次 hover 刷新 <10s 则跳过），触发 `Quota.SafeRefresh()`（异步，不等待）。
- **左键（MouseUp，Left）**：toggle 主面板。防重入：面板刚因失焦自动隐藏后的 **300 毫秒** 内的左键点击被忽略（`_lastHide` 判定，避免同一次点击先触发失焦隐藏又立刻弹回）。面板已可见 → `HideAnimated()`；不可见 → 单例复用 `ShowNearTray()`。
- **右键（MouseUp，Right）**：关闭旧菜单实例，新建 `TrayMenuWindow` 并 `ShowAtCursor()`。
  - 菜单项："Open" → 关菜单 + `TogglePopup()`；"Refresh" → 关菜单 + `SafeRefresh()` + `CheckAsync()`；"Settings" → 关菜单 + `ShowSettings()`；"Exit" → `Shutdown()`。
  - 用 `MouseUp` 而非 `MouseClick`（右键在无 ContextMenuStrip 时更可靠）。
- **Tooltip（NotifyIcon.Text）**：
  - 无数据：`"Kimi Planbar Tray"`
  - 有数据：`"Kimi Planbar Tray  5h {x}% · week {y}%"`（两个空格分隔；段缺失显示 `"?"`；`Percent:0` 格式）
  - 有错误时尾部追加 `" (update failed)"`，失败时仍展示保留的上次数据
- 图标加载：优先内嵌 `kimi-logo.png` 手工包装为 ICO（ICONDIR：reserved=0, type=1, count=1；入口 width=0/height=0 表示 256，32bpp，planes=1，payload offset=22）保留 alpha；资源缺失或异常时回退程序绘制的 32x32 蓝色圆球（圆 `(1,1,30,30)` 填充 `#1A88FF`，高光椭圆 `(7,5,10,7)` 填充 `rgba(255,255,255,90)`）。

---

## 6. 动画

### 6.1 面板滑入（`ShowNearTray`）
- 注释明确：AllowsTransparency 分层窗口上 `AnimateWindow` 不可靠，故用 WPF 动画（GPU 合成）
- 初始状态：`RootBorder.RenderTransform = TranslateTransform(0, 16)`，`Window.Opacity = 0`
- 淡入：`Opacity 0 → 1`，**160ms**，线性
- 滑入：`TranslateTransform.Y 16 → 0`（自下而上 16px），**220ms**，`CubicEase EaseOut`
- 两动画同时开始

### 6.2 面板滑出（`HideAnimated`）
- 防重入标志 `_hiding`
- 淡出：`Opacity → 0`，**130ms**，线性
- 滑出：`TranslateTransform.Y → 12`（向下 12px），**160ms**，`CubicEase EaseIn`
- 淡出完成后：`Hide()`、`Opacity = 1`（复位）、通知 `Tray.NotifyPopupHidden()`（记录 `_lastHide` 供 300ms 防重入）

### 6.3 其他
- 无其他动画。控件悬停态切换为瞬时。进度条宽度变化无动画（直接设 GridLength）。

---

## 7. 数据与 API（QuotaService）

### 7.1 请求
- URL：`GET https://api.kimi.com/coding/v1/usages`
- 头：`Authorization: Bearer {token}`、`Accept: application/json`
- HTTP 超时 **10 秒**；非 2xx → 进入失败路径

### 7.2 凭证读取优先级链（`LoadToken`，与 quota-status.py 对齐）
1. **`~/.kimi-code/credentials/kimi-code.json`**（`%USERPROFILE%/.kimi-code/`）：
   - 读取 `access_token`（字符串）
   - 校验 `expires_at`（Unix 秒，数字）> 当前 UTC 时间 + **30 秒**余量，过期则视为无效继续下一步
   - 解析异常静默吞掉
2. **兜底 `~/.kimi-code/config.toml`**（手写逐行解析，非完整 TOML parser）：
   - 按 `[section]` 分节；正则 `^(base_url|api_key)\s*=\s*"([^"]*)"` 提取键值
   - 匹配条件（`MatchProvider`）：节名以 `"providers."` 开头 **且** `base_url` 包含 `"api.kimi.com/coding"` **且** `api_key` 非空 → 返回该 `api_key`
   - 遇到新节时先结算上一节；文件结束再结算最后一节
3. 两者皆无 → 返回 `QuotaResult{ Error = "no-token" }`

### 7.3 响应 JSON 解析
- **5 小时段**：`root.limits`（数组，取第 0 个元素的 `detail` 对象）→ `ParseSegment`
- **周段**：`root.usage`（对象）→ `ParseSegment`
- `ParseSegment`：`percent = used/limit*100`（`used`、`limit` 兼容数字或数字字符串，缺失按 0；`limit<=0` 时按 1 防除零）；`resetTime`（字符串，`DateTimeOffset.TryParse`）→ `ResetAt`
- **Extra Usage**：`root.boosterWallet`（对象）：
  - 非对象/缺失 → `State = NotActivated`（"Not activated"）
  - `isEnabled == false` → `NotActivated`（防御：booster 未启用时 `amountLeft` 是"月度上限-已用"估算值而非真实余额，必须视为未开通——借鉴 KimiCodeBar v1.1.1 的 bug）
  - `balance.amountLeft`（字符串数字，兼容数字型）可解析 → `State = Ready`；单位 **1e-8 元**，换算分：`BalanceCents = (raw + 500000) / 1000000`（四舍五入）
  - 否则 → `State = NoData`（"No data"）
  - `monthlyChargeLimitEnabled == true` → `MonthlyEnabled=true`，`monthlyUsed.priceInCents` → `MonthlyUsedCents`，`monthlyChargeLimit.priceInCents` → `MonthlyLimitCents`（单位分，字符串数字）
- **注意：服务端 JSON 数字一律按字符串建模**，解析时容忍数字型兜底。

### 7.4 数据模型（等价 Rust struct）
```
QuotaSegment { percent: f64, reset_at: Option<DateTimeOffset> }
ExtraState   { NotActivated, NoData, Ready }
ExtraInfo    { state, balance_cents: Option<i64>, monthly_enabled: bool,
               monthly_used_cents: Option<i64>, monthly_limit_cents: Option<i64> }
QuotaResult  { five_hour: Option<QuotaSegment>, week: Option<QuotaSegment>,
               extra: Option<ExtraInfo>, fetched_at: DateTimeOffset, error: Option<String> }
```
（错误时 `Error` = 异常类型名，如 `"HttpRequestException"` / `"TaskCanceledException"` / `"no-token"`）

### 7.5 刷新调度与失败重试（`SafeRefresh` / `Reschedule`）
- `Reschedule()`：周期 = `max(1, RefreshMinutes)` 分钟；定时器首次延迟 **2 秒**（启动后 2s 首刷），之后按周期
- `SafeRefresh()`：
  1. `FetchAsync()`
  2. **失败时保留上次成功数据**：若 `r.Error != null && Last != null`，用 `Last` 补齐 `FiveHour`/`Week`/`Extra` 中的 null 字段（界面不清空，仅状态行提示）
  3. **失败后 30 秒快速重试**：定时器下一次触发改为 `Error != null ? 30_000ms : periodMs`，周期本身不变（成功则回到正常周期）；定时器已 Dispose 的竞态异常吞掉
  4. 触发 `Updated` 事件（UI 线程）

---

## 8. CLI 版本检查（UpdateService）

### 8.1 本地版本（`DetectLocalVersion`）
- 起子进程：`kimi --version`（`UseShellExecute=false, CreateNoWindow=true`，stdout/stderr 均重定向）
- **5000ms** 超时等待退出，超时 `Kill()` 返回 null
- 先 `WaitForExit` 再读输出（输出仅一行不会撑满管道缓冲）
- 对 stdout+stderr 合并文本正则取首个 `\d+\.\d+\.\d+`
- 任何异常 → null（面板显示 `"Not detected"`）

### 8.2 最新版本（两级 fallback）
1. **官方文档站 changelog**（优先，英文版最及时；绕开 GitHub API 限流与 hosts 屏蔽）：
   - `GET https://moonshotai.github.io/kimi-code/en/release-notes/changelog.md`
   - 请求头 `Range: bytes=0-4095`（只取前 4KB；GitHub Pages 可能忽略 Range 返回 200 全量，两种响应均兼容）
   - 正则 `^## (\d+\.\d+\.\d+)`（Multiline）首个匹配即最新版
2. **GitHub Releases API fallback**：
   - `GET https://api.github.com/repos/MoonshotAI/kimi-code/releases/latest`
   - 头 `User-Agent: KimiPlanbarTray`（必须，否则 GitHub 拒绝）
   - 取 `tag_name`（形如 `"@moonshot-ai/kimi-code@0.31.1"`），正则提取 `\d+\.\d+\.\d+`
- HTTP 超时 10 秒；两者皆失败 → `LatestVersion = null`

### 8.3 比较与状态
- `UpdateAvailable = latest != null && 两者均可解析为语义化版本 && latest > local`
- `CheckFailed = latest == null`（网络不可达时静默降级，UI 不提示）
- 完成后触发 `Updated` 事件
- 触发时机：启动时后台执行；面板 "Refresh" 按钮、托盘菜单 "Refresh" 同步触发

### 8.4 UI 表现
- 版本行显示 `LocalVersion ?? "Not detected"`
- `UpdateAvailable == true` 时显示橙色徽标「Update available」（颜色见 2.1 BadgeBg/BadgeFg）
- 整行点击跳转 `https://github.com/MoonshotAI/kimi-code/releases`

---

## 9. 设置持久化（SettingsService）

### 9.1 配置文件路径（便携模式逻辑）
- **便携模式**：exe 同目录存在 `portable.dat` 文件（内容任意，仅检测存在性）→ 配置目录 = exe 所在目录
- **否则**：`%APPDATA%\KimiPlanbarTray\`
- 配置文件：`<ConfigDir>\settings.json`

### 9.2 JSON schema（`SettingsData`，缩进格式序列化）
```json
{
  "Theme": "system",
  "RefreshMinutes": 5,
  "AutoStart": false
}
```
- `Theme`：`"system" | "light" | "dark"`，默认 `"system"`
- `RefreshMinutes`：int，可选值 1/5/10/30，默认 5
- `AutoStart`：bool，默认 false
- 加载：文件不存在或反序列化失败 → 全部回落默认值（异常静默吞掉）
- 保存：先 `CreateDirectory` 再整体覆写（异常静默吞掉）

### 9.3 开机自启（`ApplyAutoStart`）
- 注册表：`HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\Run`（per-user，不触发 UAC）
- 键名：`KimiPlanbarTray`
- `AutoStart=true` → 值 = `"{exe 完整路径}"`（带引号）
- `AutoStart=false` → 删除该值（不存在不报错）
- 异常静默吞掉

---

## 10. 测试/自检命令（App.OnStartup 命令行参数）

所有自检模式均**先于单实例 Mutex 检查**执行（保证有 GUI 实例运行时自检仍可用），打印到 stdout 后退出。子进程调用放线程池脱离 UI 线程避免死锁。

| 参数 | 行为 | 输出 |
|---|---|---|
| `--test-fetch` | 拉取一次额度，打印 JSON | `QuotaResult` 的 JSON（缩进格式、不转义非 ASCII） |
| `--test-update` | 执行一次版本检查 | 单行：`local={x} latest={y} updateAvailable={bool} checkFailed={bool}` |
| `--test-ui` | 应用当前主题后依次构造三个窗口验证资源解析与 XAML 加载 | 每窗一行 `MainWindow OK` / `SettingsWindow OK` / `TrayMenuWindow OK`；异常时 `UI-FAIL: {异常类型}: {消息}` + InnerException 消息 |
| `--screenshot <path> [--dark] [--mock]` | 真实（或模拟）额度数据 + 指定主题渲染主面板为 PNG（README 用） | `saved: <path>` |

- `--screenshot`：默认 light 主题，`--dark` 切 dark；`--mock` 注入固定数据（5h=42% 3.5 小时后重置，week=68% 4 天后重置，Extra：余额 1234 分、月度已用 4567/上限 10000 分）；否则真实拉取。渲染为 96 DPI Pbgra32 PNG，自动创建输出目录。

---

## 11. 其他实现细节

- **单实例**：命名 Mutex `"KimiPlanbarTray.SingleInstance"`，未抢到直接退出；退出时释放（异常吞掉）。关窗口不退进程（`ShutdownMode=OnExplicitShutdown`）。
- **启动顺序**：加载设置 → 构造 Theme/Quota/Update 服务 →（自检分支）→ 单实例检查 → 应用主题 → 挂系统主题事件 → 创建托盘 → 启动自动刷新（2s 后首刷）→ 后台版本检查。
- **失焦收起**：仅主面板有；设置窗打开期间用 `_suppressDeactivate` 抑制（设置窗本身失焦不收起）。
- **系统主题实时跟随**：读注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize` 的 `AppsUseLightTheme`（DWORD，0=dark，1=light，缺失默认 1）；通过 `SystemEvents.UserPreferenceChanged` 事件监听，仅当设置为 `"system"` 时重新应用（切回 UI 线程）。Tauri 复刻可用 `dark-light` crate 或自读注册表 + 监听 `WM_SETTINGCHANGE`。
- **主题切换实现**：清空应用级资源字典，依次加入 Shared + Light/Dark。所有颜色经动态资源引用，切换即时生效无需重建窗口。Shared 样式字典只加载一次。
- **DPI**：锁定系统级 DPI 感知（SystemAware）。托盘菜单定位依赖 `Cursor.Position`（物理像素）÷ `GetDpiForWindow/96` 换算 DIP，高 DPI 下缺此换算菜单会偏出屏幕。
- **DPI 陷阱（Tauri/tao 复刻）**：tao 在 app 启动时就创建全部 HWND（隐藏），位置为 `CW_USEDEFAULT`——Windows 会把新窗口放在**启动者所在屏**（例如从副屏的资源管理器窗口双击 exe，隐藏窗口就挂在副屏，带上副屏的 scale）。之后跨屏定位时若用 `LogicalPosition`/`LogicalSize`，换算用的是窗口当前所在屏的 scale 而非目标屏，多屏异 DPI 下首开必偏。**规则：跨屏落位一律用 `PhysicalPosition`（面板按主屏工作区物理像素、菜单按光标所在屏物理像素）；尺寸保持 `LogicalSize`**——tao 在 `WM_DPICHANGED` 时会用逻辑尺寸 × 新 scale 重算物理尺寸，尺寸给物理值反而会被二次放大。WPF 版无此问题：`Window` 的 HWND 在 `Show()` 时才创建，`Left`/`Top` 已先设好，换算天然用目标屏 DPI。
- **图标资源**：`kimi-logo.png` 嵌入程序集，用于：面板 logo（20x20）、设置窗 logo（18x18）、托盘图标（手工 PNG→ICO 包装，Vista+ 原生支持且保留 alpha）。Tauri 复刻需将同一 PNG 嵌入并提供 ICO（可用 `ico` crate 预生成，或直接内嵌 PNG 进 ICO 容器，同参考实现思路）。
- **事件订阅生命周期**：主面板构造时订阅 `Quota.Updated`/`Updates.Updated`，关闭时取消订阅（窗口实际只 Hide 不 Close，单例复用）；托盘订阅 `Quota.Updated` 更新 tooltip，退出时退订并销毁托盘图标。
- **异常处理基调**：所有 IO、注册表、外部进程、HTTP 调用均 try/catch 静默吞掉，失败路径以 UI 文案（"Update failed"/"Not detected"）或状态字段表达，绝不弹窗报错。
- **金额单位陷阱**：`amountLeft` 是 1e-8 元（换算分需 `(raw + 500000) / 1000000` 四舍五入），`priceInCents` 才是分；JSON 中数字均为字符串。
- **`isEnabled=false` 陷阱**：booster 未启用时 `amountLeft` 并非真实余额，必须整体判为 "Not activated"。
- **退出清理**：隐藏并销毁托盘图标 → 停刷新定时器 → 释放单实例 Mutex。

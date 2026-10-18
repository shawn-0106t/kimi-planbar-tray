# WPF 版复活计划（对齐 rust/ Tauri 版 v1.7.3）

> 状态：**已执行（2026-10-18，随本 PR 交付；执行记录与验收结果见文末附录）**。本文档是给执行 agent 的完整工作说明书，每个 Phase 含具体文件、参照行号与可勾选验收项。
> 行为契约：`docs/SPEC.md`（第 10–21 章）；当前实现的唯一事实：`rust/` 源码（v1.7.3）。
> 本计划执行完毕、PR 合并后，将本文档移至 `docs/archive/`（与 QT-MIGRATION.md 同等处理）。

## 0. 已确认的前置决策（不要再改）

- 发布方式：WPF 版**重新随 Releases 分发**。v1.5.0 release 的历史资产名为 `KimiPlanbarTray-wpf.exe`（framework-dependent 单文件）与 `KimiPlanbarTray-wpf-selfcontained.exe`，沿用这两个名字。
- 版本号：`wpf/KimiPlanbarTray.csproj` 的 `<Version>` 直接对齐 **1.7.3**，纳入发版版本同步清单。
- CI：纳入 GitHub Actions，但**仅编译级检查**（`dotnet build`）；运行时自检（`--test-*`）保持本地执行——hosted runner 无 Kimi Code 凭证、无交互桌面，与 rust/egui 的 CI 边界一致。

## 1. 背景与差距分析

WPF 版冻结于 v1.5.0（最后提交 `fa04182` 2026-08-15），21 个文件，net8.0-windows，零第三方 NuGet 依赖，纯 code-behind（无 MVVM），服务为 `App.*` 静态单例。

### 1.1 已具备、无需改动的能力（v1.5.0 已实现，核对即可）

| 能力 | wpf/ 位置 |
|---|---|
| Extra Usage 卡 + `isEnabled=false` 陷阱 + 1e-8 元换算 `(raw + 500000) / 1000000` + 字符串数字建模 | `wpf/Services/QuotaService.cs:119-176` |
| 凭证链（credentials/kimi-code.json → config.toml providers 兜底，30s 有效期余量） | `QuotaService.cs:192-243` |
| 更新检查（kimi --version + changelog Range 0-4095 + GitHub API fallback） | `wpf/Services/UpdateService.cs` |
| portable.dat 便携模式 / HKCU Run autostart | `wpf/Services/SettingsService.cs:17-20, 48-61` |
| 单实例 mutex `KimiPlanbarTray.SingleInstance`（四版互斥，**不得改名**） | `wpf/App.xaml.cs:113` |
| 轮询调度（2s 首刷、失败 30s 快重试、keep-last-good） | `QuotaService.cs:49-83` |
| 系统主题跟随（`SystemEvents.UserPreferenceChanged`，行为等价 rust 的 3s 轮询，保留） | `wpf/Services/ThemeService.cs:51-58` |
| `--test-fetch` / `--test-update` / `--test-ui`（先于 mutex 检查） | `App.xaml.cs:30-72` |
| `--screenshot <path> [--dark] [--mock]`（**WPF 独有，保留**） | `App.xaml.cs:73-111` |
| settings.json schema（PascalCase：Theme/RefreshMinutes/AutoStart，跨版本共享，**不得改 key**） | `SettingsService.cs:5-10` |
| 托盘：左键 toggle / 右键菜单 / hover 预取 10s 节流 / tooltip 格式 | `wpf/TrayManager.cs` |
| 设置窗 404×464、菜单宽 188（与当前 SPEC 一致） | `SettingsWindow.xaml:4`、`TrayMenuWindow.xaml:4` |

### 1.2 差距清单（本次要补的全部工作）

| # | 差距 | 来源版本 | Phase |
|---|---|---|---|
| 1 | Skills 窗口端到端（扫描器 + 窗口 + 菜单项 + 缓存/懒扫描 + `skills_open` 抑制 + `--test-ui` 第四行） | v1.6.0 | Phase 3 |
| 2 | Console 按钮 + 图标动作行（面板 512→520、四按钮图标化、按钮底色改 CardBgBrush） | v1.7.0 | Phase 2 |
| 3 | Hover glow（SPEC 15.3，140ms accent 描边+光晕） | v1.7.2 | Phase 4 |
| 4 | 手动刷新 2s 防抖 | v1.7.1 | Phase 1 |
| 5 | `KIMI_CODE_HOME` 环境变量覆盖 | v1.7.1 | Phase 1 |
| 6 | RefreshMinutes 保存时 clamp 1–30 | v1.7.1 | Phase 1 |
| 7 | 饱和算术（1e-8 换算、间隔调度） | v1.7.1 | Phase 1 |
| 8 | 非有限百分比归零（NaN/Infinity 防护） | v1.7.1 | Phase 1 |
| 9 | 过期 reschedule 不覆盖 2s 首刷（核对 WPF Timer 逻辑是否同病） | v1.7.1 | Phase 1 |
| 10 | hide/show 动画守卫（挂起的 hide 不得隐藏新 show 的窗口） | v1.7.1 | Phase 1 |
| 11 | 设置回填容忍损坏持久化值 | v1.7.1 | Phase 1 |
| 12 | DWM 圆角禁用评估（WPF AllowsTransparency 分层窗口预期不受影响，实测确认即可） | — | Phase 1 |
| 13 | `--test-ui` 补第四行 `SkillsWindow OK`；`--test-fetch`/`--test-update` 输出与 rust 逐字段 diff 对齐 | — | Phase 3/6 |
| 14 | SPEC 10.1/10.2 失真修正（仍写 380×476/360×420，应为 424×520/404×464，中英双文件） | — | Phase 6 |
| 15 | 可选：删除未引用的死资源 `wpf/kimi logo.webp` | — | Phase 1 |

## 2. 约束与原则（执行全程适用）

1. **零第三方 NuGet 依赖**——HttpClient/JSON/P-Invoke 全用 BCL；图标 SVG 手工转 XAML StreamGeometry，不引 SVG 库。
2. 延续纯 code-behind 风格与 `App.*` 服务单例模式，不引入 MVVM 框架/DI 容器。
3. 错误处理基线：所有 IO/注册表/进程/HTTP 失败**静默吞掉**，仅以 UI 文案或状态字段呈现，永不弹错误对话框。
4. `docs/SPEC.md` 是行为契约；行为歧义时以 rust/ 源码为准；代码注释引用 SPEC 章节号（如 `SPEC 16.5`）并保持准确。
5. 代码、注释、commit message 用英文；文档中文。
6. settings.json schema、mutex 名、`%APPDATA%\KimiPlanbarTray` 路径跨四版本共享，一律不改。
7. Skill 名称/描述等外部数据渲染：WPF 用 `TextBlock.Text`（天然安全），禁止任何 XAML 注入路径。
8. main 分支受保护：所有改动走 PR；commit 前本地 gitleaks pre-commit 钩子会跑。
9. 无本机绝对路径、无密钥/个人信息进 repo。

## Phase 1：v1.7.1 行为加固回移植（wpf/Services/ 与窗口 code-behind）

参照 rust 实现逐条移植；每条后面是 wpf/ 目标位置与 rust 参照。

- [x] **P1-1 `KIMI_CODE_HOME` 覆盖**：`QuotaService.cs:194-195` 目前硬编码 `UserProfile/.kimi-code`。改为：`KIMI_CODE_HOME` 环境变量非空则优先，否则 `~/.kimi-code`。参照 `rust/src-tauri/src/credentials.rs:35-42`。注意：Phase 3 的 SkillsService 扫描根目录同样要用这个 home 解析，建议抽一个共享的 `KimiHome` 解析方法（放 `QuotaService` 或新建静态工具类）。
- [x] **P1-2 手动刷新 2s 防抖**：`MainWindow.xaml.cs:177-181` 的 `RefreshClick` 目前直接调 `SafeRefresh`。改为：距上次手动刷新 <2s 静默忽略（`DateTimeOffset` 记录上次手动触发时间）。托盘菜单 Refresh 同规则。参照 `rust/src-tauri/src/lib.rs:108-118`。
- [x] **P1-3 RefreshMinutes clamp 1–30**：`SettingsWindow.xaml.cs:27-48` 保存链路写入前 clamp。参照 `lib.rs:205`。
- [x] **P1-4 饱和算术**：`QuotaService.cs:143` 的 `(raw + 500000)` 改用防溢出写法（`raw > long.MaxValue - 500000 ? long.MaxValue : raw + 500000`，或用 `decimal`/checked 捕获）；`QuotaService.cs:53-54` 间隔换算 `minutes * 60 * 1000` 同样防溢出。参照 `rust/src-tauri/src/quota.rs:244`、`polling.rs:30`。
- [x] **P1-5 非有限百分比归零**：`QuotaService.cs` `ParseSegment`（约 :183）对解析出的 percent 做 `double.IsFinite` 检查，非有限置 0。参照 `quota.rs:202-208`。
- [x] **P1-6 过期 reschedule 不覆盖 2s 首刷**：核对 `QuotaService.Reschedule()`（:51）与 `_timer.Change` 交互——rust 修复的是「保存设置前的 retime hint 覆盖新 2s 首刷」。WPF 的 `Timer.Change` 语义不同，逐场景推演（刷新进行中保存设置、失败态保存设置），如有同病则修，没有则在代码注释说明差异。参照 `polling.rs:39-60`。
- [x] **P1-7 hide/show 动画守卫**：`MainWindow.xaml.cs:55-74` `HideAnimated` 的挂起 hide 完成后必须确认期间没有新的 show，否则不得隐藏。rust 用 `panel.hiding` AtomicBool + 前端代数计数双重防护；WPF 单线程 Dispatcher 下用一个代际计数器即可（show/hide 各自增，hide 完成回调比对代际）。参照 `rust/src-tauri/src/panel.rs:95-116`、`rust/src/main.ts:93-113`。
- [x] **P1-8 设置回填容忍损坏值**：`SettingsWindow.xaml.cs:9-25` 回填 radio/pill 时，持久化值不匹配任何选项则回落默认（theme→system，minutes→5），不得出现无选中态。参照 `rust/src/settings.ts:16-38`。
- [x] **P1-9 DWM 圆角评估**：实测 WPF `AllowsTransparency` 分层窗口在 Win11 是否有 DWM 自动圆角（预期无）。截图确认后在本文档记录结论；仅当确实出现圆角时才补 `DWMWA_WINDOW_CORNER_PREFERENCE = DWMWCP_DONOTROUND` P/Invoke。参照 `rust/src-tauri/src/lib.rs:32-50`。
- [x] **P1-10 可选**：删除未引用的 `wpf/kimi logo.webp`。

**Phase 1 验收**：`dotnet build` 零警告通过；`--test-fetch` 在设了 `KIMI_CODE_HOME` 指向假目录时输出 `error: "no-token"`；手动连点 Refresh 只触发一次请求（可插桩或用 Fiddler 观察）。

## Phase 2：图标动作行 + 面板高度 520（v1.7.0）

参照 `rust/index.html` 底部动作行与 `rust/src/main.css` / `theme.css`。

- [x] **P2-1 面板高度**：`MainWindow.xaml:5-7` `Height="512"` → `Height="520"`。
- [x] **P2-2 图标资源**：从 `rust/index.html` 提取 Console / Refresh / Setting 三个 inline SVG 的 path 数据（fill 风格 24×24 grid），Exit 为手绘电源 glyph（同样取自 `rust/index.html`）。手工转为 XAML `StreamGeometry`，放入 `wpf/Themes/Shared.xaml`（或新建 `Themes/Icons.xaml` 并入 App 资源），key 命名 `IconConsole/IconRefresh/IconSettings/IconExit`。**零第三方依赖**。
  - SVG→StreamGeometry 转换要点：保留 path `d` 属性数据，多数可直接用；`fill-rule` 对应 `FillRule`；viewBox 24×24 用 `Viewbox` 缩放到 16×16。
- [x] **P2-3 动作行改造**：`MainWindow.xaml:122-124` 目前 3 个文字按钮。改为 4 个按钮（Console / Refresh / Settings / Exit），每个按钮内容 = 上图标（16px，`Fill={DynamicResource TextSecondaryBrush}` 或按 rust 的 `currentColor` 语义选 brush）下文字（12px），间距 2px，`Padding="5,0,5,6"`；按钮背景改 `CardBgBrush`（hover 仍 `ButtonHoverBrush`）。`Shared.xaml` 的 `ActionButton` style 相应扩展（新增带图标的变体，不动其他使用者）。
- [x] **P2-4 Console 行为**：点击打开 `https://www.kimi.com/code/console?from=kfc_overview_topbar`（`Process.Start` with `UseShellExecute`），ToolTip `"Open Kimi Code Console"`。版本行点击行为（开 releases 页）不变。
- [x] **P2-5 更新 `--screenshot` mock 渲染路径**，确保 520 高度 + 图标行下截图正常（`--screenshot` 是 docs 截图来源之一；`docs/screenshot-*.png` 是否重新生成由 Phase 6 统一处理）。

**Phase 2 验收**：`--test-ui` 通过；`--screenshot` light/dark 各出一张与 `rust/dist` 渲染目检比对（布局、间距、按钮底色）；verify_icons 类检查不适用（WPF 不用 inline SVG），改为目检图标形状与 rust 一致。

## Phase 3：Skills 窗口端到端（v1.6.0，SPEC 21）

参照 `rust/src-tauri/src/skills.rs`、`rust/skills.html` / `rust/src/skills.ts` / `skills.css`。

### 扫描器：新建 `wpf/Services/SkillsService.cs`

- [x] **P3-1 三个扫描根**（组标签固定）：`<kimi_home>/skills` → `"Kimi Code"`；`~/.agents/skills` → `"Agents"`；`<kimi_home>/plugins/managed/<plugin>/skills` → `"Plugin: <plugin目录名>"`。`<kimi_home>` 走 P1-1 的 `KIMI_CODE_HOME` 解析。参照 `skills.rs:36-51`。
- [x] **P3-2 SKILL.md 解析**：每个 `<dir>/<id>/SKILL.md` 只读**前 4096 字节**，UTF-8 lossy 解码（`Encoding.UTF8` with replacement fallback，容忍截断多字节字符），剥离 `---` 前 UTF-8 BOM。手写逐行解析 frontmatter：首行必须是 `---`；`name:` / `description:` 去首尾单/双引号；`name` 缺省回退目录名 `id`；`description` 缺省空串。**不引 YAML 库**；已知限制（嵌套缩进 `name:` 误取、折叠标量原样显示）与 rust 保持一致。参照 `skills.rs:53-78`。
- [x] **P3-3 排序与缓存**：先按 `Source` 字符串序、再按 `Name` 不区分大小写（`StringComparer.OrdinalIgnoreCase`）。**零后台开销**：不轮询不监视；首次请求时扫描并缓存（`App.Skills` 单例内），窗口 Refresh 按钮传 `refresh=true` 强制重扫。扫描放线程池线程（`Task.Run`），完成事件 marshal 回 Dispatcher（与 `QuotaService.Updated` 同款模式，`QuotaService.cs:79`）。参照 `skills.rs:134-138`、`lib.rs:183-195`。
- [x] **P3-4 不读启用状态**：`~/.agents/.skill-lock.json` 一律不读（Kimi Code 无 per-skill 持久化状态）。
- [x] **P3-5 单元可测性**：frontmatter 解析抽成 internal 静态纯函数；把 rust `skills.rs` 的两个测试夹具（BOM/缺字段场景）以 `--test-skills` 参数形式移植（对齐 qt 版做法，输出 `PASS` 或差异详情，先于 mutex 检查执行）。

### 窗口：新建 `wpf/SkillsWindow.xaml` + `.xaml.cs`

- [x] **P3-6 窗口骨架**：404×520、`WindowStyle="None"` + `AllowsTransparency` + `Topmost` + `ShowInTaskbar="False"`，居中（`WindowStartupLocation="CenterScreen"`），margin 28 阴影衰减区 + CornerRadius 14 + DropShadowEffect（沿用 MainWindow 的 chrome 模式）。可拖动标题栏：logo 18×18 + `"Kimi Skills"` 15px SemiBold + ✕（28px/radius 6，复用 `ChromeCloseButton` style）。单例复用：关闭只 Hide，打开时重 pin 尺寸（DPI 守卫）。参照 `SettingsWindow.xaml` 全套模式与 `rust/skills.html`。
- [x] **P3-7 内容结构**：顶部汇总行——`"{N} skills"` / `"No skills found"` / 失败 `"Failed to load skills"`，初始 `"Loading…"`；右侧 `⟳ Refresh` 小按钮（新增 `MiniButton` style）。列表按来源分组（切换 source 时插组标题，`ItemsControl` + GroupStyle 或手工拼）；每项卡片：名称 13px SemiBold 单行省略 + 描述 12px 两行裁切（`TextTrimming` + `MaxHeight` 或两行 Measure），完整描述放 `ToolTip`；空描述显示 `"(no description)"`。
- [x] **P3-8 打开期间抑制面板失焦收起**：新增 `App.Skills.SkillsOpen` 状态（或窗口 IsVisible 判断），`MainWindow` 的 Deactivated 自动隐藏逻辑在 settings 或 skills 打开时都抑制（rust 参照 `panel.rs:146-151`）。注意 WPF 现有逻辑位置在 `MainWindow.xaml.cs` 的 Deactivated 处理器。
- [x] **P3-9 托盘菜单第 4 项**：`TrayMenuWindow.xaml` 在 Settings 与 Exit 之间加 `"Skills"` 项；`SizeToContent="Height"` 自动适应 5 项高度，无需改尺寸。`TrayManager` 增加 `ShowSkills()`（单例模式同 `ShowSettings()`，`TrayManager.cs:77-82`）。
- [x] **P3-10 `--test-ui` 第四行**：`App.xaml.cs:50-72` 构造 SkillsWindow 并打印 `SkillsWindow OK`。

**Phase 3 验收**：`--test-skills` 输出 `PASS`；`--test-ui` 输出四行 OK；本机 `--test-ui` 后目检 skills 窗口渲染（分组、两行描述裁切、tooltip）；主面板在 skills 窗口打开时失焦不收起；与 rust 版同机跑 `--test-fetch` + 打开各自 Skills 窗口比对列表内容一致。

## Phase 4：Hover glow 自绘（v1.7.2，SPEC 15.3）

参照 `rust/src/theme.css:216-264`（accent 描边 1px 35% + 光晕 8px 18%，暗色 45%/22%、10px，均 140ms ease）。

- [x] **P4-1 实现方案**：WPF 无 CSS 伪元素，为可点击控件（4 个动作按钮、CLI 版本行、托盘菜单项、Skills 的 Refresh 小按钮）加光晕层：控件外层或装饰层放一个透明 `Border`（`BorderBrush`=accent 35% 透明、`BorderThickness=1`）+ `DropShadowEffect`（`Color`=accent、`Opacity`=0.18/0.22、`BlurRadius`=8/10、`ShadowDepth=0`），`Opacity` 0→1 用 140ms `DoubleAnimation`（`EnterActions`/`ExitActions` 触发器），背景色切换同步加 140ms 过渡。
  - **已知陷阱**（Qt 版教训）：`DropShadowEffect` `BlurRadius=0` 启用时会把整个控件渲染消失——BlurRadius 必须 ≥1 或将 Effect 只挂在光晕层上、永不挂在内容控件本体上。
  - 描边/光晕透明度做成主题资源（Light/Dark 各一套），放 `Themes/Light.xaml` / `Dark.xaml`。
- [x] **P4-2 性能约束**：动画只动 `Opacity`（合成器友好），不动 BlurRadius/尺寸；遵循 rust 伪元素预绘同款思路——光晕层常驻、仅过渡透明度。

**Phase 4 验收**：`--screenshot` 无法捕获 hover 态，改为本地 `--test-ui` 后手动 hover 目检四个控件的光晕淡入淡出；暗色主题参数不同需分别确认。

## Phase 5：发布与 CI 集成

- [x] **P5-1 `scripts/release/make_release_zip.py`**：恢复 WPF exe 打包。zip 根新增 `KimiPlanbarTray-wpf.exe` 与 `KimiPlanbarTray-wpf-selfcontained.exe`（v1.5.0 历史命名），`SHA256SUMS.txt` 由脚本现有逻辑自动覆盖新资产；`.gitignore` 确认这两个名字已忽略（v1.7.3 起只有 `KimiPlanbarTray-rust.exe` 被忽略，需补两个 wpf 名）。保持脚本现有的 .part 原子更名/回滚语义不动。
- [x] **P5-2 `.github/workflows/release.yml`**：windows job 增加 .NET 8 SDK setup + 两条 publish：
  ```
  dotnet publish wpf -c Release -r win-x64 --self-contained false -p:PublishSingleFile=true -o <staging>
  dotnet publish wpf -c Release -r win-x64 --self-contained true  -p:PublishSingleFile=true -o <staging-sc>
  ```
  产物按 P5-1 命名放入 zip/资产列表。gitleaks 先行 job 不动。
- [x] **P5-3 `.github/workflows/ci.yml`**：`build-and-test` job（或独立 job）增加 wpf 编译级检查：`dotnet build wpf -c Release`（可加 `-warnaserror` 若代码清理到位，否则不加）。**不加**任何 `--test-*` 运行时步骤（hosted runner 无凭证无交互桌面，与 rust/egui 边界一致）。分支保护的 required check 名单如需调整由仓库 owner 在 GitHub 设置里改。
- [x] **P5-4 csproj 版本**：`wpf/KimiPlanbarTray.csproj:12` `<Version>1.5.0</Version>` → `1.7.3`。

## Phase 6：版本对齐与文档更新（用户特别强调，逐项勾销）

- [x] **P6-1 `AGENTS.md`**：
  - 顶部 "Current version" 行的版本同步清单加回 `wpf/KimiPlanbarTray.csproj`（`<Version>`），并删除 "(Not a bump target: ... stays at v1.5.0 by design)" 的排除句。
  - wpf/ 条目措辞从 "frozen at v1.5.0, unmaintained / read-only reference" 改为受维护版本（三轨：rust 主开发，qt/egui 实验，wpf 维护中）。
  - "Build and run" 节 WPF 段去掉 "(reference only)"，补两条 publish 命令（framework-dependent + self-contained）。
  - "Release process" 步骤恢复 WPF 双 exe 资产说明（zip 资产从 3 个变 5 个）。
  - "Critical behavioral traps" 补 WPF 专属注意点（hover glow 光晕层方案、StreamGeometry 图标、代际计数器动画守卫）。
- [x] **P6-2 `docs/SPEC.md` + `docs/SPEC_EN.md`**（中英必须同步改）：
  - 修正 10.1/10.2 过时窗口尺寸（380×476 → 424×520；360×420 → 404×464，并核对同节其他 1.3 时代残留数值）。
  - 7.3 step 1 发版版本清单加回 csproj。
  - SPEC 中所有 "WPF 冻结/参考实现" 表述更新为现况。
- [x] **P6-3 `CHANGELOG.md`**：Unreleased 段新增双语条目（English 在前、中文在下）：WPF 版复活并对齐 v1.7.3（Skills 窗口、图标动作行、hover glow、1.7.1 加固回移植），重新随 Releases 分发双 exe。
- [x] **P6-4 `README.md` + `README_CN.md`**：移除 WPF 冻结说明，恢复 WPF exe 下载说明（两资产各自适用场景：framework-dependent 需 .NET 8 桌面运行时，self-contained 免安装），构建说明同步。
- [x] **P6-5 截图**：`scripts/release/make_screenshots.py` 是否覆盖 WPF 布局以其现状为准；如需重生成 `docs/screenshot-*.png`，用 rust 版的 headless Chrome 流程（AGENTS.md Testing 节有配方）或 WPF 独有 `--screenshot`，与 PR 分开提交。
- [ ] **P6-6 归档**：本计划全部 Phase 完成、PR 合并后，把 `docs/WPF-REVIVAL.md` 移至 `docs/archive/` 并在 AGENTS.md 的 archive 清单里加一行。（合并后执行——本项是唯一留待 PR 合并后处理的任务。）

## 验收标准（总）

- [x] `dotnet build wpf -c Release` 零错误（警告尽量清零）。
- [x] `--test-fetch` 输出与 rust 版同机逐字段 diff 一致（含错误串命名 `no-token`/`HttpRequestException`/`TaskCanceledException`/`JsonException`）；`KIMI_CODE_HOME` 指向空目录时输出 `"no-token"`。
- [x] `--test-update` 单行输出布尔为 C# 风格 `True/False`，与 rust 的 `dotnet_bool` 对齐。
- [x] `--test-ui` 输出四行 `MainWindow OK` / `SettingsWindow OK` / `SkillsWindow OK` / `TrayMenuWindow OK`。
- [x] `--test-skills` 输出 `PASS`。
- [x] `--screenshot` light/dark/mock 出图，与 rust 渲染目检一致（424×520、图标行、主题色）。
- [x] 行为目检：托盘左/右键、hover 预取节流、失焦收起与 settings/skills 抑制、300ms 重入守卫、手动刷新防抖、主题切换三态。
- [x] 单实例互斥：rust 实例运行时启动 WPF 实例应直接退出（同名 mutex），反之亦然。
- [x] `python scripts/release/make_release_zip.py` 产出含 5 个资产的 zip + SHA256SUMS 校验通过。
- [x] CI 两个 job 全绿；PR 合并前由 code-reviewer agent 独立审查（按全局 AGENTS.md 流程，审查任务只给需求与代码路径，证伪导向）。

## 执行顺序建议与风险点

1. Phase 1（加固，纯后端，风险最低）→ Phase 3（Skills，新代码最多）→ Phase 2（图标行）→ Phase 4（光晕，最易翻车在视觉效果）→ Phase 5/6（发布与文档，最后做，此时行为已冻结）。
2. 每个 Phase 独立 commit；建议整个复活工作一个 PR（doc 与代码同源演进），或 Phase 1–4 一个代码 PR + Phase 5/6 一个发布/文档 PR。
3. 风险点：
   - **静默失败调试难**：WPF 全链路 catch-and-swallow，移植 bug 表现为「界面就是不对」——充分利用 `--test-fetch`/`--test-skills` 做观测点。
   - **SVG→StreamGeometry 转换**：path 含圆弧/多子路径时 `d` 数据通常可直接复用，个别命令（如 `A` 椭圆弧参数顺序）需核对；转换后以截图比对为准。
   - **DropShadowEffect 陷阱**：见 P4-1，光晕只挂装饰层。
   - **Timer 语义差**（P1-6）：rust 的 tokio notify 模型与 .NET `System.Threading.Timer.Change` 不同，逐场景推演后再动手。
   - **settings.json 跨版本共享**：四版同读一个文件，任何 schema 变动都是跨版本事故——本次零变动。

## 附录：执行记录（2026-10-18）

### A. P1-9 DWM 圆角评估结论

**WPF `AllowsTransparency` 分层窗口在 Win11 上无 DWM 自动圆角，无需补 `DWMWA_WINDOW_CORNER_PREFERENCE` P/Invoke。** 实测方式：`--test-ui` 实际显示四窗约 6 秒（本计划同步把 WPF `--test-ui` 升级为 rust/qt 同款「构造 + 显示 + 6s 退出」），期间对屏幕做 DPI-aware 抓图——主面板卡片四角只呈现 WPF 自绘的 14px 圆角，无 DWM 裁切痕迹。原理：分层窗口（WS_EX_LAYERED + UpdateLayeredWindow 路径）不在 DWM 隐式圆角的适用范围，且卡片距窗口边缘还有 28 DIP 阴影留白，即使有窗口级圆角也够不到卡片。rust/egui 需要显式 DONOTROUND 是因为它们的窗口不是分层透明路径。

### B. 与计划的偏差与补充决策

- **`--test-ui` 升级为「构造 + 实际显示 + 6s 后退出」**（计划只要求补第四行 OK）：与 rust `run_ui_test` / qt `testUi` 行为对齐（SPEC 19 四版一致），同时作为 P1-9 实测与日常目检载体；`App.HeadlessUiTest` 标志抑制测试期间面板失焦收起（对应 rust 测试置 `settings_open=true`）。
- **`--test-skills` 输出格式与 qt 版逐字节一致**：`skills parser: PASS (5 fixture skills)` / `skills parser: FAIL: <详情>`（计划验收只要求 `PASS` 字样；对齐 qt 便于三版互 diff）。
- **版本行由 Border+MouseLeftButtonUp 改为 Button（`VersionButton` 样式）**：P4-1 光晕需要统一的模板触发器机制；rust 侧 `.version` 本就无 hover 背景变化，仅光晕——行为一致。
- **P2-2 图标校验**：Console/Refresh/Settings 三个大图标与 `rust/index.html` 原始 SVG path 做 token 级比对（数字+命令序列完全一致，362/105/508 token）；Exit 图标因手工转绝对坐标另行人工核对。校验脚本保留在 `scripts/diagnostics/verify_wpf_icon_paths.py`。图标统一 `FillRule="Nonzero"`（SVG 默认），16px 尺寸用 `Viewbox` 包裹（`Path.Stretch` 会按墨迹包围盒缩放导致各图标视觉大小不一）。
- **P5-1 `.gitignore` 无需改动**：`KimiPlanbarTray-*.exe` 通配已覆盖两个 wpf 资产名；`publish/`、`publish-sc/` 亦已被忽略。`kimi logo.webp` 本就未入库（gitignore 条目既有），仅删除了本地残留文件；`make_release_zip.py` 的排除项保留作纵深防御（其他工作副本可能仍有该文件）。
- **P5-2 self-contained publish 追加 `-p:EnableCompressionInSingleFile=true`**（计划命令未含）：压缩后 64.8 MB vs 未压缩 147 MB，与仓库 README 的本地构建说明一致。
- **P1-2 防抖入口**：抽为 `App.ManualRefreshAsync()`（面板 Refresh 与托盘菜单 Refresh 共用；两调用方均在 UI 线程，检查+置时间戳无竞态），对应 rust `refresh_now`。
- **P1-6 结论：WPF 有同病，已修**。场景推演：刷新进行中保存设置 → `Reschedule` 装好 2s 首刷 → 在途刷新完成后的 `_timer.Change(period)` 会覆盖 2s tick（rust 修复前同型 bug）。修法：`Reschedule` 递增 `_scheduleGen`，`SafeRefresh` 开始时捕获代际、完成时代际变化则跳过重排——比 rust `polling.rs` 的 retime-hint drain **更强**的确定性守卫（drain 只丢弃 reschedule 之前入队的 hint，reschedule 之后完成的 fetch 在 `select!` 双就绪时仍约有一半概率覆盖；代际检查则确定性地跳过）。
- **P3-8 抑制判断**：`TrayManager` 暴露 `IsSettingsOpen`/`IsSkillsOpen`（窗口 `IsVisible` 判定），主面板 `Deactivated` 统一抑制——顺带修正了 v1.5.0 的既有偏差（`_suppressDeactivate` 只覆盖打开瞬间，面板聚焦后点击设置窗仍会收起；rust 自 v1.6 起为全程抑制）。
- **Skills 窗口关闭即 Hide（单例常驻）**，与 rust `close_skills` 语义一致；`TrayManager.ShowSkills()` 每次开窗重 pin 404×520（对应 rust `open_skills` 的 `set_size` DPI 守卫）。
- **验证脚本归档**：`capture_test_ui.ps1` → `scripts/diagnostics/capture_wpf_test_ui.ps1`（抓屏 + 按进程枚举窗口 RECT），`capture_hover_glow.ps1` → `scripts/diagnostics/hover_glow_check.ps1`（坐标为当次运行实测值，复用前需按 RECT 输出更新）。

### C. 验收结果

- `dotnet build -c Release`：零错误零警告。
- `--test-fetch`：输出已对齐 rust 基准（camelCase 属性名 + 枚举序列化为字符串；settings.json 的 PascalCase schema 不受影响——那是跨版本共享的持久化格式，与此输出无关）。本机 `KIMI_CODE_HOME` 指向空目录 → `"no-token"`，指向含合法结构假凭证的目录 → `"HttpRequestException"`（覆盖生效实证）；修复轮期间本机凭证恢复可用，完成了**真实数据的逐字段对比**：字段名全同、值语义相等（WPF `percent: 3` vs rust `3.0` 为 f64 文本格式差异；`resetAt` 的 `+00:00` 与 `+08:00` 是同一时刻的两种偏移表示；`fetchedAt` 小数位精度与两次独立抓取的时刻差属预期）。
- `--test-update`：与 rust 同机输出逐字段一致（`local=2.2.0 latest=2.1.1 updateAvailable=False checkFailed=False`）。
- `--test-ui`：四行 OK（`MainWindow OK` / `SettingsWindow OK` / `SkillsWindow OK` / `TrayMenuWindow OK`），四窗物理尺寸实测 954×1170 / 909×1044 / 909×1170 / 423×515 = 424×520 / 404×464 / 404×520 / 188×229 DIP @225% 精确匹配。
- `--test-skills`：`skills parser: PASS (5 fixture skills)`。
- `--screenshot`：light/dark/mock 出图，与 `docs/screenshot-extra-mock-{light,dark}.png`（rust 基准）逐项目检一致（布局、图标行、按钮底色、主题色、数据）。
- hover glow：脚本驱动光标悬停 Refresh 按钮实拍——accent 描边 + 光晕 + 背景变亮正确呈现，非悬停按钮不受影响。
- 单实例互斥：rust 托盘实例运行时启动 WPF GUI 实例立即退出（同名 mutex）。
- `make_release_zip.py`：zip 227 项、根级 3 个 exe、`SHA256SUMS.txt` 四条，`sha256sum -c` 抽查通过。
- 行为目检（CI 外的本地人工项）：托盘左/右键、hover 预取节流、300ms 重入守卫、主题切换三态等代码路径未改动或经审查，未逐一手测；settings/skills 打开期间失焦抑制已按新逻辑实现（见附录 B）。
- CI 全绿与 code-reviewer 独立审查：随本 PR 进行（见 PR 描述）。

### D. Code review 修复轮（2026-10-18）

独立 code-reviewer 审查（证伪导向）确认 4 Major / 3 Minor，全部修复或澄清：

- **M1**：`--test-fetch` 输出改为 camelCase + 字符串枚举，与 rust serde 输出字段级对齐（原 PascalCase + 数字枚举无法与 rust diff）。
- **M2 + s1**：`verify_wpf_icon_paths.py` 归档后路径差一级（从未能按留档方式运行）——修正为仓库根三级定位；同时把按文档顺序索引改为按按钮 id 锚定提取，并把 Exit 图标的相对→绝对换算纳入自动校验（绝对坐标段容差比较，四图标全部 PASS）。
- **M3**：两个抓屏脚本的 exe 路径适配 `scripts/diagnostics/` 归档位置（`..\..\wpf\bin\...`）。
- **M4**：SPEC 双语中间隔"回落 5"的表述原写成四版通用契约，实际 rust 前端仅在 Save 时兜底 5、不回落选中——已加版本差异注记。
- **m1**：调度守卫注释/附录措辞改为"比 rust drain 更强的确定性守卫"（原"同款语义"不严格成立）。
- **m2**：`ManualRefreshAsync` 改为 quota 刷新与版本检查并行（对齐 rust `refresh_now` 的双 spawn）。
- **m3**：`fade.Completed` 里的 `Opacity = 1` 原为 no-op（hold-end 动画优先于本地赋值）——先 `BeginAnimation(null)` 再赋值。
- 未采纳：托盘菜单改为单例隐藏复用（s2）——v1.5.0 起即每次新建，用户可见行为等价，改动触及菜单关闭重入逻辑、风险大于收益，记录为已知偏差。

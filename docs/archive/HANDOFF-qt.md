# HANDOFF — Qt 版开发交接（kimi-planbar-tray qt/）

> 最后更新：2026-09-12（Phase 0 完成当日）。继续 Qt 版开发前请先读本文件 + `docs/archive/QT-MIGRATION.md`（方案与模块映射）+ `docs/SPEC.md` 第二篇（行为契约）。
> 参考：`docs/archive/HANDOFF.md` 是 WPF→Rust 的历史交接（已冻结），本文件是 Qt 版的活交接文档，随 Phase 推进更新。

---

## 1. 当前进度

- **Phase 0 环境：已完成并验收**（详见 `docs/archive/QT-MIGRATION.md` 第 9 章状态节）。
- **Phase 1 无头核心：已完成并验收**（2026-09-13）。credentials / quota / settings / update / state 五模块已平移至 `qt/src/`，`main.cpp` 已重写为"CLI 解析 → `--test-*` → `CreateMutexW` → QApplication"。同机验收：rust/qt 两版 `--test-fetch` 逐字段 diff 一致（仅 fetchedAt 时间戳各自不同）、`--test-update` 行完全一致、no-token 错误路径形状一致、rust GUI 常驻时 qt `--test-*` 正常且 qt GUI 被互斥锁静默拒绝。注意：GUI 子系统 exe 的 CRT stdio 在无控制台时不可用，`--test-*` 输出改为 `GetStdHandle + WriteFile`（Git Bash 管道与 cmd 控制台都覆盖），失败才退回 AttachConsole + CONOUT$；fetchedAt 的 9 位纳秒用 `GetSystemTimePreciseAsFileTime`（100ns 单位，末两位恒为 0，与 rust 一致）。`--test-ui` 留到 Phase 2/3（窗口尚未建）。
- **Phase 2 托盘+主面板：已完成并验收**（2026-09-13）。新增 `polling` / `trayicon` / `panel` / `app` / `theme` / `windows/panelwindow` 与 `async_util` / `format_util` / `icons.h`（SVG 路径照抄 `rust/index.html`），`assets/` 复制了 `icon.png` + `kimi-logo.png`（qrc）。验收：构建 0 错 0 警；`--test-fetch`/`--test-update` 与 rust 版回归一致；浅色/深色面板截图与 `docs/screenshot-{light,dark}.png` 目检一致（含 Extra 卡 ¥22.37、月限额行、版本行 0.42.0、倒计时文案）；托盘左键 toggle、失焦自动隐藏（300ms 重入守卫）、第二实例静默退出均实测通过（rust GUI 常驻时 qt `--test-*` 照常）。已踩坑：① QPushButton 的样式 sizeHint 不看子布局，动作按钮行被压塌——IconButton 需 `setMinimumHeight(45)` + 纵向 Fixed；② hover 光晕 QSS 无 `::after`/box-shadow，仅近似为 hover 时 1px accent 描边（无光晕、无 140ms 过渡，SPEC 15.3 偏差，Phase 4 定夺）；③ 托盘 hover-to-refresh 未接（QSystemTrayIcon 无 hover 事件，SPEC 14 偏差，按 7.3 暂缓方案 A）；④ 右键菜单窗为 Phase-3 stub（Context 暂不动作）；⑤ 焦点丢失自动隐藏依赖真实激活切换，PowerShell 后台 `SetForegroundWindow` 会被系统拒绝，测试要用真实点击。
- **Phase 3 其余三窗：已完成并验收**（2026-09-13）。新增 `skills.h/.cpp`（三处根目录扫描 + frontmatter 行解析，语义与 rust 逐行一致）、`windows/{settingswindow,skillswindow,menuwindow}.h/.cpp`、`windows/titlebar.h`（可拖动标题栏）、`windows/clamplabel.h`（2 行 clamp + 省略号自绘，QTextLayout 实现；QLabel 只会截断半个字形）；`app.h/.cpp` 接管四窗单例路由（openSettings/saveSettings/openSkills/loadSkills/showMenu/menuAction）。验收：构建 0 错 0 警；`--test-fetch`/`--test-update` 回归一致；新增 `--test-ui`（四窗构造 + 打印 MainWindow/SettingsWindow/SkillsWindow/TrayMenuWindow OK，6s 自退，与 rust 行序一致）与 Qt 独有的 `--test-skills`（移植 rust 两个 cargo 测试夹具，PASS）。GUI 实测：托盘右键→菜单（光标所在屏、贴底翻转、SetForegroundWindow）→Settings/Skills 全链路；设置保存回写 settings.json（同 schema 2 空格缩进）+ HKCU Run 增删 + 主题即时应用 + Poller reschedule；Skills 首开懒扫描 102 条、Refresh 强制重扫；settingsOpen/skillsOpen 抑制面板失焦隐藏。已踩坑：① `QTimer::singleShot` 在 `--test-ui` 路径未触发（原因未查明），6s 退出改为 `QThread::sleep + QCoreApplication::exit`（线程安全）；② header-only 类加 `Q_OBJECT` 必须把头文件列进 CMakeLists 源清单，否则 AUTOMOC 不处理（链接找不到 staticMetaObject）；③ 构建前必须先杀 qt GUI 进程（exe 文件锁），且只能按 PID/路径杀；④ 托盘溢出浮层（TopLevelWindowForOverflowXamlIsland）会随手势开合，脚本化测试用真实点击 chevron + 轮询等待最稳，UIA Invoke 不可靠。
- **字体统一补充（2026-09-13，Phase 3 追加）**：原 `main.cpp` 强制 `QFont("Segoe UI", 9)`，中文落进任意 fallback；改为 `QFontDatabase::systemFont(GeneralFont)`（`applySystemFont()`，GUI 与 `--test-ui` 两路同用），对齐 Web 端 `'Segoe UI', system-ui, sans-serif` 的意图（中文 Windows 上 GeneralFont = 微软雅黑，中英文统一）。QSS 无 `font-family`，其余 setFont 只改字号/字重、继承族名。四窗截图复核：Skills 中文描述已按雅黑渲染。
- **Phase 4 Parity 检查：已完成并验收**（2026-09-13）。SPEC 10–21 逐章对照 rust 版审计：10/11/12/13/14/15/16/17/18/19/21 全部条款匹配，仅两处遗留偏差本轮闭环——① SPEC 15.3 hover 光晕：已升级为 1px accent 描边（QSS）+ `windows/hoverglow.h` 的 QGraphicsDropShadowEffect 光晕（亮 8px/18%、暗 10px/22%，140ms 渐显渐隐），覆盖面板 4 按钮 + 版本行 + 菜单项 + Skills Refresh + 设置 Save；**坑：QGraphicsDropShadowEffect 启用后 blurRadius=0 会把整个 widget 渲染消失**，必须非 hover 时 `setEnabled(false)`，Enter 启用、Leave 动画结束后停用。② SPEC 14 hover-to-refresh：方案 A（nativeEventFilter 截获 shell 托盘回调消息）实测抓不到 Qt 内部托盘窗（`Qt693TrayIconMessageWindowClass`）的 WM_MOUSEMOVE，放弃；最终落地为 500ms 轮询 `QSystemTrayIcon::geometry()`（内部走 Shell_NotifyIconGetRect）+ "光标在图标矩形内且自上次 tick 有移动"才算 hover 事件（与 rust 的 Enter/Move 事件语义一致，停放不重触发）+ 10s 节流，实测日志验证命中。视觉回归：`--test-ui` 下四窗 × 双主题截图全过（面板含版本行与 4 按钮完整渲染）；**测试基建坑**：Qt 顶层窗的 Win32 类名是 `Qt693QWindowToolSaveBits`，UIA 的 ClassName 属性才是 C++ 类名（PanelWindow 等），脚本找窗要用 UIA；PS 5.1 的 SetCursorPos 有 DPI 虚拟化错乱，必须 `SetProcessDPIAware()` + `System.Windows.Forms.Cursor.Position`；`--test-ui` 里 settings/skills 原本都居中互相遮挡，已错开 460px（仅测试布局，输出契约不变）。多 DPI：全程逻辑坐标（PerMonitorV2），本机 225% 下四窗物理尺寸/位置逐一核对精确匹配（面板 954×1170 = 424×520×2.25，右缘超出工作区 10 逻辑 px 为设计内透明边距）；其他 DPI 档位未实测（改系统缩放需重登录，扰动过大），按 Qt 逻辑坐标语义属安全。回归：构建 0 错 0 警，四项 `--test-*` 全过且与 rust 一致，单实例完好。
- **Phase 5 打包：已完成并验收**（2026-09-13）。`qt/package_release.py`（Python，UTF-8 安全）一键重建 Release + windeployqt 到 `qt/dist/`（已加入 `qt/.gitignore`）：`--release --no-translations --compiler-runtime --dir dist`。产出 28 文件 / 36.1 MB 目录形态（exe + Qt6Core/Gui/Network/Svg/Widgets + platforms/styles/imageformats/iconengines/networkinformation/tls 插件 + MSVC CRT）。验收：PATH 中完全剔除 Qt 目录后，dist exe 四项 `--test-*` 全过（fetch/update 与 rust 逐字段一致）；GUI 实测托盘 toggle 面板开合正常（真实 HTTPS 数据，schannel TLS 插件生效）。已踩坑：① `--compiler-runtime` 在 VS Build Tools 布局下找不到 CRT（无 VCINSTALLDIR），且 VS 2026 的 CRT 目录叫 `Microsoft.VC145.CRT`（不是 VC143）——脚本里按 `*/x64/Microsoft.VC14*.CRT` 手动拷贝；② PNG 支持内建于 Qt6Gui，无需 qpng 插件；③ 托盘溢出浮层照旧延迟开合，UIA Invoke + 长轮询 + UIA 坐标点击的组合最稳。版本：parity 已达，qt 版按 QT-MIGRATION 第 6 章并入统一版本——`qt/CMakeLists.txt` `project(VERSION 1.7.2)` + `main.cpp` `setApplicationVersion("1.7.2")`（应用内无版本展示位，仅为 exe/框架元数据）。发布 zip（`KimiPlanbarTray-qt-v<x.y.z>.zip`）与根目录 `make_release_zip.py` 的集成留待用户确认发布策略后进行（根文件不在本轮权限内）。
- **Phase 0–5 全部完成**。Qt 版达到与 rust 1.7.2 的 SPEC parity 并可打包分发。
- **闪烁修复补充（2026-09-13 晚）**：Phase 4 的 QGraphicsDropShadowEffect 光晕在 `WA_TranslucentBackground` 无边框窗里 hover 时会闪——effect 的 enable/disable 切换渲染路径 + 每帧把子树重渲染成 pixmap。`windows/hoverglow.h` 已重写为**自绘光晕**：halo 是顶层窗的独立子 widget（目标控件矩形外扩 8/10px，同心圆角描边 alpha 由内向外递减，亮 18%/暗 22% 峰值），`WA_TransparentForMouseEvents` + 透明中心，完全不触碰目标控件自身的 QSS 绘制；动画只驱动一个 qreal progress 并 `update()`（140ms OutCubic，进出场同速，Enter/Leave 互相取代的守卫保留）。目标与 Phase 4 相同（面板 4 按钮 + 版本行、菜单项、Skills Refresh、设置 Save）。四窗 × 双主题 hover 截图复核无闪烁，dist 已重新打包，四项 `--test-*` 回归通过。
- **Code review 全量修复（2026-09-13 晚）**：15 项全修 + 4 项 parity 核查。修复：① http_util.h 删多余 `reply->deleteLater`（reply 随 nam 作用域销毁）+ body 4 MiB 截断；② quota.cpp `parseResetTime` 校验时区（±14h/分≤59 + QTimeZone::isValid，非法→nullopt，对齐 DateTimeOffset.TryParse）；③ async_util.h `ok` 改 `std::atomic<bool>`，QThread 改为 finished→自身 deleteLater + 投递走 context 绑定连接（修复退出路径泄漏）；④ app.cpp 加 `~App()` 删除四个无父窗（stack 析构在 QApplication 之前，安全）+ loadSkills 加 in-flight 守卫；⑤ main.cpp `printStdout` 只对自建的 CONOUT$ 句柄 CloseHandle；⑥ **skillswindow.cpp 外部数据纯文本**：name/分组标签 `setTextFormat(Qt::PlainText)`，tooltip 用 `toHtmlEscaped()`——恶意夹具（`<b>`/`<img onerror>`/`&amp;`）实测按字面渲染；⑦ hoverglow.h 事件过滤器挂整条祖先链，面板滑动动画中 halo 跟随按钮（中程截图实证）；⑧ panelwindow.cpp 捕获 `QEvent::ScreenChangeInternal` 按新 DPR 重渲图标+logo（跨 DPI 实测未做，代码路径已核）；⑨ titlebar.h 拖偏移量改 `std::optional<QPoint>`；⑩ CMakeLists 加 `resources.rc`（VERSIONINFO 对齐 rust exe：shawnqi/KimiPlanbarTray/1.7.2）+ Pillow 由 icon.png 生成 icon.ico 嵌入 exe 图标；⑪ package_release.py 三路径支持 KPT_QT_DIR/KPT_CMAKE/KPT_MSVC_REDIST 环境变量覆盖 + 打包后跑 dist `--test-skills` 冒烟（非零即失败）；⑫ settings.cpp 手写序列化器对 theme 做 JSON 转义。Parity 核查（读 rust 源码对齐）：A 手动刷新——rust `refresh_now` 同发 safe_refresh + check_and_emit，qt 已一致；B 非法 RefreshMinutes——rust 回填不选中任何 pill、保存回退 5，qt 已一致；C settings 写入——rust 是裸 `fs::write`（无 temp+rename），qt 保持裸写；D frontmatter 收尾 `---`——rust 为 trim_end 后整行 trim 比较，qt 一致。回归：构建 0/0，重打包 dist（含冒烟），四项 `--test-*` 与 rust 逐字节一致。

## 2. 环境（一次性投入，已完成）

| 组件 | 位置 / 版本 |
|---|---|
| Qt kit | `C:/Qt/6.9.3/msvc2022_64`（aqtinstall `--archives qtbase qtsvg` 精简安装；windeployqt 在 qtbase 内） |
| CMake | VS 生成工具内置 4.2.3：`C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe`（不在 PATH，使用时手动 export） |
| 编译器 | MSVC 14.50（VS Build Tools 2026，`C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools`） |
| Python | 3.13（aqtinstall 已装） |

构建与运行：

```bash
export PATH="/c/Program Files (x86)/Microsoft Visual Studio/18/BuildTools/Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin:$PATH"
cd qt
cmake -B build -G "Visual Studio 18 2026" -A x64 -DCMAKE_PREFIX_PATH=C:/Qt/6.9.3/msvc2022_64
cmake --build build --config Release

# 运行 qt exe 必须先加 Qt DLL 到 PATH（未 windeployqt 前）：
export PATH="/c/Qt/6.9.3/msvc2022_64/bin:$PATH"
./build/Release/kimi-planbar-tray.exe
```

## 3. 已建文件（全部在 qt/ 下）

- `qt/CMakeLists.txt` — `project(VERSION 0.1.0)`（qt 版独立版本号，parity 前不与 rust 同步）
- `qt/src/main.cpp` — Phase 0 stub：无边框透明空窗（`FramelessWindowHint | WindowStaysOnTopHint | Tool` + `WA_TranslucentBackground`），Phase 1 要重写为"CLI 解析 → `--test-*` 自检 → 互斥锁 → QApplication"的正式入口
- `qt/.gitignore` — 排除 `build/`

## 4. Phase 1 任务清单（明天从这里开始）

按 `QT-MIGRATION.md` 第 3、5 章，把 `rust/src-tauri/src/` 五个模块 1:1 平移到 `qt/src/`：

| Rust 源 | 目标文件 | 要点 |
|---|---|---|
| `credentials.rs` | `qt/src/credentials.h/.cpp` | 凭证链逐行平移；config.toml 手写逐行解析（Qt 无 TOML 库）；`KIMI_CODE_HOME` 覆盖；30s 过期余量；MatchProvider 三条件 |
| `quota.rs` | `qt/src/quota.h/.cpp` | QNetworkAccessManager + QJsonDocument；**JSON 数字按字符串建模**（先判 `isString()` 再 `isDouble()`）；`amountLeft` 1e-8 元 → `(raw + 500000) / 1000000` 分；`isEnabled=false` → 整卡 "Not activated" |
| `settings.rs` | `qt/src/settings.h/.cpp` | settings.json 同 schema 同路径；`portable.dat` 重定向；HKCU Run 用 `QSettings(NativeFormat)` |
| `update.rs` | `qt/src/update.h/.cpp` | QProcess 跑 `kimi --version`（5s 超时）；changelog `Range: bytes=0-4095` + GitHub API 兜底 |
| `state.rs` | `qt/src/state.h/.cpp` | Phase 1 只需保旧值缓存骨架 |
| `main.rs`/`lib.rs` 的 CLI 分支 | 重写 `qt/src/main.cpp` | 顺序：`--test-*` 分支 **先于** `CreateMutexW` 单实例（锁名 `KimiPlanbarTray.SingleInstance`，不得改名） |

`--test-fetch` / `--test-update` 输出必须与 rust 版**逐字段一致**（先跑 `./rust/src-tauri/target/release/kimi-planbar-tray.exe --test-fetch` 看输出形状）。GUI 子系统 exe 无控制台：test 分支开头 `AttachConsole(ATTACH_PARENT_PROCESS)` + `freopen("CONOUT$", "w", stdout)`，失败静默（QT-MIGRATION 4.6）。

验收：同机先后跑 rust / qt 两版 `--test-fetch` 与 `--test-update`，JSON 逐字段对比一致（时间戳/百分比若因秒级间隔漂移就重跑一次确认）。

## 5. 注意事项（已踩过 / 易踩的坑）

1. **进程名冲突**：qt 版与 rust 版 exe 同名 `kimi-planbar-tray.exe`。用户机器上常驻一个 rust 版托盘进程（开发期间不要杀它）。结束 qt 测试进程时务必用启动时捕获的 PID 或按完整路径匹配，**不要** `taskkill /IM kimi-planbar-tray.exe`。Git Bash 的 `kill` 对 `ps -W` 的 Windows PID 无效，要用 bash job 的 `$!`。
2. **qt exe 缺 DLL 会静默退出**：运行前必须 `export PATH="/c/Qt/6.9.3/msvc2022_64/bin:$PATH"`（Phase 5 windeployqt 后则不需要）。
3. Phase 1–2 期间 rust/ 若发新版，qt/ 功能基线以开始实现时的 rust **1.7.2** 为准，追赶差异放到 Phase 4。
4. 单实例锁是三版互斥的：qt 版 GUI 实例与 rust/wpf 版同时只能跑一个（但 `--test-*` 分支在锁之前，不受影响）。
5. 代码、注释、commit 用英文；与用户对话用中文。
6. 不要动 `rust/`、`wpf/`、`docs/`（除本交接文档与 QT-MIGRATION 状态节）及根目录文件；新代码只落在 `qt/`。

## 6. 后续 Phase 速览

- Phase 2 托盘+主面板：QSS 双主题（色值照抄 `rust/src/theme.css` / SPEC 11）、圆角必须做在子容器上（QSS border-radius 对顶层窗不可靠）、托盘图标生命周期内只 `show()` 一次（Qt on Windows 已知 bug，风险 7.4）。
- Phase 3 其余三窗；Phase 4 SPEC 10–21 逐章 parity + 截图对比 + 多 DPI 验证；Phase 5 windeployqt 打包（目录形态，动态链接守 LGPL）。
- hover-to-refresh（SPEC 14）Qt 无直接信号：先试 nativeEventFilter（方案 A），超预算则放弃并在 SPEC 14 补注行为偏差（风险 7.3）。

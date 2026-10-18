# egui 版「开窗闪白色原生标题栏」根因与修复规格

> 文档性质：实施规格（What / Why / How），供后续 agent 直接执行。
> 当前状态：**未实施**。egui 版自 2026-10-17 起处于冻结状态，落地前需用户解冻。
> 适用基线：HEAD `3028e7a`，egui 版 1.7.3，eframe `=0.36.2`，winit `0.30.13`，`windows` crate `0.62`。
> 验证环境：Windows 11 26H2 build 26300，主屏 2880x1800 @225%，副屏 3456x1944 @(5184,704)。

---

## 0. TL;DR

1. **What**：打开任意窗口（面板 / 设置 / 技能 / 托盘菜单）瞬间，窗口顶部闪出整条**原生白色标题栏**（含标题文字与最小化/最大化/关闭按钮），随后消失。它不是 egui 画的，是 Win32 非客户区（NC）的 caption。
2. **Why**：winit 为了保留 DWM 阴影，始终给无装饰窗口保留 `WS_CAPTION`；26H2 上 DWM 会把这条 caption 渲染成整条白框，所以代码用 `DWMNCRP_DISABLED` + NC 折叠把它关掉。DWM 不画之后，`DefWindowProc` 在 `WM_NCACTIVATE` 里改用 **GDI 自绘 caption**，画进窗口重定向表面，下一次 present 才被覆盖 → 正好一帧白条。现有补法是在转发 `WM_NCACTIVATE` 期间临时剥掉 `WS_CAPTION`，但它只覆盖"激活"这一条路径，且样式化依赖 200ms 轮询 → 首秀与其它 caption 绘制消息仍会漏。
3. **How**：分两层加固——(A) 在 **HWND 创建时**（`WH_CBT` 钩子）+ `WM_SHOWWINDOW` 同步完成样式化，消除 200ms 轮询竞态；(B) 让 caption **在任何时刻都画不出来**（吞 `WM_NCUAHDRAWCAPTION/FRAME`，并在 `WM_STYLECHANGED` 里把 `WS_CAPTION` 永久剥掉，保留现有 `WM_NCACTIVATE` strip 作兜底）。(C) 可选：样式化后强制一次 present 擦掉可能残留的 GDI 画面。

**重要前提**：本次排查实测 HEAD release 构建**不复现**该现象，而 2026-10-17 19:14 的 debug 二进制（早于修复提交 `85e106c`）**稳定复现**。实施前先按 §4.1 用阳性对照自证检测手段有效，再谈"修复后不闪"。

---

## 1. What（现象与判定）

### 1.1 视觉表现

- 窗口顶部出现一整条 **原生标题栏**：左侧窗口图标 + 标题文字（如 `Kimi Planbar Tray Main Panel`），右侧最小化 / 最大化 / 关闭按钮。
- 出现时长：多数是 **1 帧**（约 16ms，肉眼为"闪一下"）；若窗口上屏后没有下一次 present 排程，会滞留到下一次重绘（归档里记录过 3.5–7 秒的"首秀残留"）。
- 颜色随系统主题：亮色主题下是白色/浅蓝条，暗色主题下是深蓝条 + 橙色关闭按钮。用户描述的"外层一圈框"通常与它同时出现——那是同一次未样式化状态下 DWM 画的窗口边框，见 §2.6。

### 1.2 触发面

- 四个窗口全覆盖：主面板（root viewport）、设置、技能、托盘右键菜单。
- 两条路径都会触发：**首次 show**（窗口从隐藏变可见）与**激活/失焦**（`WM_NCACTIVATE`）。
- 旧构建上表现更强：每次打开、每个窗口都出现。

### 1.3 判定标准（可复现 / 可验收）

不要用"肉眼看"当判据，用**窗口顶部带状区域的高速连拍**：

- 取样区域：`kpt-windows.json` 给出的窗口物理矩形，取 **顶部 28 物理像素 × 全宽**（225% 下 28px ≈ 12 逻辑像素，正好覆盖 caption 上沿且基本不含卡片内容）。
- 指标：每帧该带的 `bright%`（灰度 ≥200 的像素占比）。
- 阳性：单帧 `bright% ≥ 80`（实测旧构建为 **90.3%**）。
- 基线：HEAD release 构建实测峰值 **2.56%**（5 轮开关，每轮约 120 帧 + 设置窗 176 帧）。
- 因此验收阈值取 **bright% < 5%**，并辅以"单帧平均灰度跳变 ≤ 0.15"。

### 1.4 现状（实施前必须知道）

| 构建 | 场景 | 实测结果 |
|---|---|---|
| HEAD release（本次重建） | `--test-ui` 四窗首开，137 帧 | 无亮帧（bright% ≤ 0.75） |
| HEAD release | 托盘左键开面板 ×5 轮，**77 fps** 顶部带连拍 | 峰值 2.56%，无 caption 帧；并已确认面板确实成为 foreground（排除"没激活所以不闪"） |
| HEAD release | 从面板打开设置窗，176 帧 | 无 caption 帧 |
| **2026-10-17 19:14 debug 构建** | `--test-ui`，390 帧 | 682–960ms 持续出现深色原生 caption；**971ms 单帧 bright% = 90.3** 的白色 caption |

结论：
- 修复代码已在 HEAD 里；用户看到的现象来自**修复前构建**（最可能是 `egui/target/debug` 的旧二进制）。
- 但 HEAD 仍存在一条**首秀竞态**（§2.4 P1），属于"偶发/时序相关"，必须一并堵掉，否则换机器、换构建仍会复现。

---

## 2. Why（根因机制）

### 2.1 机制链（源码级）

1. 四个 viewport 都是 `with_decorations(false)` + `with_transparent(true)` + 置顶 + 不占任务栏。
   见 `egui/src/main.rs:181-193`（root）与 `egui/src/app.rs:583-596`（settings / skills / menu）。
2. egui-winit 把 `decorations=false` 翻译成 winit 的 `with_undecorated_shadow(true)`
   （`egui-winit-0.36.2/src/lib.rs:2195-2197`），目的是让无装饰窗口仍能拿到 DWM 投影。
3. winit 0.30.13 因此**始终保留** `WS_CAPTION | WS_BORDER`
   （`winit-0.30.13/src/platform_impl/windows/window_state.rs:258`），只靠 `WM_NCCALCSIZE` 里
   `rgrc[0].top += 1; rgrc[0].bottom += 1`（`event_loop.rs:1182-1196`）把非客户区藏起来。
4. Win11 26H2（build 26300+）把这条 retained caption 交给 DWM 渲染，直接画成**整条白色标题栏 + 关闭按钮**，盖在透明内容上。
5. 现有修复：`DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED`（让 DWM 不渲染 NC）+ wndproc 子类化把 NC 区完全折叠
   （`egui/src/windows.rs:275-334` 的 `style_hwnd`、`210-271` 的 `nc_collapse_proc`；`WM_NCCALCSIZE` 返回 0、`WM_NCPAINT` 吞掉）。
6. DWM 不画之后，**`DefWindowProc` 在 `WM_NCACTIVATE`（得焦/失焦）里改用 GDI 自己画 caption**。这条绘制不走 `WM_NCPAINT`（实测吞掉也没用），直接写进窗口的**重定向表面**（也就是 GL surface 的位图）。
7. GDI 画上去之后，只有下一次 `present` 才会覆盖它 → **1 帧白色标题栏**。若之后没有重绘排程，就一直停留（"首秀残留"）。
8. `WM_NCACTIVATE` 不能吞：winit 用它设置 `is_active`
   （`winit-0.30.13/src/platform_impl/windows/event_loop.rs:2132-2143`，末尾 `ProcResult::DefWindowProc`），
   而 egui 的 `Focused(true)` 与面板自动隐藏的 `panel_was_focused` latch 都依赖它。
9. 现有补法：转发 `WM_NCACTIVATE` 期间临时 `SetWindowLongPtrW(GWL_STYLE, style & !WS_CAPTION)`，转发返回后恢复
   （`egui/src/windows.rs:261-269`）。`DefWindowProc` 找不到 caption 可画 → 不闪。
10. 样式化由 `spawn_watcher` **每 200ms 轮询**完成（`egui/src/windows.rs:343-367`），按"HWND 出现"和"hidden→visible"触发。
    这就是所有时序漏洞的根源。

### 2.2 为什么不能直接吞掉 `WM_NCACTIVATE`

- winit 的 `WindowState::set_active` 只在这个消息里更新；吞掉后 `has_active_focus` 永远为假。
- 后果：egui 收不到 `Focused(true)` → `panel_was_focused` 永假 → 失焦自动隐藏失效（SPEC 15 回归）。
- 所以只能"转发 + 在转发期间制造一个没有 caption 的样式假象"，或者"让窗口根本没有 caption"。

### 2.3 现有防护覆盖到哪一步

| 路径 | 现状 |
|---|---|
| 26H2 持续白框（DWM 渲染） | 已覆盖：`DWMNCRP_DISABLED` + NC 折叠 |
| 激活瞬间 GDI caption（`WM_NCACTIVATE`） | 已覆盖：转发期间临时剥 `WS_CAPTION` |
| 首秀 show 时尚未样式化 | **未覆盖**：靠 200ms 轮询，存在竞态 |
| `WM_NCACTIVATE` 之外的 caption 绘制消息 | **未覆盖**：`0xAE/0xAF`、`WM_STYLECHANGED`、`WM_SETTEXT`、主题切换全部原样转发 |

### 2.4 仍然会闪的四条路径

**P1 首秀竞态（HEAD 仍存在，时序相关）**
HWND 创建后 **<200ms** 内被 show → 样式（DWM 属性 + 子类）还没装 → 26H2 直接画出整条原生 caption。
覆盖场景：进程冷启动后立刻点托盘；`--test-ui` 四窗齐开；任何"创建即显示"的窗口。
若上屏后没有下一次 present，caption 会滞留（归档里的 3.5–7s 残留就是这个）。
19:14 debug 构建的 682–960ms 持续 caption 帧即为实证。

**P2 激活闪帧（旧构建实测复现，HEAD 靠 strip 覆盖）**
样式装好后，`WM_NCACTIVATE` 触发的 GDI caption。现有 strip 只在转发那一刻有效，
且依赖 `DefWindowProc` 同步绘制；一旦绘制被推迟到样式恢复之后，守卫就漏。

**P3 其它 caption 绘制消息（未覆盖）**
`WM_NCUAHDRAWCAPTION (0x00AE)`、`WM_NCUAHDRAWFRAME (0x00AF)`；winit 每次可见性翻转都会
`SetWindowLongW(GWL_STYLE, ...)`（`window_state.rs:398-424`），随后 `SWP_FRAMECHANGED`；
以及 `WM_SETTEXT`、主题切换。子类目前把这些全部原样转发给 winit / `DefWindowProc`。

**P4 新 HWND（未覆盖）**
viewport 重建、换构建、换机器都会让 HWND 重新走一遍 P1。watcher 按标题找窗口，失败即跳过。

### 2.5 实测证据（2026-10-18，本机）

- HEAD release：见 §1.4 的三行数据；`GetForegroundWindow` 校验证明面板确实拿到了前台。
- 19:14 debug 构建：`--test-ui` 连拍抓到
  - `d_0068`（848ms）等连续帧：深色激活态原生 caption；
  - `d_0079`（971ms）：**单帧白色 caption，bright% = 90.3**；
  - 之后 310 帧全部干净（watcher 完成样式化）。
- 该 debug 构建缺少 `WM_NCACTIVATE` strip（提交 `85e106c` 之前的代码），所以 P1 与 P2 同时可见。

### 2.6 「一圈框」与「白标题栏」的关系

窗口处于未样式化状态时，26H2 会**同时**画出 DWM 窗口边框（外圈 1–2px 细线）与顶部 caption。
caption 面积最大、最显眼，所以用户先注意到"白色标题栏"；两者是同一根因族（NC 渲染未关断 / caption 未剥）。
修复只需针对 caption 与 NC 渲染，不需要单独处理边框。

---

## 3. How（修复规格）

> 全部改动集中在 `egui/src/windows.rs`，外加少量调用点（`egui/src/main.rs`、`egui/src/app.rs`、`egui/src/panel_pos.rs`）。
> 不新增依赖、不升级 eframe/winit、不改 UI 行为。

### 3.0 总体策略与实施顺序

1. **先做 B**（让 caption 画不出来）：单独就能消掉 P2/P3，改动小、风险低。
2. **再做 A**（创建时样式化）：消掉 P1/P4。
3. C 只在 V1 仍观察到残留时启用。

建议每步单独构建 + 连拍验收，便于定位回归。

### 3.1 A. 消除首秀竞态（创建时样式化）

**A1. 线程级 `WH_CBT` 钩子（核心）**

- 位置：`egui/src/main.rs::run_app`（`:135`），在 `eframe::run_native` 之前安装。
- 必须安装在**创建窗口的线程**（main / UI 线程）；hook 句柄存 `static`，进程生命周期内不 unhook。
- 在 `HCBT_CREATEWND` 里用 `CBT_CREATEWNDW.lpcs->lpszName` 匹配四个已知标题，命中即调用
  "按 HWND 样式化"的版本（见 A3 的重构），**不等下一次轮询**。

```rust
// egui/src/windows.rs
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, CBT_CREATEWNDW, HCBT_CREATEWND, WH_CBT,
};

static CBT_HOOK: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

unsafe extern "system" fn cbt_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HCBT_CREATEWND as i32 {
        let hwnd = HWND(wparam.0 as *mut core::ffi::c_void);
        let cbt = &*(lparam.0 as *const CBT_CREATEWNDW);
        if !cbt.lpcs.is_null() {
            let name = (*cbt.lpcs).lpszName;
            if title_matches_any(name) {
                style_hwnd_hwnd(hwnd); // A3 拆分出来的按 HWND 版本
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

pub fn install_cbt_hook() {
    use windows::Win32::System::Threading::GetCurrentThreadId;
    let hook = unsafe {
        SetWindowsHookExW(WH_CBT, Some(cbt_proc), None, GetCurrentThreadId())
    };
    if let Ok(h) = hook {
        CBT_HOOK.store(h.0 as isize, std::sync::atomic::Ordering::SeqCst);
    }
}
```

要点：
- `title_matches_any` 用 `PCWSTR::as_wide()`（windows crate 里 `CREATESTRUCTW.lpszName` 是 `PCWSTR`）
  取 `&[u16]` 与四个标题常量逐一比较（含结尾 NUL 语义按实际实现）。
- 该钩子只影响本线程创建窗口，不会拦截其它进程。
- 在 `HCBT_CREATEWND` 阶段窗口"已创建、尚未显示/未发 `WM_CREATE`"，装 DWM 属性 + 子类化是安全的。

**A2. `WM_SHOWWINDOW` 兜底（窗口尚未上屏）**

在 `nc_collapse_proc` 增加分支：`WM_SHOWWINDOW (0x0018)` 且 `wparam != 0` 时，重新应用
`DWMWA_NCRENDERING_POLICY = DWMNCRP_DISABLED` 并把 `WS_CAPTION` 剥掉，再转发。
这一消息在窗口真正可见**之前**发出，因此它覆盖"创建时钩子没命中 / 属性丢失 / 新 HWND 重建"的情况。

**A3. 样式化函数拆分 + 已知调用点**

- 把 `style_hwnd(title)`（`egui/src/windows.rs:275`）拆成：
  - `style_hwnd_hwnd(hwnd: HWND)`：真正做 DWM 属性 + `WS_EX_TOOLWINDOW` + 子类化 + `SWP_FRAMECHANGED`；
  - `style_hwnd(title)`：`find_hwnd(title)` 后转调，保持现有调用方不变。
- 同步调用点（幂等、微秒级）：
  - `App::new`（`egui/src/app.rs:216`）：对 root 调一次 `style_hwnd(MAIN_TITLE)`；
  - `show_panel`（`egui/src/panel_pos.rs:207`）在 `Visible(true)`（`:220`）之前；
  - `show_centered`（`:250`）在 `Visible(true)`（`:274`）之前，按 viewport 选 settings/skills 标题；
  - `show_menu_at`（`:362`）在 `Visible(true)`（`:389`）之前。
- **保留 watcher**（`egui/src/windows.rs:343-367`）：winit 每次可见性翻转会把
  `GWL_STYLE / GWL_EXSTYLE` 按自己的 flag 集重写一遍（`WS_EX_TOOLWINDOW` 被抹掉），需要它回补；
  DWM 属性与 wndproc 子类不会丢。

**A′ 兜底方案**（若 CBT 钩子在当前 windows crate 版本上不可靠）：
不装钩子，改为 show 路径同步样式化 + **首 3 秒 10ms、之后 200ms** 的 watcher 轮询。
可以消掉绝大多数首秀竞态，但不是硬保证；只在 A1 验证失败时使用，并在代码注释里写明原因。

### 3.2 B. 让 caption 永远画不出来

**B1. 吞掉未文档化的 caption 绘制消息**

`nc_collapse_proc` 增加：

```rust
const WM_NCUAHDRAWCAPTION: u32 = 0x00AE;
const WM_NCUAHDRAWFRAME: u32 = 0x00AF;
if msg == WM_NCUAHDRAWCAPTION || msg == WM_NCUAHDRAWFRAME {
    return LRESULT(0);
}
```

这两条是 Windows 主题/边框绘制内部用的未文档化消息；即使某些版本不走 SendMessage（吞掉无效），也无副作用。

**B2. `WM_STYLECHANGED` 里永久剥 `WS_CAPTION`（主修）**

winit 每次可见性翻转都会重写 `GWL_STYLE`（`window_state.rs:398-408`），把 `WS_CAPTION` 加回来。
在子类里同步剥掉，保证窗口**任何可见时刻都没有 caption 位**：

```rust
const WM_STYLECHANGED: u32 = 0x007D;

if msg == WM_STYLECHANGED && lparam.0 != 0 {
    let ss = &*(lparam.0 as *const windows::Win32::UI::WindowsAndMessaging::STYLESTRUCT);
    if ss.styleNew & WS_CAPTION.0 != 0 {
        let cur = GetWindowLongPtrW(hwnd, GWL_STYLE);
        SetWindowLongPtrW(hwnd, GWL_STYLE, cur & !(WS_CAPTION.0 as isize));
    }
    // 继续转发；不发 SWP_FRAMECHANGED（NC 区已折叠，无需重算，避免闪烁）
}
```

要点：
- 递归安全：第二次 `WM_STYLECHANGED` 的 `styleNew` 已不含 `WS_CAPTION`，条件不成立。
- winit 明确不处理 `WM_STYLECHANGING` / `WM_STYLECHANGED`（归档 handoff 已核实），不会打架。
- 不发 `SWP_FRAMECHANGED`：避免触发 NC 重算与额外重绘。
- 一旦 `WS_CAPTION` 长期缺席，`DefWindowProc` 无论从哪条消息进来都**没有 caption 可画**，P2/P3 一并消失。

**B3. 保留现有兜底**

- `WM_NCACTIVATE` 的"转发期间临时剥 + 恢复"（`egui/src/windows.rs:261-269`）**保留**：
  它覆盖"子类刚装上、B2 还没机会生效"的第一次激活。
- `WM_NCPAINT` 吞掉、`WM_NCCALCSIZE` 返回 0 **保留**。

### 3.3 C.（可选）强制一次 present 擦掉残留

若 V1 仍观察到残留帧：在 `style_hwnd_hwnd` 完成后追加
`InvalidateRect(hwnd, None, false)`，并通过 `egui::Context::request_repaint_of(vp)` 立即排一次绘制，
把可能已经画进重定向表面的 GDI 内容覆盖掉。
这是保险措施，不是主修；启用前先确认 V1 确实还有残留，否则只会增加无谓重绘。

### 3.4 改动清单

| 文件 | 改动 |
|---|---|
| `egui/src/windows.rs` | `nc_collapse_proc`：新增 `WM_SHOWWINDOW` / `WM_NCUAHDRAWCAPTION` / `WM_NCUAHDRAWFRAME` / `WM_STYLECHANGED` 分支；`style_hwnd` 拆出 `style_hwnd_hwnd`；新增 `install_cbt_hook` + `cbt_proc`；`spawn_watcher` 保持不变 |
| `egui/src/main.rs` | `run_app` 内、`run_native` 之前调用 `windows::install_cbt_hook()` |
| `egui/src/app.rs` | `App::new` 末尾对 root 同步 `style_hwnd(MAIN_TITLE)` |
| `egui/src/panel_pos.rs` | 三条 show 路径在 `Visible(true)` 之前同步 `style_hwnd(title)` |

不新增依赖；不修改 `Cargo.toml` 的版本锁定。

### 3.5 副作用与兼容性

- **DWM 阴影彻底消失**：自 `DWMNCRP_DISABLED` 起本来就没有；egui 按 SPEC 10 自绘 shadow，是唯一视觉来源。
  验收时用 `docs/screenshot-*.png` 对照阴影/圆角。
- **aero-snap 不受影响**：窗口 `resizable=false` 且 `WS_EX_TOOLWINDOW`，本来就不参与贴靠。
- **尺寸数学不变**：winit 的 `adjust_rect`（`window_state.rs:428-452`）本来就对无装饰窗口剥掉
  `WS_CAPTION | WS_SIZEBOX` 再算外框，外层尺寸与 `kpt-windows.json` 不应变化。
- **任务栏**：`WS_EX_TOOLWINDOW` 仍由 watcher 在每次可见性翻转后回补，行为不变。
- **焦点/自动隐藏**：`WM_NCACTIVATE` 仍然完整转发给 winit，`panel_was_focused` 语义不变。

### 3.6 明确不要做的事

- 不要吞掉或改写 `WM_NCACTIVATE` 的转发（会破坏焦点 latch 与自动隐藏）。
- 不要升级/替换/fork winit、eframe（版本是刻意锁定的）。
- 不要用 `DWMWA_BORDER_COLOR = COLOR_NONE` 当主修（上一轮尝试只去掉四边 hairline，顶部 1px 仍在）。
- 不要只改 `WM_NCACTIVATE` 的 strip 就收工（那正是当前 HEAD 的状态）。
- 不要依赖"把 watcher 调快"当根治（P2/P3 与轮询无关）。
- 不要动 UI 布局、颜色、动画参数（SPEC 10/15/21 是契约）。

---

## 4. 验收

### 4.1 V0：先证明检测手段有效（强制）

用**已知有 bug 的构建**做阳性对照，例如 `egui/target/debug/kimi-planbar-tray-egui.exe`
（2026-10-17 19:14 构建，早于修复提交；若已被覆盖，可临时回退 `windows.rs` 的
`WM_NCACTIVATE` 分支重建一个）。用它跑 `--test-ui` 并按 §1.3 取样：

- 必须抓到至少 1 帧 `bright% ≥ 80`（实测 90.3）。
- 若抓不到，先修检测手段（帧率、取样带、DPI 坐标），**不要**进入下一步。

### 4.2 V1：视觉矩阵（每项 ≥300 帧）

| # | 场景 | 目的 |
|---|---|---|
| 1 | 冷启动后 **<300ms** 立即点托盘开面板 | 专打 P1 |
| 2 | 面板开 → 关 → 开，共 5 轮 | 打 P2（每次激活） |
| 3 | 从面板打开设置 / 技能（各自本实例首开） | 打 P4 |
| 4 | 托盘右键菜单首开 | 打 P4 |
| 5 | `--test-ui` 四窗齐开 | 打"创建即显示" |
| 6 | 主屏 225% + 把窗口拖到副屏（3456x1944@(5184,704)）各一遍 | 混合 DPI |

判据：
- 每个场景 `bright%` 峰值 **< 5%**（HEAD 基线 2.56）；
- 无单帧平均灰度跳变 > 0.15；
- 顶部 28px 带逐帧无内容变化（可再用帧间 diff 复核）。

### 4.3 V2：非视觉回归

- `panel_was_focused` 自动隐藏：面板聚焦后点其它窗口，约 2s 内隐藏。
- `spawn_pointer_rearm`：重显示窗口后不移动鼠标也能点按钮。
- `kpt-windows.json` 的四窗坐标/尺寸与改动前一致（±1px）。
- `cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`、`cargo test`（现有 9 个）全过。
- `--test-ui` 打印四行 `OK`，约 6s 退出。
- 深浅主题各截一张窗口边缘图，与 `docs/*.png` 对照，确认圆角/阴影无回归。

### 4.4 命令清单（Windows / PowerShell 7）

```powershell
# ---- 在 egui/ 目录执行：构建与静态检查 ----
cd egui
cargo build --release --locked
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test

# 自检（四行 OK）
.\target\release\kimi-planbar-tray-egui.exe --test-ui
```

```powershell
# ---- 回到仓库根目录执行：高速连拍 ----
# 坐标/尺寸以 kpt-windows.json 与 find_tray_icon.ps1 的当前输出为准；
# 下面这组是 225% 单主屏的参考值（窗口 rect 1948,544,954,1170；托盘中心 1920,1746）
pwsh -NoProfile -File scripts\diagnostics\burst_capture.ps1 `
  -Left 1948 -Top 544 -Width 954 -Height 28 -DurationMs 1600 `
  -OutDir "$env:TEMP\kpt-fix-v1" -Prefix f `
  -ClickX 1920 -ClickY 1746 -ClickAtMs 400
```

逐帧分析（放在任意临时 .py 里执行，需要 Pillow；不要在仓库里落盘）：

```python
import os
from pathlib import Path
import numpy as np
from PIL import Image

d = Path(os.path.expandvars(r"%TEMP%\kpt-fix-v1"))  # 改成实际输出目录
for f in sorted(d.glob("f_*.png")):
    a = np.asarray(Image.open(f).convert("RGB")).astype(int)
    band = a[:28].mean(axis=2)
    b = 100.0 * (band >= 200).mean()
    if b > 5:
        print("FAIL", f.name, round(b, 1))
print("done")
```

### 4.5 通过标准（一句话）

阳性对照能抓到 ≥80% 亮帧；修复后 §4.2 六个场景全部 <5%；§4.3 全部通过；仓库无新增依赖与无关注释。

---

## 5. 风险与回滚

| 风险 | 说明 | 处理 |
|---|---|---|
| CBT 钩子不触发 | 必须装在创建窗口的同一线程（main）；装在别的线程完全无效 | A′ 兜底方案；若仍不行，改用"首 3 秒 10ms 轮询 + show 前同步样式化" |
| 永久剥 `WS_CAPTION` 的视觉退化 | DWM 不再提供阴影；当前已无阴影，egui 自绘 shadow 覆盖 | 对照 `docs/*.png` 验收；若出现异常，回滚 `windows.rs` 即可 |
| `WM_STYLECHANGED` 与未来 winit 版本冲突 | 当前 winit 明确忽略该消息；未来若处理，可能出现样式抖动 | 用 `styleNew & WS_CAPTION` 位判断做最小干预；升级 winit 时重新回归 |
| `0xAE/0xAF` 未文档化 | 某些版本可能只是内部调用 | 吞掉无副作用；主修是 B2，不依赖它 |
| 误伤其它窗口 | 钩子按标题匹配，只处理四个已知窗口 | 标题常量集中在 `egui/src/windows.rs`，新增窗口必须同步 |
| 回滚 | 改动集中在 4 个文件 | `git revert` 对应提交即可；二进制无持久化状态 |

---

## 6. 附录

### 6.1 源码索引

| 位置 | 内容 |
|---|---|
| `egui/src/windows.rs:210-271` | `nc_collapse_proc`：`WM_NCCALCSIZE`→0、`WM_NCPAINT`→0、`WM_NCACTIVATE` 临时剥 caption、其余转发 |
| `egui/src/windows.rs:261-269` | 现有激活闪帧修复（strip 只在转发期间生效） |
| `egui/src/windows.rs:275-334` | `style_hwnd`：`DWMNCRP_DISABLED`、`WS_EX_TOOLWINDOW`、子类化（`KPT_NC_SUBCLASSED` 标记）、`SWP_FRAMECHANGED` |
| `egui/src/windows.rs:343-367` | `spawn_watcher`：200ms 轮询，HWND 出现 / hidden→visible 时样式化 |
| `egui/src/panel_pos.rs:207-240` | `show_panel`：`InnerSize` → `OuterPosition` → `Visible(true)` → `Focus` |
| `egui/src/panel_pos.rs:250-301` | `show_centered`（settings / skills） |
| `egui/src/panel_pos.rs:362-405` | `show_menu_at`（托盘菜单） |
| `egui/src/app.rs:216-260` | `App::new`（CBT 之外的 root 同步样式化落点） |
| `egui/src/app.rs:583-596` | 三个 deferred viewport 的 builder（`decorations=false` + `transparent`） |
| `egui/src/app.rs:601-638` | `register_viewports`（每个 root pass 重新注册） |
| `egui/src/app.rs:651-677` | 首帧：`eframe` 强制 show 后立即 `Visible(false)`；`--test-ui` 四窗齐开 |
| `egui/src/main.rs:135-217` | `run_app` 与 `NativeOptions`；CBT 钩子安装点 |
| `winit-0.30.13/src/platform_impl/windows/window_state.rs:258` | 无装饰窗口仍保留 `WS_CAPTION \| WS_BORDER` |
| `winit-0.30.13/src/platform_impl/windows/window_state.rs:398-424` | 每次可见性翻转重写 `GWL_STYLE/GWL_EXSTYLE` + `SWP_FRAMECHANGED` |
| `winit-0.30.13/src/platform_impl/windows/event_loop.rs:1182-1196` | `MARKER_UNDECORATED_SHADOW` 的 `top+=1/bottom+=1` |
| `winit-0.30.13/src/platform_impl/windows/event_loop.rs:2132-2143` | `WM_NCACTIVATE` → `set_active`，末尾 `DefWindowProc` |
| `egui-winit-0.36.2/src/lib.rs:2195-2197` | `decorations=false` → `with_undecorated_shadow(true)` |

### 6.2 证据产物

本次排查的连拍帧保存在 `%TEMP%`（可复现，也可删除）：

- `kpt-burst-a1`：HEAD release `--test-ui` 首开，面板区 137 帧；
- `kpt-burst-b1`：HEAD release 托盘点击开面板，面板区 105 帧；
- `kpt-flash-seq\run1..run5`：HEAD release 五轮开关，顶部 28px 带，77fps；
- `kpt-debug-band`：19:14 debug 构建 `--test-ui`，390 帧（含 `d_0079_971ms.png` 白色 caption 帧）。

### 6.3 术语

- **NC（non-client area）**：标题栏、边框等由系统绘制、不属于客户区的部分。
- **DWM / DWMNCRP**：桌面窗口管理器；`DWMNCRP_DISABLED` 表示不让 DWM 渲染非客户区。
- **GDI caption**：`DefWindowProc` 在 DWM 不画时用 GDI 直接画到窗口 DC 的标题栏。
- **redirection surface**：DWM 为每个窗口维护的合成位图；GDI 画上去的内容要等下一次 present 才被覆盖。
- **CBT hook**：`WH_CBT` 线程钩子，可在窗口创建（`HCBT_CREATEWND`）等时刻同步介入。

### 6.4 相关文档

- `docs/archive/HANDOFF-egui-26H2-ncframe.md`：26H2 白框/闪帧修复的完整 arc（含 bug #1/#2、回归结果、残留）。
- `docs/archive/EGUI-MIGRATION.md` §7：DWM hairline「半透明框」的定案与最终消除。
- `AGENTS.md` 的「egui edition specifics」段：现有陷阱清单（冻结状态、`WM_NCACTIVATE`、首秀残留）。
- `docs/SPEC.md` §10/§20：窗口尺寸、阴影、定位与 DPI 契约（验收对照）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)

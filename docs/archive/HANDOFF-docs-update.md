# HANDOFF — 文档更新任务（qt 版收官后的文档收口）

> 2026-09-13 晚。qt/ 版开发已全部结束，本文档是给"文档更新"这一收尾任务的简短交接。
> 完整开发历史见 `docs/archive/HANDOFF-qt.md` 与 `docs/archive/QT-MIGRATION.md`（均已归档）。

## 1. 当前状态（事实速查）

- **qt/ 版 v1.7.2 完成**：Phase 0–5 全通过 + 悬停闪烁修复 + code review 15 项修复（含 1 Medium：Skills 富文本注入面已堵）。版本号已加入统一同步列表（`qt/CMakeLists.txt` `project(VERSION 1.7.2)` + `main.cpp` setApplicationVersion）。
- **可分发产物**：`qt/dist/`（28 文件 / 36.2 MB，目录形态，动态链接守 LGPL）。打包：`PYTHONUTF8=1 python qt/package_release.py`（路径可用 `KPT_QT_DIR` / `KPT_CMAKE` / `KPT_MSVC_REDIST` 覆盖）。
- **常驻进程**：用户机器跑的是 rust 版托盘 v1.7.2。三版共用互斥锁 `KimiPlanbarTray.SingleInstance`，同时只能活一个。
- **环境**：Qt kit `C:/Qt/6.9.3/msvc2022_64`；cmake 用 VS 内置（`C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin\cmake.exe`，不在 PATH）；编译器 MSVC 14.50。
- **遗留决策点**：qt 版定位"实验性质"，用户计划上传 GitHub；发布 zip 是否纳入 `make_release_zip.py` 未定（用户决策）。

## 2. 文档更新任务清单

1. **AGENTS.md**
   - qt/ 段落从"PLANNED / in progress"改为"completed, experimental"，说明版本已入同步列表。
   - 补齐约束（现只散落在 HANDOFF-qt.md）：
     - 打包：`qt/package_release.py`；dist 为目录形态、无单 exe。
     - `--test-*` 输出机制：`WriteFile(GetStdHandle(STD_OUTPUT_HANDLE))`，AttachConsole+CONOUT$ 仅兜底（QT-MIGRATION 4.6 的旧说法已过时）。
     - 新踩坑：QGraphicsDropShadowEffect blur=0 启用会隐藏控件（光晕为自绘控件 hoverglow.h）；QPushButton 样式 sizeHint 塌缩 → `setMinimumHeight(45)`；Q_OBJECT 头文件必须列入 CMakeLists 源清单；构建前先按 PID 杀 qt GUI（exe 文件锁）。
     - 行为偏差定稿：托盘 hover 刷新 = 500ms 轮询 `QSystemTrayIcon::geometry()`（非事件驱动，10s 节流）；DPI 100% 未实测（225% 实测通过，其余由 PerMonitorV2 构造覆盖）。
2. **SPEC.md / SPEC_EN.md**：登记 qt 版偏差（上两条 + 光晕为自绘近似）。两文件章节号一致，改动需同步。
3. **README.md / README_CN.md**：增加 qt 版说明（实验性、`qt/dist/` 目录形态分发、与 rust/wpf 互斥、无签名 SmartScreen 提示）。
4. **归档**：已完成——`HANDOFF-qt.md` / `QT-MIGRATION.md` / `HANDOFF-code-review.md` 及本文档均已移入 `docs/archive/`，AGENTS.md / README / SPEC 中的引用路径已同步修正。
5. **QT-MIGRATION.md §9 状态节**已是最新（五阶段完成），归档即可，无需再改。

## 3. 纪律（沿用）

- 代码/注释/commit 用英文；文档中文（README.md 英文、README_CN.md 中文、SPEC.md 中文 + SPEC_EN.md 英文对照）。
- 归档用 `git mv` 保留历史；发布 zip 与 SHA256SUMS 不提交。
- wpf/ 保持冻结只读。

# HANDOFF — scripts/ 规整与 release 资产一致性

> 状态：**进行中**，未归档。§3 的待办全部落地后，按仓库惯例移入 `docs/archive/` 并冻结。
> 对应提交：`45a46a7`（脚本规整）、`674543a`（校验和范围修正）、`22d59ca`（本文件入库 + 版本清单修订为 9 处），均已在 `main` 并推送 GitHub。
> 入库状态：核验时（2026-09-21）仍为"待提交"的 `AGENTS.md` active-handoff 指针 + 版本清单改写与本文档，已随 `22d59ca` 提交；`origin/main` == 该提交。
> 编写日期：2026-09-21；最后核验：2026-09-21（结论与证据见 §2.1）。

## 0. 现状一句话

根目录 8 个脚本已收进 `scripts/{release,diagnostics}`；发布打包脚本的 `SHA256SUMS.txt` 现在只列真正会上传的资产。三个最新 release（v1.7.0 / v1.7.1 / v1.7.2）经逐文件比对确认与仓库一致，已发布资产按决定不做追溯修改。剩下的是 3 个已登记未修的小项，以及下次发版前的版本号 bump。

## 1. 已完成

### 1.1 脚本目录规整（`45a46a7`）

`git mv` 移动，历史保留：

```
scripts/release/       make_release_zip.py   make_screenshots.py   verify_icons.py
scripts/diagnostics/   csp_visual_check.ps1  dump_tray_windows.ps1  measure_run.ps1
                       inspect_window_dpi.ps1  analyze_wpf_shadow.py
```

- 三个 Python 脚本的仓库根解析改为 `Path(__file__).resolve().parents[2]` / `os.path.join(..., "..", "..")`（`scripts/release/` 比仓库根深两层；第一版少套了一层，被验证当场抓出）。
- `inspect_window_dpi.ps1`：写死的旧机器路径（`C:\Users\rexxa\...`）→ 必填参数 `-ExePath`。
- `analyze_wpf_shadow.py`：写死的旧机器临时路径 → 必填参数 `<png-path>`，缺参报 usage。
- `csp_visual_check.ps1`：默认截图输出 `$PSScriptRoot` → `$env:TEMP`，避免诊断脚本在仓库内堆未跟踪 png。
- 文档同步：`AGENTS.md`（版本同步清单、顶层文件清单、发布流程）、`docs/SPEC.md` 与 `docs/SPEC_EN.md`（各 5 处）、`qt/src/icons.h` 注释。
- 有意未动：`README.md` / `README_CN.md`（不含这 8 个脚本的引用）、`docs/archive/*`（冻结的历史快照，仍含旧的裸文件名）。

验证证据：`scripts/release/verify_icons.py` 完整运行 exit 0（4 项 SVG 逐字节比对全 OK）；`make_release_zip.py` 以占位二进制完整跑通（命令见 §4），zip 内 `scripts/...` 路径正确；`scripts/diagnostics/*.ps1` 四个文件经 PowerShell Parser 解析全部通过；`inspect_window_dpi.ps1` 缺 `-ExePath` 时拒绝执行，不会误启动 GUI。

### 1.2 SHA256SUMS.txt 覆盖范围修正（`674543a`）

- 问题：v1.6.0 起 release 只把 `KimiPlanbarTray-rust.exe` 作为独立资产上传（两个 WPF exe 只存在于 zip 内部），但校验和文件一直列 3 行 → 下载者看到两行指向不存在的文件。
- 改法：`BINARIES` 增加 `uploaded` 标志（WPF 两项 `False`、rust `True`）；`write_checksums()` 只列"标记为上传的二进制 + zip 自身"；docstring 同步。
- 改后预期输出：2 行 —— `KimiPlanbarTray-rust.exe`、`KimiPlanbarTray-v1.7.2.zip`（随 `VERSION` 变化）。
- 验收证据：哈希逐条重算一致（zip 行 == 磁盘 zip 真实 sha256，exe 行 == zip 内该成员解出后的 sha256）；zip 仍含 3 个二进制，且源码树与 `git ls-files --cached --others --exclude-standard` 集合完全一致；任一二进制缺失即 exit 1 且不产出半成品；标准 `sha256sum` 格式（两空格、LF、无 BOM、不自引用）。已按用户规范委派未参与编写的独立 agent 做证伪式复验（假设至少 2 处错误），4 条验收标准全部达成、未被证伪。

## 2. release 审计结论（2026-09-21 实查）

| 检查项 | v1.7.0 | v1.7.1 | v1.7.2 |
|---|---|---|---|
| tag 提交 | `9ac0198` | `ce6969f` | `df7d48c` |
| 在 main 历史里（距 `674543a`） | 是（7 个提交） | 是（5） | 是（4） |
| tag 内版本号自洽 | 1.7.0 | 1.7.1 | 1.7.2 |
| zip 内源码 vs tag 树 | 文件集 100% 一致；仅 30 处 CRLF 差异 | 仅 29 处 | 仅 48 处 |
| zip 资产摘要 vs GitHub 记录 | MATCH | MATCH | MATCH |
| 包内 exe sha256 vs 已发布校验和 | MATCH | MATCH | MATCH |

- CRLF 差异成因：`core.autocrlf=true`，zip 打包的是工作区文件（CRLF），git 内是 LF；归一化行尾后零差异。**已知设计后果，暂不处理**（要改需从 git blob 而非工作区取内容）。
- 版本号自洽的检查点当时有 5 处：`rust/package.json`、`rust/src-tauri/Cargo.toml`、`rust/src-tauri/tauri.conf.json`、`rust/src-tauri/Cargo.lock` 的 `kimi-planbar-tray` 条目、`make_release_zip.py`——`qt/` 是 v1.7.2 之后才加入的，所以这三个 tag 里没有 qt 的三处（`qt/CMakeLists.txt`、`qt/src/main.cpp`、`qt/resources.rc`）。`rust/package-lock.json` 不属于此列：它从未参与 bump，至今仍停在 1.6.0（见 §2.1）。
- v1.0.0–v1.4.0 无 `SHA256SUMS.txt` 资产；v1.5.0 是最后一个同时独立上传 WPF exe 的版本（三个哈希均 MATCH）。
- GitHub 上 v1.6.0 / v1.7.0 / v1.7.1 / v1.7.2 的 release 说明正文仍写着 "SHA256SUMS.txt covers all binaries"（v1.7.0 正文更写成 "Checksums for the three exes"），与 §1.2 修复后的口径（只列真正上传的资产）不一致。**已发布资产不追溯修改**，故仅记录为已知文案偏差；下次发版时的 release notes 按 `sha256sum -c` 的实际覆盖范围措辞。
- 决定：**已发布资产不追溯修改**（重传会改变摘要，破坏他人已下载的校验）。

### 2.1 核验记录（2026-09-21 复验）

对本文档的事实性陈述做了一次只读复验，证据分三类：

- **外部可查（GitHub Release API 交叉核对，全部成立）**：三个 tag 的提交号与距 `674543a` 的提交数（7 / 5 / 4）；三个 tag 内 5 处版本号自洽（含 `Cargo.lock` 的 `kimi-planbar-tray` 条目）且 tag 树内无 `qt/`；v1.6.0 资产 = rust exe + zip + `SHA256SUMS.txt`（无独立 WPF exe）；v1.5.0 = 3 个 exe + `SHA256SUMS.txt`（最后一个独立上传 WPF exe 的版本）；v1.4.0 = 3 个 exe、无 `SHA256SUMS.txt`（与 "v1.0.0–v1.4.0 无校验和资产" 一致）；已发布的 v1.7.2 `KimiPlanbarTray-rust.exe` 摘要 `deab4c11…` 与本地副本一致。
- **本地可查（仓库实测，全部成立）**：§1.1 的 8 处脚本位置、参数化改造与文档同步点（`docs/SPEC.md` / `docs/SPEC_EN.md` 各 5 处、`qt/src/icons.h` 注释、两份 README 无引用、`docs/archive/*` 仍含旧裸名）；`git grep -i rexxa` 零命中；§1.2 的 `uploaded` 标志与 `write_checksums()` 取用范围；§4 的三条占位路径确被 `.gitignore` 忽略（`git check-ignore`：根 `.gitignore:4` / `:5`，`rust/.gitignore:3`）；仓库内当前**不存在任何 `.pyc`**；F1 / F2 / F3 三项仍为未修状态。
- **独立证伪复验抓出的两处漏项（已修入本文档与 AGENTS/SPEC）**：① `rust/package-lock.json`（tracked）的根版本写在第 3 行与 `packages[""]` 的第 9 行，**至今仍是 1.6.0**——它因落后三个版本而躲过了 `git grep 1.7.2` 扫描，原先 8 处清单漏了它，现补为第 9 处；② `docs/SPEC.md` / `docs/SPEC_EN.md` §7.3 第 1 步仍是 6 处清单，与更新后的 AGENTS/本文档冲突，已同步为同一份 9 处清单。此外该复验确认 §2 "检查点只有 4 处" 表述不准（tag 内 5 处，含 Cargo.lock），已改正。
- **未能本地复核**：§2 表格的 "zip 内源码 vs tag 树 CRLF 差异 30 / 29 / 48 处"——本机无 zip 产物、也不重新下载（65 MB），故不重算；已发布 zip 的摘要与 GitHub 记录一致（§2 行），原结论保留。

## 3. 待办

### 3.1 下次发版前必做

1. **bump 版本号（9 处 + `AGENTS.md` 顶部一行）**：
   - `rust/package.json`、`rust/src-tauri/Cargo.toml`、`rust/src-tauri/tauri.conf.json`
   - `rust/src-tauri/Cargo.lock` 中 `name = "kimi-planbar-tray"` 条目的 `version`（cargo 会随 `Cargo.toml` 改写，但必须确认并提交——v1.7.0 曾是单独的 `chore: sync Cargo.lock` 提交 `9ac0198`）
   - `rust/package-lock.json` 的根版本（第 3 行与 `packages[""]` 的第 9 行；不是构建输入，但属版本元数据，**当前仍停在 1.6.0**——`cd rust && npm install` 会让 npm 自行同步）
   - `scripts/release/make_release_zip.py` 的 `VERSION`
   - `qt/CMakeLists.txt` 的 `project(VERSION ...)`、`qt/src/main.cpp` 的 `setApplicationVersion`
   - `qt/resources.rc`：数值元组 `FILEVERSION` / `PRODUCTVERSION`（`1, 7, 2, 0` 逗号形式）+ 字符串 `FileVersion` / `ProductVersion`（由 `qt/CMakeLists.txt` 编入 qt exe 的版本资源）
   - 外加 `AGENTS.md` 顶部的 "Current version" 行；并同步三处清单：`AGENTS.md` 的 Release process 第 1 步、`docs/SPEC.md` 与 `docs/SPEC_EN.md` 的 §7.3 第 1 步（2026-09-21 起与本节一致）
   - **不是** bump 目标：`wpf/KimiPlanbarTray.csproj` 的 `<Version>1.5.0</Version>`（WPF 版冻结在 v1.5.0，按设计不动）
   原因：HEAD（`674543a`）已比 v1.7.2 多 4 个提交，但版本号仍是 1.7.2；不 bump 会与既有同名资产冲突（GitHub 拒绝同名上传）。清单完整性由 §2.1 的独立证伪复验证实（首版 8 处漏了 `rust/package-lock.json`）。
2. 构建 rust release exe → `PYTHONUTF8=1 python scripts/release/make_release_zip.py` → 上传 zip + rust exe + `SHA256SUMS.txt` 到新 tag。
3. WPF exe 仅在"异常改动了 WPF 版"时重建上传（WPF 冻结在 v1.5.0）。

### 3.2 已登记、用户决定暂缓的三项

- **F2（低）zip 非原子写入**：`ZipFile(OUT, "w")` 会先截断同名文件，写入中途失败会留下"看起来像成品"的残件 + 上一轮校验和。建议先写 `OUT.part` 再 `os.replace`，校验和最后写。（既有问题）
- **F3（低）`.gitignore` 缺 `__pycache__/`**：任何 import 该脚本的工具留下的 `.pyc` 会被 `git ls-files --others` 收进 release zip（已实测会被打入）。当前仓库内无任何 `.pyc`（`git ls-files --cached --others --exclude-standard` 过滤 `__pycache__` / `.pyc` 为空），属预防性修复。（既有卫生问题）
- **F1（低）本地自校验不友好**：校验和用的是发布资产名，仓库根直接 `sha256sum -c` 只有 zip 行能过。二选一：① 文档说明"把资产按发布名放进同一目录再校验"；② 把 `KimiPlanbarTray-*.exe` 加进 `.gitignore` + `EXCLUDE_FILES`，允许在仓库根就地放置。**注意**：现在往仓库根拷裸 exe 会变成 untracked-not-ignored，被下一轮打包收进 zip，选②前不要这么做。

### 3.3 其他观察

- 仓库根的 `publish/`（内含 `KimiPlanbarTray-rust.exe`、`SHA256SUMS.txt`）被 `.gitignore` 的 `publish/` 规则整体忽略，从未进入版本库。2026-09-21 复验确认它**不是普通暂存目录，而是 v1.7.2 已发布资产的本地副本（缺 zip）**：
  - `publish/KimiPlanbarTray-rust.exe` sha256 = `deab4c1104469732b08dc5f91c4847475566d90e45b2a0be5d82f1c1316a58b0`，与 GitHub v1.7.2 release 上传的同名资产摘要一致（5,924,864 B）。
  - `publish/SHA256SUMS.txt`（285 B）sha256 = `a4c8f08f172e54ce5a649fd1ff37b05f12ab2cabc162985b618d7ba52af37e08`，与 v1.7.2 上传的 `SHA256SUMS.txt` **逐字节一致**——即 §1.2 所述"修复前的 3 行"文件本体：两行指向并未作为独立资产上传的 WPF exe，且不含 zip 行。它同时是 §1.2 的本地实证（其 WPF 两行 == v1.5.0 冻结二进制摘要，rust 行 == v1.7.2 二进制摘要）。
  - 处置：**下次发版前删除或重新生成该目录里的 `SHA256SUMS.txt`，禁止直接复用**（新脚本产出的应是 2 行：rust exe + zip）。目录本身是否保留作为分发暂存，仍需用户确认。

## 4. 复现与验证命令

本机没有构建产物（`wpf/publish/`、`wpf/publish-sc/`、`rust/src-tauri/target/release/`、`rust/dist/` 均不存在），因此打包脚本要用占位二进制驱动：

```bash
# 占位二进制（三条路径都被 .gitignore 覆盖，不会污染版本库）
mkdir -p wpf/publish wpf/publish-sc rust/src-tauri/target/release
printf 'dummy' > wpf/publish/KimiPlanbarTray.exe
printf 'dummy' > wpf/publish-sc/KimiPlanbarTray-selfcontained.exe
printf 'dummy' > rust/src-tauri/target/release/kimi-planbar-tray.exe

PYTHONUTF8=1 python scripts/release/make_release_zip.py; echo "exit=$?"
cat SHA256SUMS.txt
python -m zipfile -l KimiPlanbarTray-v1.7.2.zip | grep -E "\.exe"   # 注意列表是空格补齐的，别用 \.exe$

# 清理
rm -f KimiPlanbarTray-*.zip SHA256SUMS.txt
rm -rf wpf/publish wpf/publish-sc rust/src-tauri/target
```

```bash
# 图标逐字节比对（本机图标库缺失时 exit 2 = SKIP）
PYTHONUTF8=1 python scripts/release/verify_icons.py
```

- `make_screenshots.py` 需先 `cd rust && npm run build`，且会覆盖 `docs/*.png` 基准，慎跑。
- 用户规范：交付级改动的验证须委派只读的 code-reviewer agent 独立执行（或以 `kimi-review` 独立入口），只给需求与路径、以证伪为导向。

## 5. 环境备注

- 中文 Windows（代码页 936）；Bash 工具是 Git Bash，用 POSIX 语法；Python 一律 `PYTHONUTF8=1`。
- `core.autocrlf=true` → 工作区 CRLF、git 内 LF（解释 §2 的 CRLF 差异）。
- 本机 `curl` 走 schannel，下载 GitHub release 资产需加 `--ssl-no-revoke`。
- 推送凭据由 Git Credential Manager 提供（`credential.helper=manager`），本机可直接 push。

# HANDOFF — scripts/ 规整与 release 资产一致性

> 状态：**已完成**，已归档于 `docs/archive/` —— **冻结，不再更新**。§3.2 三项与 §3.3 均已落地（收尾记录见 §6）；§3.1 保留为发版时流程的历史快照，live 清单在 `AGENTS.md` 的 Release process 与 `docs/SPEC.md` / `docs/SPEC_EN.md` §7.3。
> 对应提交：`45a46a7`（脚本规整）、`674543a`（校验和范围修正）、`22d59ca`（本文件入库 + 版本清单修订为 9 处）、`5d31aaa`（状态收尾 + 补末行换行），以及 §6 记录的收尾提交。
> 入库状态：核验时（2026-09-21）仍为"待提交"的 `AGENTS.md` active-handoff 指针 + 版本清单改写与本文档，已随 `22d59ca` 提交。
> 编写日期：2026-09-21；最后核验：2026-09-21（结论与证据见 §2.1）；收尾：2026-09-23（§6）。

## 0. 现状一句话

根目录 8 个脚本已收进 `scripts/{release,diagnostics}`；发布打包脚本的 `SHA256SUMS.txt` 现在只列真正会上传的资产。三个最新 release（v1.7.0 / v1.7.1 / v1.7.2）经逐文件比对确认与仓库一致，已发布资产按决定不做追溯修改。三个已登记小项（F1/F2/F3）已于 2026-09-23 全部修复、`publish/` 的过期校验和已删除；只剩下次发版前的版本号 bump（§3.1，属发版时流程）。

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
| 在 main 历史里（`tag..674543a` 方向） | 是（7 个提交） | 是（5） | 是（4） |
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
- **本地可查（仓库实测，全部成立）**：§1.1 的 8 处脚本位置、参数化改造与文档同步点（`docs/SPEC.md` / `docs/SPEC_EN.md` 各 5 处、`qt/src/icons.h` 注释、两份 README 无引用、`docs/archive/*` 仍含旧裸名）；`git grep -i rexxa` 零命中；§1.2 的 `uploaded` 标志与 `write_checksums()` 取用范围；§4 的三条占位路径确被 `.gitignore` 忽略（`git check-ignore`：根 `.gitignore:4` / `:5`，`rust/.gitignore:3`）；仓库内当前**不存在任何 `.pyc`**；F1 / F2 / F3 三项当时仍为未修状态（已于 2026-09-23 修复，见 §3.2 与 §6）。
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

### 3.2 已登记、用户决定暂缓的三项 —— 2026-09-23 全部修复

- **F2（低）zip 非原子写入 → 已修**：`make_release_zip.py` 把 zip 与 `SHA256SUMS.txt` **各自**先写成 `<name>.part`，两者都完成后再改名到正式名——**校验和文件先改名、zip 最后改名**；zip 改名失败时把改名前的校验和内容（几百字节，已在内存里）原样写回，所以失败后留在盘上的始终是**上一轮一致的那一对**，既不会出现"看起来像成品"的残缺 zip 配旧校验和，也不会出现"新 zip 配旧/缺失校验和"。`except BaseException` 分支清理两个 `.part`（`os.remove` 自身失败被吞掉，保证原始异常照常抛出，Ctrl-C 也走这条路径）。写校验和也改为先写 `.part`，顺带消除了旧的"先截断 `SHA256SUMS.txt` 再写"窗口。该改法自带的新风险已一并封堵：`.part` 在 `add_tree()` 期间就存在，若不被忽略会被 `git ls-files --others` 收进 zip 自己——`.gitignore` 的 `KimiPlanbarTray-*.zip*` 与 `SHA256SUMS.txt*`、脚本的 `EXCLUDE_SUFFIXES = (".zip", ".part", ".pyc")`、`EXCLUDE_FILES` 里的 `PART` / `SUMS_PART` 项多重覆盖。**已知残留窗口**（三处，均为"进程被杀/二次失败"级别）：① 校验和改名成功后、zip 改名完成前进程被强杀 → 新校验和 + 旧 zip（同目录两文件无法真正原子提交）；② 回滚写回校验和本身也失败（此时磁盘上已是新 zip + 旧内容，错误信息以 zip 改名失败为准）；③ 改名瞬间被别的进程独占（`os.access` 看不见，只有改名时才暴露）——此时按 ①② 的规则处理：第二次改名失败即回滚，第一次改名失败则原封不动。另外，**只读的残留 `.part`** 无法被自行清理，但会被预检按文件名拒绝（`cannot replace …: not writable`），不会以难懂的 `PermissionError` 形式出现。
- **F3（低）`.gitignore` 缺 `__pycache__/` → 已修**：`.gitignore` 增 `__pycache__/` 与 `*.pyc`；脚本侧 `EXCLUDE_DIRS` 增 `__pycache__`、`EXCLUDE_SUFFIXES` 增 `.pyc`，git 不可用时的兜底 walk 分支同样排除。仓库内仍无任何 `.pyc`（预防性修复）。
- **F1（低）本地自校验不友好 → 已修，采用方案②**：`.gitignore` 增 `KimiPlanbarTray-*.exe`；脚本侧新增 `EXCLUDE_PATTERNS = ("kimiplanbartray-*.exe",)`，且 `EXCLUDE_FILES` 也统一改成小写比较，主路径与兜底 walk 都生效，Windows/Linux 上结果一致（不受 `os.path.normcase` / `core.ignorecase` 差异影响）。于是可把发布名 exe 放在仓库根就地 `sha256sum -c SHA256SUMS.txt`，且不会被收进 zip。**残留限制**：必须使用发布原名（`KimiPlanbarTray-rust.exe` / `KimiPlanbarTray-wpf.exe` / `KimiPlanbarTray-wpf-selfcontained.exe`）；改名后的副本仍会被打包，下一轮打包前请自行清理。
- **三轮独立证伪复验追加修复的项（2026-09-23）**：① **目标预检** —— 新增 `ensure_replaceable(OUT, SUMS, PART, SUMS_PART)`，构建前按文件名拒绝只读目标、占位目录与只读残留 `.part`，把 `os.access` 看得见的失败挡在动手之前；独占锁只有真正改名时才发现，由下面的"校验和先改名 + 内存回滚"兜住。② **git 归属兜底 + 空集保护** —— `source_files()` 用 `REPO_SENTINELS`（`AGENTS.md`、本脚本自身）判断 `git ls-files` 是否答的是本仓库，若这棵树被解包进别人的 checkout（git 成功但列的是外层仓库）则退回过滤 walk；`add_tree()` 在源码集为空时 `SystemExit`，不再产出"只有二进制"的 zip。③ 把缺少二进制的前置检查移进 `try`，使前次崩溃残留的 `.part` 也会被清理。④ 兜底 walk 的 `EXCLUDE_DIRS` 补 `build`（`qt/.gitignore` 忽略它，而 walk 读不到 `.gitignore`；漏掉时会打进 `qt/build/**`）。⑤ 清理与回滚里的 `os.remove` / 写回失败一律吞掉，只上抛原始错误。⑥ 改名顺序改为"校验和先、zip 后"，并在 zip 改名失败时按内存内容回滚校验和（复验 #2 的 D1）；`EXCLUDE_FILES` 统一小写比较。

### 3.3 其他观察

- 仓库根的 `publish/`（内含 `KimiPlanbarTray-rust.exe`、`SHA256SUMS.txt`）被 `.gitignore` 的 `publish/` 规则整体忽略，从未进入版本库。2026-09-21 复验确认它**不是普通暂存目录，而是 v1.7.2 已发布资产的本地副本（缺 zip）**：
  - `publish/KimiPlanbarTray-rust.exe` sha256 = `deab4c1104469732b08dc5f91c4847475566d90e45b2a0be5d82f1c1316a58b0`，与 GitHub v1.7.2 release 上传的同名资产摘要一致（5,924,864 B）。
  - `publish/SHA256SUMS.txt`（285 B）sha256 = `a4c8f08f172e54ce5a649fd1ff37b05f12ab2cabc162985b618d7ba52af37e08`，与 v1.7.2 上传的 `SHA256SUMS.txt` **逐字节一致**——即 §1.2 所述"修复前的 3 行"文件本体：两行指向并未作为独立资产上传的 WPF exe，且不含 zip 行。它同时是 §1.2 的本地实证（其 WPF 两行 == v1.5.0 冻结二进制摘要，rust 行 == v1.7.2 二进制摘要）。
  - 处置（2026-09-23 落地）：已删除 `publish/SHA256SUMS.txt`（删前 285 B、sha256 `a4c8f08f…`，与 v1.7.2 上传件逐字节一致），`publish/` 目录与其中的 `KimiPlanbarTray-rust.exe` **保留作本地分发暂存**（用户定）——下次发版由新脚本在仓库根产出 2 行校验和后再按需拷入，**不得复用旧文件**。

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
- `sha256sum` 只随 Git Bash 提供（`C:\Program Files\Git\usr\bin\sha256sum.exe`）。在受限沙箱（workspace-write）里 msys 程序**创建不了 signal pipe**（`fatal error - couldn't create signal pipe, Win32 error 5`），直接调 `sha256sum.exe` 与 `bash -lc "sha256sum -c SHA256SUMS.txt"` **都会失败**；本文档 §6 的成功记录来自把该命令在更宽沙箱权限下重跑。不允许放宽权限时改用 `certutil -hashfile <file> SHA256` 或 Python `hashlib` 复核（两者结果一致）。

## 6. 收尾记录（2026-09-23）

用户决定：本轮只做 §3.2 三项修复 + 文档同步 + 归档；**不做版本号 bump**（`VERSION` 与 9 处版本元数据保持 1.7.2，`rust/package-lock.json` 的 1.6.0 也保持原样）。§3.1 因此仍留作"下次发版前"的流程项，live 清单在 `AGENTS.md` Release process 第 1 步与 `docs/SPEC.md` / `docs/SPEC_EN.md` §7.3 第 1 步。

改动面：代码卫生 = `scripts/release/make_release_zip.py` + `.gitignore`；文档 = `AGENTS.md`（归档指针 + Release process 第 3 步）、`docs/SPEC.md` 与 `docs/SPEC_EN.md` §7.3 第 3/4 步、本文件并 `git mv` 入 `docs/archive/`；`publish/SHA256SUMS.txt` 的删除不产生提交（untracked + ignored）。收尾提交：`cc21645`（代码卫生）、`9e30cbb`（归档 `git mv`，100% 重命名，故 `git log --follow` 跨移动连通）、`aac70e7`（文档与本文档内容）；本行哈希由紧随其后的一个小提交补记。

验证证据（本机实测，占位二进制驱动，准备与清理命令沿用 §4）；表内每一项都在两轮独立复验所指出的修复之后重跑过：

| 验收项 | 命令要点 | 结果 |
|---|---|---|
| V1 忽略规则 | `git check-ignore -v` 五个探针 | 命中 `.gitignore` 的 `KimiPlanbarTray-*.exe`（含全大写变体）/ `KimiPlanbarTray-*.zip*` / `SHA256SUMS.txt*` / `__pycache__/`，exit 0；`git ls-files --others --exclude-standard` 无 `.exe`/`.pyc`/`.part` |
| V2 兜底 walk 排除 | 令 `subprocess.run` 抛 `OSError` 后调 `source_files()` | 泄漏 0（132 个文件，无 publish/target/pycache/exe，大写 exe 也不漏） |
| V3 正常打包 | `PYTHONUTF8=1 python scripts/release/make_release_zip.py` | exit 0；`.zip.part` 与 `SHA256SUMS.txt.part` 均无残留；`SHA256SUMS.txt` 恰 2 行；zip 内 3 个 exe 且 `KimiPlanbarTray-rust.exe` 仅 1 条（无重名）；`.pyc`/`.part` 条目 0 |
| V3b 本地校验 | `sha256sum -c SHA256SUMS.txt`（Git Bash） | `KimiPlanbarTray-rust.exe: OK`、`KimiPlanbarTray-v1.7.2.zip: OK` |
| V4 失败注入 | 令 `add_tree` 抛 `RuntimeError` 后调 `main()` | `RuntimeError` 照常上抛；两个 `.part` 都无残留；旧 zip 与旧 `SHA256SUMS.txt` 摘要均不变 |
| V4c/V4d 只读目标 | 分别把 `SHA256SUMS.txt` / zip 置为只读后调 `main()` | 报 `cannot replace …: not writable`（预检，构建前即中止）；旧 zip + 旧校验和逐字节不变；无 `.part` |
| V4e 清理自身失败 | `os.remove` 抛 `PermissionError` + `add_tree` 抛 `RuntimeError` | 上抛的仍是 `RuntimeError`（不再被清理异常顶替） |
| V4f 残留 `.part` + 缺二进制 | 先放一个假 `.part`，再移走 rust 占位二进制 | 报 `missing binary: …`；假 `.part` 被清理；zip/校验和不变 |
| V4b 旧行为对照 | 对 TEMP 中 4096 B 假文件用旧的 `ZipFile(path, "w")` + 抛错 | 首个成员写入**前**失败 → 4096 B 变 **22 B 且 `is_zipfile()==True`**（空但合法的 zip，正是 `PK\x05\x06`）；写完一个成员后失败 → 仍是 `is_zipfile()==True` 的残缺产物，大小随负载而定（实测 115 B / 3117 B / 4219 B 不等，故本行不引用固定数字）。两者都是能通过 `is_zipfile()` 检查的残缺成品——证明 F2 的修法确有必要 |
| V5 集合一致性 | zip 源码成员 vs `source_files()` | 132 == 132，无缺、无多；3 个二进制都在；无重名条目 |
| V6 git 归属兜底 | 令 `subprocess.run` 返回外层仓库的文件列表 | 哨兵判定非本仓库 → 退回过滤 walk，取到 132 个本仓库文件（修前会静默漏文件） |
| V7 空源码集 | `source_files = lambda: []` 后调 `add_tree` | `SystemExit: no source files found …`（修前会产出"只有二进制"的 zip 并 exit 0） |
| V8 大小写一致性 | `fnmatchcase` + `lower()` 探针 | `EXCLUDE_FILES` / `EXCLUDE_PATTERNS` / `EXCLUDE_SUFFIXES` 三者统一按小写比较：`KIMIPLANBARTRAY-RUST.EXE`、`sha256sums.txt` 在主路径与 walk 路径都被排除，且 132 个 tracked 文件无一被误排 |
| V9 校验和被独占锁 | 用 `CreateFileW(..., dwShareMode=0)` 锁住 `SHA256SUMS.txt` 后调 `main()` | 第一次改名（校验和）即失败 `PermissionError`；旧 zip 与旧校验和逐字节不变；无 `.part` |
| V10 目标被目录占位 | 把 `SHA256SUMS.txt` 换成同名目录后调 `main()` | 构建前即报 `cannot replace …: is a directory`；无 `.part` |
| V11 zip 被独占锁（回滚路径） | 锁住 `OUT` 后调 `main()` | 校验和先改名成功、zip 改名失败 → 旧校验和**逐字节写回**、zip 保持旧内容、无 `.part`——这正是复验 #2 抓出的那条路径 |
| V12 兜底 walk 的 build 目录 | 造出 `qt/build/CMakeCache.txt` 后只走 walk 分支 | 不被打进（`EXCLUDE_DIRS` 补了 `build`）；`qt/.gitignore` 忽略它，而 walk 读不到 `.gitignore` |
| V13 原先无校验和 + zip 改名失败 | 删掉 `SHA256SUMS.txt` 并锁住 `OUT` 后调 `main()` | 回滚把新建的校验和删除，回到"无校验和 + 旧 zip"的原始状态；随后正常跑一次即可恢复 |
| V14 只读残留 `.part` | 造一个只读的 `KimiPlanbarTray-v1.7.2.zip.part` 后调 `main()` | 预检直接报 `cannot replace …KimiPlanbarTray-v1.7.2.zip.part: not writable`（不再以难懂的 `PermissionError: … '…part'` 形式出现）；旧 zip 与旧校验和不变 |
| V5b 无回归 | `PYTHONUTF8=1 python scripts/release/verify_icons.py` | exit 0（4 项逐字节 OK） |

独立复验史（用户规范 §4 末条，全部只给需求与路径、以证伪为导向）：**复验 #1** 判 Request Changes，提出 6 项——校验和落在原子性范围之外、git 归属错判、清理异常顶替原始异常、前置检查遗留 `.part`、V4b 表述不严谨、大小写语义不对称；对应修复见 §3.2 第 4 条与上表 V4c–V8。**复验 #2** 判 Request Changes，指出 7 项——① 第二次改名失败（校验和被独占锁 / 被目录占位）仍会留下"完整新 zip + 旧或缺失校验和"，且清理会删掉唯一正确的 `SUMS_PART`；② 只读的 `.part` 清不掉；③ `ensure_replaceable` 用 `os.access` 看不见独占锁；④ `EXCLUDE_FILES` 仍区分大小写；⑤ 兜底 walk 读不到 `.gitignore`，会打进 `qt/build/**`；⑥ 哨兵依赖 `AGENTS.md` 仍在库；⑦ §5 关于 Bash 校验的说法在受限沙箱里不成立。处置：改为"校验和先改名、zip 最后改名、zip 失败时按内存内容回滚校验和"，`ensure_replaceable` 补"目录占位"判定，`EXCLUDE_FILES` 统一小写，walk 补 `build`，并在 §3.2 / §5 写清残留窗口与沙箱限制（行 ⑥ 作为已知取舍保留：哨兵是启发式，误判时退回的 walk 已能正确排除构建产物）。上表 V9–V13 即针对复验 #2 的实测证据。**复验 #3** 逐条确认复验 #2 的 7 项确已修复（含真实独占锁与目录占位两种场景）、全部回归项通过（132 == 132、`certutil`/`hashlib` 双路校验两个摘要、`verify_icons.py` exit 0、两处回滚子场景），仍判 Request Changes，但只剩提示与文档质量：① 只读残留 `.part` 会让整轮以难懂的 `PermissionError` 中止（现改为预检按名拒绝，见 V14）；② 现网文档没写出残留窗口与沙箱限制（已在 `AGENTS.md` 第 3 步、`docs/SPEC*.md` §7.3 第 3 步与 §5 补齐）；③ V4b 措辞自相矛盾（已改）。

未做（按计划）：版本号 bump（§3.1）、`verify_icons.py` / `make_screenshots.py` / `scripts/diagnostics/*` / 两份 README、已发布 release 资产与其正文的追溯修改、§2 的 CRLF 差异。

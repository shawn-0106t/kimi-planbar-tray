# HANDOFF — GitHub 基线补初始化 + 依赖批量升级（2026-10-09）

> 状态：**主体已完成**。ci.yml 权限修复（2026-10-09 用户曾决定暂缓）已于 2026-10-10 执行，本文件随该修复 PR 一并入库（原接手步骤第 5 条落地）；其余收尾项见「六、未完成与接手步骤」。
> 背景：本仓库建于全局 AGENTS.md「GitHub 基线」规则成型之前，本轮按 `~/.kimi-code/references/github-baseline.md` 逐项补齐，随后处置了 Dependabot 首轮依赖 PR 与安全告警。CI 门禁范围经用户确认**仅 rust/**（wpf 冻结、qt 实验性，均不纳入）。

## 一、已落地（repo 文件，commit `223e42d` + `3606347`）

- `.github/workflows/ci.yml`：
  - 新增 `build-and-test` job 内两个门禁 step：`cargo fmt --check`、`cargo clippy --all-targets --locked -- -D warnings`（均在 npm/tsc/vite 步骤**之前**）
  - 新增 `gitleaks` job：ubuntu-latest、`fetch-depth: 0` 全历史扫描、`GITLEAKS_VERSION` 钉 **8.29.1**（避开 8.30.x "规则加载但永不命中"回归，gitleaks issue #2170）、action `gitleaks/gitleaks-action@e0c47f4f...`（v3.0.0，SHA 钉版风格与仓库一致）
- `.github/dependabot.yml`：cargo（`/rust/src-tauri`）+ npm（`/rust`）+ github-actions 三生态，weekly，minor+patch 按生态合组单 PR、major 单开
- `SECURITY.md`：双语，私密漏洞报告流程 + token 数据流 scope
- `.gitignore`：补 `.env` / `*.key` / `*.pem` / `*credentials*.json|toml`（**带扩展名限定**，`credentials.rs`/`.h`/`.cpp` 源码不受影响，已 check-ignore 验证零误伤）
- rust 代码：`cargo fmt` 一次性格式化 6 文件（纯重排）+ 修 2 处 clippy 存量警告（`panel.rs` HWND 恒等 cast、`lib.rs` needless borrow，code-reviewer 核实零行为变化）
- 文档同步：`AGENTS.md`、`docs/SPEC.md`、`docs/SPEC_EN.md` 的 CI/Dependabot 描述段

## 二、已落地（GitHub repo 设置，均 gh api 执行并验证）

- Dependabot **alerts** + **security updates**：已开
- **私密漏洞报告**（private vulnerability reporting）：已开（注意：REST `PATCH /repos` 的 `security_and_analysis` 字段写不进此项，须用专用端点 `PUT /repos/{owner}/{repo}/private-vulnerability-reporting`）
- **Branch protection**（main）：required checks = `Build & test (windows-latest)` + `Secret scan (gitleaks)`，`enforce_admins=true`（含管理员），strict=false（不要求 up-to-date），禁 force push/deletion。**从此 main 不能直推，改动一律走 PR**
- **Auto-merge**：`allow_auto_merge=true`（本轮为批量合并 Dependabot PR 临时开启，可保留）

## 三、已落地（本机，不进 git）

- `.git/hooks/pre-commit`：从 `~/.git-template/hooks/pre-commit` 补装（三层扫描：gitleaks + 密钥正则 + 隐私/路径正则），已在两次 commit 中实战运行。旧 repo 不吃 `init.templateDir`，换机需重装。

## 四、验证与审查记录

- code-reviewer 子代理独立审查：代码与 workflow 配置零缺陷（含 action SHA 逐一比对、tauri 源码级确认 clippy step 排序安全、check-ignore 全量扫描）；3 条待办全部处置（2 条为时序项，1 条文档顺序措辞已修）
- push 后 CI 双 job 在真实 runner 通过；pre-commit 钩子实战正常

## 五、Dependabot 首轮处置记录（9 PR + 3 告警）

| 项 | 内容 | 处置 |
|---|---|---|
| PR #9 | source-map-js 1.2.2（High 安全修复，npm dev） | ✅ squash 合并，alert #1 自动 fixed |
| PR #8 | rustls 0.23.45（Medium 安全修复） | ✅ squash 合并，alert #3 自动 fixed |
| PR #1 | npm minor+patch 组（2 项） | ✅ auto-merge 合并 |
| PR #4 | cargo minor+patch 组（tauri 2.11.6 等 5 项） | ✅ auto-merge 合并 |
| PR #2 | typescript 5.9.3 → **7.0.2**（dev） | ✅ 合并，tsc --noEmit 过 |
| PR #3 | vite 6.4.3 → **8.3.3**（dev） | ✅ 合并，vite build 过 |
| PR #5 | reqwest 0.12.28 → **0.13.4** | ✅ 合并 |
| PR #7 | winreg 0.52.0 → **0.55.0** | ✅ 合并 |
| PR #6 | windows 0.61.3 → **0.62.2** | ✅ auto-merge 合并（`9879cb1`）；合并触发的 main 组合态 CI 见接手步骤第 1 条 |
| alert #2 | glib unsoundness（Medium，需 0.20.0） | 🚫 **dismissed**（`not_used` + 注释）：Windows-only 项目，glib 只在 Linux target 的 transitive lock 分支，不编译进产物；上游 gtk 栈升级后自然消除，届时 GitHub 会重新告警提醒复查。（2026-10-10 复核：当时的 dismiss 实际未落成，告警仍为 open，已重新执行 dismiss 并确认 state=dismissed） |

## 六、未完成与接手步骤（按序）

1. **确认 #6 合并后的 main 最终组合态 CI**：windows 0.62.2 合入（`9879cb1`）触发的 run `37950158058` 在撰写本文时仍在跑（本地 main 已同步至该 commit）。接手第一步：`gh run watch 37950158058`（或 Actions 页）确认绿。CI 编译级已在 PR 分支验证过，风险低；若红，优先排查 DPI/panel 路径（windows 0.61→0.62 的变化集中在 metadata 层）。
2. **本地运行时自检**（CI 只覆盖编译级）：按 AGENTS.md Testing 节流程，`cd rust/src-tauri && cargo build` 后对 debug exe 跑 `--test-fetch` 与 `--test-ui`，确认 windows 0.62 后配额拉取与窗口构造无行为回归。
3. **依赖版本文档同步**：`AGENTS.md` Repository layout 中 `windows 0.61` 的表述改为 0.62；顶部「`rust/package-lock.json` 目前 stale 在 1.6.0」的说法复核一次（本轮 npm 侧 Dependabot PR 可能已顺带 resync 根 version 字段）。
4. **Code scanning 告警修复（用户已决定暂缓，方案留档备执行）**——CodeQL `actions/missing-workflow-permissions`（Medium，`ci.yml:43`）：
   - 是什么：`build-and-test` job 未声明 `permissions`，GITHUB_TOKEN 走仓库默认宽权限（个人 repo 默认 contents 可写）；gitleaks job 当时写了 job 级 `permissions: contents: read`，build-and-test 漏了。属供应链加固项。
   - 原因：本轮写 ci.yml 时只在最小权限有明确诉求的 job 上加了 permissions，未做 workflow 级统一声明；用户开启 CodeQL default setup 后被其 Actions 规则集检出。
   - 修复（改 `ci.yml` 两处，走分支 + PR，CI 绿后 squash 合并，CodeQL 下次 main 分析自动消警）：
     1. workflow 顶层加 `permissions: contents: read`（两个 job 都只需读；rust-cache/npm 缓存走 Actions Runtime Token，不受影响），删除 gitleaks job 内冗余的 job 级 permissions 块
     2. `on.pull_request` 去掉 `paths-ignore`（坑 1 联动）
5. **本 HANDOFF 入库**：优先随 ci.yml 修复 PR 一并提交；若要单独入库，必须先完成坑 1 的 `paths-ignore` 修复，否则 docs-only PR 会被 required checks 卡死（见下节坑 1）。

## 七、已知坑与后续注意

1. **docs-only PR 会卡死在 required checks**：`on.pull_request` 的 `paths-ignore` 会让纯文档 PR 不触发 CI → required checks 永不出现 → `enforce_admins=true` 下连管理员都无法 bypass → PR 永久无法合并。修复：`pull_request` 触发去掉 `paths-ignore`（push 到 main 保留，省 runner 时间）。顺带弥补了「md-only 改动绕过 gitleaks」的缺口（reviewer 曾作为 Suggestion 提出）。
2. **批量合并经验**：auto-merge 逐个触发即可，Cargo.lock/package-lock 跨 PR 的 auto-merge 通常友好（按 crate/包分段）；真冲突时对 PR 评论 `@dependabot rebase` 比手动解稳。
3. **依赖版本升级后的文档同步**（收尾清单）：
   - `AGENTS.md` Repository layout 中 `windows 0.61` 的表述 → 0.62（#6 合并后）
   - `AGENTS.md` 顶部「package-lock.json 目前 stale 在 1.6.0」的说法需复核（npm PR 可能已顺带 resync 根 version）
   - `Cargo.toml`/`tauri.conf.json` 等版本描述随下次 release 流程统一核对
4. **运行时自检建议**（CI 只覆盖编译级）：windows 0.62 涉及 DPI/panel 代码路径，合并 #6 后本地跑 `cargo build` + `--test-fetch` + `--test-ui` 验证无行为回归（AGENTS.md Testing 节流程）。
5. gitleaks `GITLEAKS_VERSION` 钉版在 8.29.1，上游修复 #2170 并验证后可解钉（跨 action 大版本原样继承）。
6. CodeQL default setup 只分析 default branch，PR 上显示 "skipping" 属正常。

## 八、索引

- 基线 commit：`223e42d`（fmt+clippy）、`3606347`（ci/dependabot/SECURITY/gitignore/文档）
- 依赖升级落地：`ddbc92a`、`7eb3a56`、`f272978`、`af2ede9`、`351bf4d`、`f66763b`、`2f3fa92`、`44ab1a0`（#6 已落，`9879cb1`）
- 参考规则：`~/.kimi-code/references/github-baseline.md`；模板：`~/.kimi-code/scripts/github-baseline/`

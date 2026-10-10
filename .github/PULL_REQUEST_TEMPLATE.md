## What / 改动内容

<!-- What does this PR change, and why? / 本 PR 改了什么、为什么？ -->

## Verification / 验证

<!-- How you verified it beyond CI: local --test-fetch / --test-update / --test-ui self-checks,
     visual comparison against docs/*.png baselines, etc. (see CONTRIBUTING.md) -->
<!-- CI 之外的验证方式：本地 --test-fetch / --test-update / --test-ui 自检、与 docs/*.png 基准对比等（见 CONTRIBUTING.md） -->

## Checklist / 检查清单

- [ ] Changes target `rust/` (or are docs/community files); qt/ changes mirror a rust/ change and were discussed in an issue first
      改动指向 `rust/`（或为文档/社区文件）；qt/ 改动须镜像 rust/ 的变更且已在 issue 中先行讨论
- [ ] CI passes (fmt, clippy, tsc, vite build, cargo build/test, secret scan)
      CI 通过（fmt、clippy、tsc、vite build、cargo build/test、密钥扫描）
- [ ] Runtime self-checks run locally (`--test-fetch` / `--test-ui`)
      已在本地执行运行时自检（`--test-fetch` / `--test-ui`）
- [ ] Docs updated if behavior changed (`docs/SPEC.md` + `docs/SPEC_EN.md` kept in sync)
      行为有变化时已更新文档（`docs/SPEC.md` 与 `docs/SPEC_EN.md` 保持同步）

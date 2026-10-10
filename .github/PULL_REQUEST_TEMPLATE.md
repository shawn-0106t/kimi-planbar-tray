## What

<!-- What does this PR change, and why? -->

## Verification

<!-- How you verified it beyond CI: local --test-fetch / --test-update / --test-ui self-checks,
     visual comparison against docs/*.png baselines, etc. (see CONTRIBUTING.md) -->

## Checklist

- [ ] Changes target `rust/` (or are docs/community files); qt/ changes mirror a rust/ change and were discussed in an issue first
- [ ] CI passes (fmt, clippy, tsc, vite build, cargo build/test, secret scan)
- [ ] Runtime self-checks run locally (`--test-fetch` / `--test-ui`)
- [ ] Docs updated if behavior changed (`docs/SPEC.md` + `docs/SPEC_EN.md` kept in sync)

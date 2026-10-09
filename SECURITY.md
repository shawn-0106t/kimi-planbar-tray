# Security Policy / 安全策略

## 支持版本 / Supported Versions

仅最新 Release 接受安全修复 / Only the latest release receives security fixes.

| 版本 / Version | 支持状态 / Supported |
|---|---|
| 最新 tag / latest tag | ✅ |
| 更早版本 / older | ❌（请先升级 / please upgrade first） |

## 报告漏洞 / Reporting a Vulnerability

**中文**：请使用 GitHub 的**私密漏洞报告**（仓库 Security 标签页 → Report a vulnerability），**不要**为安全问题开公开 issue 或 Discussion。

**English**: Please use GitHub's **private vulnerability reporting** (Security tab → Report a vulnerability). Do **not** open public issues for security matters.

## 范围说明 / Scope

- 本应用读取本地 Kimi Code CLI 凭证（`~/.kimi-code/credentials/kimi-code.json` 的 OAuth access_token，或 `~/.kimi-code/config.toml` 里的 api_key），仅作为 Bearer token 发往 `https://api.kimi.com/coding/v1/usages` 查询配额；token 不落盘到其他位置、不写日志、不上报遥测、不发送到任何其他端点。
- This app reads the local Kimi Code CLI credential (the OAuth access_token from `~/.kimi-code/credentials/kimi-code.json`, or an api_key from `~/.kimi-code/config.toml`) and sends it only as a Bearer token to `https://api.kimi.com/coding/v1/usages`. The token is never persisted elsewhere, logged, transmitted as telemetry, or sent to any other endpoint.
- 值得报告的问题示例 / Examples worth reporting：凭证泄漏或外发、文件权限不当、路径处理缺陷导致的任意文件读写、注入与崩溃。
- 不在本仓库范围 / Out of scope：上游依赖自身的漏洞（请报告给上游 / report to the upstream project）；纯视觉/体验问题（开普通 issue 即可 / open a regular issue）。

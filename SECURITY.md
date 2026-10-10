# Security Policy

English | [中文](#安全策略)

## Supported Versions

Only the latest release receives security fixes.

| Version | Supported |
|---|---|
| latest tag | ✅ |
| older | ❌ (please upgrade first) |

## Reporting a Vulnerability

Please use GitHub's **private vulnerability reporting** (Security tab → Report a vulnerability). Do **not** open public issues or Discussions for security matters.

## Scope

- This app reads the local Kimi Code CLI credential (the OAuth access_token from `~/.kimi-code/credentials/kimi-code.json`, or an api_key from `~/.kimi-code/config.toml`) and sends it only as a Bearer token to `https://api.kimi.com/coding/v1/usages`. The token is never persisted elsewhere, logged, transmitted as telemetry, or sent to any other endpoint.
- Examples worth reporting: credential leaks or exfiltration, improper file permissions, arbitrary file read/write via path-handling flaws, injection, crashes.
- Out of scope: vulnerabilities in upstream dependencies (report those to the upstream project); purely visual/UX issues (open a regular issue).

---

# 安全策略

[English](#supported-versions) | 中文

## 支持版本

仅最新 Release 接受安全修复。

| 版本 | 支持状态 |
|---|---|
| 最新 tag | ✅ |
| 更早版本 | ❌（请先升级） |

## 报告漏洞

请使用 GitHub 的**私密漏洞报告**（仓库 Security 标签页 → Report a vulnerability），**不要**为安全问题开公开 issue 或 Discussion。

## 范围说明

- 本应用读取本地 Kimi Code CLI 凭证（`~/.kimi-code/credentials/kimi-code.json` 的 OAuth access_token，或 `~/.kimi-code/config.toml` 里的 api_key），仅作为 Bearer token 发往 `https://api.kimi.com/coding/v1/usages` 查询配额；token 不落盘到其他位置、不写日志、不上报遥测、不发送到任何其他端点。
- 值得报告的问题示例：凭证泄漏或外发、文件权限不当、路径处理缺陷导致的任意文件读写、注入与崩溃。
- 不在本仓库范围：上游依赖自身的漏洞（请报告给上游项目）；纯视觉/体验问题（开普通 issue 即可）。

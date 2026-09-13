// Credential chain, 1:1 port of rust/src-tauri/src/credentials.rs (SPEC 16.2):
// 1) <kimi_home>/credentials/kimi-code.json -> access_token (expires_at > now+30s)
// 2) <kimi_home>/config.toml -> provider whose base_url contains api.kimi.com/coding
// 3) none -> caller reports "no-token"
// <kimi_home> honors the KIMI_CODE_HOME override.

#ifndef CREDENTIALS_H
#define CREDENTIALS_H

#include <QString>
#include <optional>

namespace credentials {

// USERPROFILE, falling back to HOMEDRIVE+HOMEPATH.
std::optional<QString> homeDir();

// <home>/.kimi-code, honoring the KIMI_CODE_HOME override (SPEC 16.2 / 21.2).
QString kimiHome(const QString &home);

// Returns the OAuth access token or the config.toml api_key; std::nullopt when
// neither is available (caller surfaces "no-token").
std::optional<QString> loadToken();

} // namespace credentials

#endif // CREDENTIALS_H

// CLI version check, 1:1 port of rust/src-tauri/src/update.rs (SPEC section 17):
// local `kimi --version` (5s timeout, kill) -> docs changelog (Range 0-4095)
// -> GitHub Releases API fallback. All failures degrade silently.

#ifndef UPDATE_H
#define UPDATE_H

#include <QString>
#include <optional>

struct UpdateStatus {
    std::optional<QString> localVersion;
    std::optional<QString> latestVersion;
    bool updateAvailable = false;
    bool checkFailed = false;
};

namespace update {

// Runs the full check synchronously (local detect + remote latest).
UpdateStatus check();

} // namespace update

#endif // UPDATE_H

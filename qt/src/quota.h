// Quota fetch + defensive parsing, 1:1 port of rust/src-tauri/src/quota.rs
// (SPEC section 16). Traps honored here:
//  - server JSON numbers are modeled as strings, numeric fallback tolerated
//  - Extra Usage amountLeft unit is 1e-8 yuan -> cents = (raw + 500000) / 1000000
//  - isEnabled == false must be reported as NotActivated

#ifndef QUOTA_H
#define QUOTA_H

#include <QDateTime>
#include <QString>
#include <optional>

struct QuotaSegment {
    double percent = 0.0;
    std::optional<QDateTime> resetAt; // local time
};

enum class ExtraState { NotActivated, NoData, Ready };

struct ExtraInfo {
    ExtraState state = ExtraState::NotActivated;
    std::optional<qint64> balanceCents;
    bool monthlyEnabled = false;
    std::optional<qint64> monthlyUsedCents;
    std::optional<qint64> monthlyLimitCents;
};

struct QuotaResult {
    std::optional<QuotaSegment> fiveHour;
    std::optional<QuotaSegment> week;
    std::optional<ExtraInfo> extra;
    QDateTime fetchedAt;    // local time, seconds precision
    int fetchedAtNanos = 0; // sub-second nanoseconds (system clock has 100ns units)
    std::optional<QString> error;

    static QuotaResult failed(const QString &kind);

    // On failure keep last-known-good data: fill null fields from `last`
    // (SPEC 16.5 step 2; the UI only shows the failure hint in the status line).
    void fillMissingFrom(const QuotaResult &last);
};

namespace quota {

// Fetches https://api.kimi.com/coding/v1/usages once (synchronous, 10s
// timeout). Never throws; failure kinds match the .NET exception-type names
// used by the reference ("no-token", "HttpRequestException",
// "TaskCanceledException", "JsonException").
QuotaResult fetch();

} // namespace quota

#endif // QUOTA_H

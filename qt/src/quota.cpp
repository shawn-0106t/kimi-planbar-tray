#include "quota.h"

#include "credentials.h"
#include "http_util.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonValue>
#include <QRegularExpression>
#include <QTimeZone>
#include <cmath>
#include <limits>

#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

namespace {

const char kUsagesUrl[] = "https://api.kimi.com/coding/v1/usages";

// Current local time with the sub-second fraction from the precise system
// clock (100ns units), matching chrono::Local::now() on Windows.
void nowPrecise(QDateTime &dt, int &nanos)
{
    FILETIME ft;
    GetSystemTimePreciseAsFileTime(&ft);
    const quint64 ticks = (static_cast<quint64>(ft.dwHighDateTime) << 32) | ft.dwLowDateTime;
    const qint64 unixMs = static_cast<qint64>(ticks / 10000ULL - 11644473600000ULL);
    nanos = static_cast<int>(ticks % 10000000ULL) * 100; // 100ns units -> ns
    dt = QDateTime::fromMSecsSinceEpoch(unixMs, Qt::UTC).toLocalTime();
}

// JSON number-or-string -> double, missing -> 0.
double getF64(const QJsonObject &v, const QString &key)
{
    const QJsonValue x = v.value(key);
    if (x.isDouble())
        return x.toDouble();
    if (x.isString()) {
        bool ok = false;
        const double d = x.toString().trimmed().toDouble(&ok);
        if (ok)
            return d;
    }
    return 0.0;
}

// JSON number-or-string -> qint64 (fractional/out-of-range doubles rejected,
// mirroring serde_json's Value::as_i64).
std::optional<qint64> getI64(const QJsonValue &v)
{
    if (v.isDouble()) {
        const double d = v.toDouble();
        if (d == std::trunc(d) && d >= -9.0e18 && d <= 9.0e18)
            return static_cast<qint64>(d);
        return std::nullopt;
    }
    if (v.isString()) {
        bool ok = false;
        const qint64 n = v.toString().trimmed().toLongLong(&ok);
        if (ok)
            return n;
    }
    return std::nullopt;
}

// resetTime: RFC3339 first, then the looser shapes DateTimeOffset.TryParse
// accepts (space separator, with/without offset; no offset = local time).
std::optional<QDateTime> parseResetTime(const QString &s)
{
    static const QRegularExpression re(QStringLiteral(
        "^(\\d{4})-(\\d{2})-(\\d{2})[T ](\\d{2}):(\\d{2}):(\\d{2})(\\.\\d+)?\\s*(Z|[+-]\\d{2}:?\\d{2})?$"));
    const QRegularExpressionMatch m = re.match(s.trimmed());
    if (!m.hasMatch())
        return std::nullopt;
    const QDate date(m.captured(1).toInt(), m.captured(2).toInt(), m.captured(3).toInt());
    // Qt is millisecond-based: keep the first three fraction digits
    int msec = 0;
    const QString frac = m.captured(7);
    if (frac.size() > 1)
        msec = (frac.mid(1, 3) + QStringLiteral("000")).left(3).toInt();
    const QTime time(m.captured(4).toInt(), m.captured(5).toInt(), m.captured(6).toInt(), msec);
    if (!date.isValid() || !time.isValid())
        return std::nullopt;
    const QString off = m.captured(8);
    if (off.isEmpty())
        return QDateTime(date, time, Qt::LocalTime);
    if (off == QLatin1String("Z"))
        return QDateTime(date, time, QTimeZone::UTC).toLocalTime();
    const int sign = off.startsWith(QLatin1Char('-')) ? -1 : 1;
    const QString digits = QString(off).remove(QLatin1Char(':')).mid(1);
    const int hours = digits.left(2).toInt();
    const int minutes = digits.mid(2, 2).toInt();
    // Validate like DateTimeOffset.TryParse: offsets beyond +/-14:00 or with
    // minutes > 59 are invalid -> no reset time (Rust returns None).
    if (hours > 14 || minutes > 59)
        return std::nullopt;
    const QTimeZone zone(sign * (hours * 3600 + minutes * 60));
    if (!zone.isValid())
        return std::nullopt;
    return QDateTime(date, time, zone).toLocalTime();
}

QuotaSegment parseSegment(const QJsonValue &v)
{
    // Non-object input behaves like an empty object, as serde Value::get does
    const QJsonObject o = v.isObject() ? v.toObject() : QJsonObject();
    const double used = getF64(o, QStringLiteral("used"));
    double limit = getF64(o, QStringLiteral("limit"));
    if (limit <= 0.0)
        limit = 1.0; // guard against division by zero
    QuotaSegment seg;
    const QJsonValue rt = o.value(QStringLiteral("resetTime"));
    if (rt.isString())
        seg.resetAt = parseResetTime(rt.toString());
    // getF64 can yield inf/NaN from hostile strings ("1e999", "NaN"); keep
    // percent finite so serialization never emits a non-finite double
    const double percent = used / limit * 100.0;
    seg.percent = std::isfinite(percent) ? percent : 0.0;
    return seg;
}

std::optional<qint64> parseCents(const QJsonValue &money)
{
    if (!money.isObject())
        return std::nullopt;
    return getI64(money.toObject().value(QStringLiteral("priceInCents")));
}

ExtraInfo parseExtra(const QJsonValue &wallet)
{
    ExtraInfo info;
    if (!wallet.isObject())
        return info; // not an object -> NotActivated
    const QJsonObject w = wallet.toObject();

    // isEnabled defense: when the booster is disabled, amountLeft is an estimate
    // (monthly limit minus used), NOT the real balance -> must read as NotActivated.
    const QJsonValue enabled = w.value(QStringLiteral("isEnabled"));
    if (enabled.isBool() && !enabled.toBool())
        return info;

    const QJsonValue balance = w.value(QStringLiteral("balance"));
    std::optional<qint64> raw;
    if (balance.isObject())
        raw = getI64(balance.toObject().value(QStringLiteral("amountLeft")));
    if (raw) {
        info.state = ExtraState::Ready;
        // saturating_add: a pathological amountLeft near i64::MAX must not overflow
        const qint64 rounded = *raw > std::numeric_limits<qint64>::max() - 500000
            ? std::numeric_limits<qint64>::max()
            : *raw + 500000;
        info.balanceCents = rounded / 1000000; // 1e-8 yuan -> cents, rounded
    } else {
        info.state = ExtraState::NoData;
    }

    const QJsonValue monthly = w.value(QStringLiteral("monthlyChargeLimitEnabled"));
    if (monthly.isBool() && monthly.toBool()) {
        info.monthlyEnabled = true;
        info.monthlyUsedCents = parseCents(w.value(QStringLiteral("monthlyUsed")));
        info.monthlyLimitCents = parseCents(w.value(QStringLiteral("monthlyChargeLimit")));
    }
    return info;
}

} // namespace

QuotaResult QuotaResult::failed(const QString &kind)
{
    QuotaResult r;
    nowPrecise(r.fetchedAt, r.fetchedAtNanos);
    r.error = kind;
    return r;
}

void QuotaResult::fillMissingFrom(const QuotaResult &last)
{
    if (!fiveHour)
        fiveHour = last.fiveHour;
    if (!week)
        week = last.week;
    if (!extra)
        extra = last.extra;
}

namespace quota {

QuotaResult fetch()
{
    const std::optional<QString> token = credentials::loadToken();
    if (!token)
        return QuotaResult::failed(QStringLiteral("no-token"));

    SyncHttpResult resp = syncGet(
        QUrl(QString::fromLatin1(kUsagesUrl)),
        {{QByteArray("Authorization"), QByteArray("Bearer ") + token->toUtf8()},
         {QByteArray("Accept"), QByteArray("application/json")}});
    if (resp.timedOut)
        return QuotaResult::failed(QStringLiteral("TaskCanceledException"));
    if (!resp.ok || resp.status < 200 || resp.status >= 300)
        return QuotaResult::failed(QStringLiteral("HttpRequestException"));

    QJsonParseError parseError;
    const QJsonDocument doc = QJsonDocument::fromJson(resp.body, &parseError);
    if (parseError.error != QJsonParseError::NoError)
        return QuotaResult::failed(QStringLiteral("JsonException"));
    // Non-object roots parse fine but yield all-null segments, as in Rust
    const QJsonObject root = doc.isObject() ? doc.object() : QJsonObject();

    QuotaResult r;
    nowPrecise(r.fetchedAt, r.fetchedAtNanos);

    // 5-hour segment: root.limits[0].detail
    const QJsonValue limits = root.value(QStringLiteral("limits"));
    if (limits.isArray() && !limits.toArray().isEmpty()) {
        const QJsonValue first = limits.toArray().first();
        const QJsonValue detail =
            first.isObject() ? first.toObject().value(QStringLiteral("detail")) : QJsonValue();
        if (!detail.isUndefined())
            r.fiveHour = parseSegment(detail);
    }
    // Week segment: root.usage
    const QJsonValue usage = root.value(QStringLiteral("usage"));
    if (usage.isObject())
        r.week = parseSegment(usage);
    r.extra = parseExtra(root.value(QStringLiteral("boosterWallet")));
    return r;
}

} // namespace quota

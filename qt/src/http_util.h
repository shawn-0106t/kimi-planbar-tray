// Synchronous HTTP GET helper for the headless --test-* paths: drives
// QNetworkAccessManager through a local QEventLoop with a hard timeout.
// All failures are silently swallowed and reported via the result fields
// (error-handling baseline: no dialogs, no logging).

#ifndef HTTP_UTIL_H
#define HTTP_UTIL_H

#include <QEventLoop>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QTimer>

struct SyncHttpResult {
    bool ok = false;       // a response was received (any HTTP status)
    bool timedOut = false; // aborted by our own watchdog
    int status = 0;        // HTTP status code, 0 when no response
    QByteArray body;
};

inline SyncHttpResult syncGet(const QUrl &url,
                              const QList<QPair<QByteArray, QByteArray>> &headers,
                              int timeoutMs = 10000)
{
    SyncHttpResult out;
    QNetworkAccessManager nam;
    QNetworkRequest req(url);
    for (const auto &h : headers)
        req.setRawHeader(h.first, h.second);

    QNetworkReply *reply = nam.get(req);
    QEventLoop loop;
    QTimer watchdog;
    watchdog.setSingleShot(true);
    QObject::connect(reply, &QNetworkReply::finished, &loop, &QEventLoop::quit);
    QObject::connect(&watchdog, &QTimer::timeout, &loop, [&out, reply, &loop]() {
        out.timedOut = true;
        reply->abort();
        loop.quit();
    });
    watchdog.start(timeoutMs);
    loop.exec();

    if (!out.timedOut) {
        out.ok = true;
        out.status = reply->attribute(QNetworkRequest::HttpStatusCodeAttribute).toInt();
        out.body = reply->readAll();
        // Bound the body: the endpoints we call answer with a few KB of JSON
        // or markdown; a hostile/erroring server must not grow memory without
        // limit (4 MiB cap, truncated — parsers fail closed downstream).
        if (out.body.size() > 4 * 1024 * 1024)
            out.body.truncate(4 * 1024 * 1024);
    }
    // No deleteLater: the reply is a child of `nam` and dies with it at scope
    // exit; a posted deletion would only run if an event loop spun again.
    return out;
}

#endif // HTTP_UTIL_H

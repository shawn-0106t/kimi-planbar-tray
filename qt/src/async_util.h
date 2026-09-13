// Runs `work` on a throwaway QThread, then delivers the result to `context`'s
// thread through a queued connection. Replaces the Rust edition's
// tauri::async_runtime::spawn for the synchronous fetch/check functions.

#ifndef ASYNC_UTIL_H
#define ASYNC_UTIL_H

#include <QObject>
#include <QThread>
#include <atomic>
#include <memory>

template <typename T, typename Work, typename Deliver>
void runAsync(QObject *context, Work work, Deliver deliver)
{
    auto result = std::make_shared<T>();
    // Written by the worker thread, read on the context's thread: atomic.
    auto ok = std::make_shared<std::atomic<bool>>(false);
    QThread *t = QThread::create([result, ok, work]() mutable {
        *result = work();
        ok->store(true);
    });
    // Delivery is context-bound (auto-disconnects if the context dies first)...
    QObject::connect(
        t, &QThread::finished, context,
        [result, ok, deliver]() mutable {
            if (ok->load())
                deliver(std::move(*result));
        },
        Qt::QueuedConnection);
    // ...while the thread object deletes itself — no dependence on the
    // context still being alive at exit (app teardown otherwise leaks it).
    QObject::connect(t, &QThread::finished, t, &QObject::deleteLater);
    t->start();
}

#endif // ASYNC_UTIL_H

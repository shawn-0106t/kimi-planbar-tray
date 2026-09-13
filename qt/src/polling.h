// Refresh scheduling, 1:1 port of rust/src-tauri/src/polling.rs (SPEC 16.5):
// first refresh 2s after start, period = max(1, RefreshMinutes), failure keeps
// last-known-good data and retries fast after 30s. The Rust reschedule/retime
// Notify pair becomes single-shot QTimer restarts on the GUI thread.

#ifndef POLLING_H
#define POLLING_H

#include <QObject>

class QTimer;
class App;
struct QuotaResult;

class Poller : public QObject
{
    Q_OBJECT
public:
    explicit Poller(App *app);

    void start();      // begin the cycle: first refresh 2s after launch
    void safeRefresh(); // manual/scheduled entry: fetch, then retime the tick
    void reschedule(); // settings saved: restart the cycle with the 2s delay

private:
    void onFetched(QuotaResult &&r);
    qint64 successDelayMs() const;

    App *m_app;
    QTimer *m_timer;
    bool m_inFlight = false;
};

#endif // POLLING_H

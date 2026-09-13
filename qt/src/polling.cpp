#include "polling.h"

#include "app.h"
#include "async_util.h"
#include "quota.h"
#include "state.h"
#include "trayicon.h"
#include "windows/panelwindow.h"

#include <QTimer>

Poller::Poller(App *app) : QObject(app), m_app(app)
{
    m_timer = new QTimer(this);
    m_timer->setSingleShot(true);
    connect(m_timer, &QTimer::timeout, this, &Poller::safeRefresh);
}

void Poller::start()
{
    m_timer->start(2000); // first refresh 2s after launch (SPEC 16.5)
}

void Poller::reschedule()
{
    m_timer->start(2000); // mirrors timer.Change(2s, period) on settings save
}

qint64 Poller::successDelayMs() const
{
    const qint64 mins = std::max<qint64>(1, m_app->state().settings.refreshMinutes);
    return mins * 60 * 1000;
}

void Poller::safeRefresh()
{
    // A fetch already in flight is left alone (the Rust edition would spawn a
    // parallel one; the result and retiming are identical either way).
    if (m_inFlight)
        return;
    m_inFlight = true;
    runAsync<QuotaResult>(this, [] { return quota::fetch(); },
                          [this](QuotaResult &&r) { onFetched(std::move(r)); });
}

void Poller::onFetched(QuotaResult &&r)
{
    m_inFlight = false;
    AppState &st = m_app->state();
    if (r.error && st.lastQuota)
        r.fillMissingFrom(*st.lastQuota);
    st.lastQuota = r;
    m_app->panelWindow()->renderQuota(st.lastQuota);
    m_app->trayIcon()->updateTooltip();
    // Every refresh (scheduled, manual, or hover) moves the next scheduled
    // tick: 30s after a failure, one period after a success (SPEC 16.5 step 3)
    m_timer->start(r.error ? 30000 : successDelayMs());
}

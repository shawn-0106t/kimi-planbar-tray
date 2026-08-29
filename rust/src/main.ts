import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './theme.css';
import './main.css';
import logoUrl from './assets/kimi-logo.png';
import {
  AppStateDto,
  ExtraInfo,
  QuotaResult,
  QuotaSegment,
  UpdateStatus,
  clampPercent,
  fmtPercent,
  fmtYuan,
  formatReset,
  initTheme,
} from './common';

function el<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

// ---- Rendering (mirrors MainWindow.xaml.cs Render/SetCard/RenderExtra) ----

function setCard(prefix: string, seg: QuotaSegment | null | undefined): void {
  const pct = el(`${prefix}-pct`);
  const fill = el(`${prefix}-fill`);
  const reset = el(`${prefix}-reset`);
  if (!seg) {
    pct.textContent = '--';
    fill.style.width = '0%';
    reset.textContent = '';
    return;
  }
  pct.textContent = fmtPercent(seg.percent);
  fill.style.width = `${clampPercent(seg.percent)}%`;
  reset.textContent = seg.resetAt ? formatReset(seg.resetAt) : '';
}

function renderExtra(extra: ExtraInfo | null | undefined): void {
  const balance = el('extra-balance');
  const monthlyPanel = el('extra-monthly');
  if (!extra) {
    balance.textContent = '--';
    monthlyPanel.hidden = true;
    return;
  }
  if (extra.state === 'Ready') {
    balance.textContent = extra.balanceCents != null ? fmtYuan(extra.balanceCents) : '--';
  } else if (extra.state === 'NoData') {
    balance.textContent = 'No data';
  } else {
    balance.textContent = 'Not activated';
  }
  if (extra.monthlyEnabled && extra.monthlyLimitCents != null && extra.monthlyLimitCents > 0 && extra.monthlyUsedCents != null) {
    const p = clampPercent((extra.monthlyUsedCents / extra.monthlyLimitCents) * 100);
    el('extra-fill').style.width = `${p}%`;
    el('extra-monthly-text').textContent =
      `Used ${fmtYuan(extra.monthlyUsedCents)} this month / ${fmtYuan(extra.monthlyLimitCents)} limit`;
    monthlyPanel.hidden = false;
  } else {
    monthlyPanel.hidden = true;
  }
}

function renderQuota(r: QuotaResult | null): void {
  setCard('week', r?.week);
  setCard('five', r?.fiveHour);
  renderExtra(r?.extra);
  const lastUpdated = el('last-updated');
  if (!r) {
    lastUpdated.textContent = '';
  } else if (r.error) {
    lastUpdated.textContent = 'Update failed';
  } else {
    const d = new Date(r.fetchedAt);
    const hh = String(d.getHours()).padStart(2, '0');
    const mm = String(d.getMinutes()).padStart(2, '0');
    lastUpdated.textContent = `Updated ${hh}:${mm}`;
  }
}

function renderVersion(u: UpdateStatus): void {
  el('cli-version').textContent = u.localVersion ?? 'Not detected';
  el('badge').hidden = !u.updateAvailable;
}

// ---- Show/hide animation (SPEC 15.1 / 15.2) ----

function playShow(): void {
  document.body.classList.remove('leave');
  // Force reflow so the enter transition always restarts from the initial state
  void document.body.offsetWidth;
  document.body.classList.add('enter');
}

function playHide(): void {
  document.body.classList.remove('enter');
  document.body.classList.add('leave');
  // Fade-out takes 130ms; tell the backend to actually hide the window afterwards
  window.setTimeout(() => {
    invoke('finish_hide_panel').catch(() => undefined);
    document.body.classList.remove('leave');
  }, 170);
}

// ---- Bootstrap ----

window.addEventListener('DOMContentLoaded', async () => {
  el<HTMLImageElement>('logo').src = logoUrl;

  let state: AppStateDto;
  try {
    state = await initTheme();
  } catch {
    return;
  }
  renderQuota(state.quota);
  renderVersion(state.update);

  await listen<QuotaResult>('quota-updated', (e) => renderQuota(e.payload));
  await listen<UpdateStatus>('update-status', (e) => renderVersion(e.payload));
  await listen('panel-show', () => playShow());
  await listen('panel-hide', () => playHide());

  el('btn-console').addEventListener('click', () => {
    invoke('open_console').catch(() => undefined);
  });
  el('btn-refresh').addEventListener('click', () => {
    invoke('refresh_now').catch(() => undefined);
  });
  el('btn-settings').addEventListener('click', () => {
    invoke('open_settings').catch(() => undefined);
  });
  el('btn-exit').addEventListener('click', () => {
    invoke('quit_app').catch(() => undefined);
  });
  el('version-row').addEventListener('click', () => {
    invoke('open_releases').catch(() => undefined);
  });
});

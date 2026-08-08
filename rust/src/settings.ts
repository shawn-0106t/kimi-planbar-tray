import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './theme.css';
import './settings.css';
import logoUrl from './assets/kimi-logo.png';
import { AppStateDto, initTheme } from './common';

function el<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

/** Backfill the current settings into the controls (SPEC 4.2: every open). */
async function backfill(): Promise<void> {
  try {
    const state = await invoke<AppStateDto>('get_state');
    const s = state.settings;
    const themeInput = document.querySelector<HTMLInputElement>(
      `input[name="theme"][value="${s.theme}"]`,
    );
    if (themeInput) themeInput.checked = true;
    const intervalInput = document.querySelector<HTMLInputElement>(
      `input[name="interval"][value="${s.refreshMinutes}"]`,
    );
    if (intervalInput) intervalInput.checked = true;
    el<HTMLInputElement>('autostart').checked = s.autoStart;
  } catch {
    /* backend unavailable: keep current control state */
  }
}

window.addEventListener('DOMContentLoaded', async () => {
  el<HTMLImageElement>('logo').src = logoUrl;

  try {
    await initTheme();
  } catch {
    /* backend unavailable: keep default theme */
  }
  await backfill();
  // The window is reused (hidden, not destroyed): re-backfill on every open
  await listen('settings-show', () => {
    backfill().catch(() => undefined);
  });

  el('btn-save').addEventListener('click', () => {
    const theme =
      document.querySelector<HTMLInputElement>('input[name="theme"]:checked')?.value ?? 'system';
    const refreshMinutes = Number(
      document.querySelector<HTMLInputElement>('input[name="interval"]:checked')?.value ?? '5',
    );
    const autoStart = el<HTMLInputElement>('autostart').checked;
    invoke('save_settings', { settings: { theme, refreshMinutes, autoStart } }).catch(
      () => undefined,
    );
  });

  el('btn-close').addEventListener('click', () => {
    invoke('close_settings').catch(() => undefined);
  });

  // Custom title bar drag (SPEC 4.1: DragMove on left button down)
  el('titlebar').addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest('.chrome-close')) return;
    invoke('start_drag').catch(() => undefined);
  });
});

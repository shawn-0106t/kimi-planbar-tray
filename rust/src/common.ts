import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

// ---- Shared IPC types (mirrors src-tauri structs, camelCase) ----

export interface QuotaSegment {
  percent: number;
  resetAt?: string | null;
}

export type ExtraState = 'NotActivated' | 'NoData' | 'Ready';

export interface ExtraInfo {
  state: ExtraState;
  balanceCents?: number | null;
  monthlyEnabled: boolean;
  monthlyUsedCents?: number | null;
  monthlyLimitCents?: number | null;
}

export interface QuotaResult {
  fiveHour?: QuotaSegment | null;
  week?: QuotaSegment | null;
  extra?: ExtraInfo | null;
  fetchedAt: string;
  error?: string | null;
}

export interface UpdateStatus {
  localVersion?: string | null;
  latestVersion?: string | null;
  updateAvailable: boolean;
  checkFailed: boolean;
}

export interface SettingsDto {
  theme: string; // system | light | dark
  refreshMinutes: number;
  autoStart: boolean;
}

export interface AppStateDto {
  quota: QuotaResult | null;
  update: UpdateStatus;
  settings: SettingsDto;
  theme: string; // effective theme: light | dark
}

// ---- Theme ----

export function applyTheme(theme: string): void {
  document.documentElement.dataset.theme = theme === 'dark' ? 'dark' : 'light';
}

/** Apply current effective theme and subscribe to live changes. */
export async function initTheme(): Promise<AppStateDto> {
  const state = await invoke<AppStateDto>('get_state');
  applyTheme(state.theme);
  await listen<string>('theme-changed', (e) => applyTheme(e.payload));
  return state;
}

// ---- Formatting (UI-SPEC 3.3 / 3.5) ----

/** FormatReset: span = at - now, English countdown text. */
export function formatReset(atIso: string): string {
  const spanMs = new Date(atIso).getTime() - Date.now();
  if (Number.isNaN(spanMs)) return '';
  if (spanMs < 0) return 'Resets soon';
  const totalSec = Math.floor(spanMs / 1000);
  const days = Math.floor(totalSec / 86400);
  if (days >= 1) {
    const hours = Math.floor(totalSec / 3600) % 24;
    return `Resets in ${days}d ${hours}h`;
  }
  const totalHours = Math.floor(totalSec / 3600);
  if (totalHours >= 1) {
    const minutes = Math.floor(totalSec / 60) % 60;
    return `Resets in ${totalHours}h ${minutes}m`;
  }
  const minutes = Math.floor(totalSec / 60);
  return `Resets in ${Math.max(1, minutes)}m`;
}

/** FmtYuan: cents -> ¥, fraction omitted for whole yuan. */
export function fmtYuan(cents: number): string {
  if (cents < 0) return '-' + fmtYuan(-cents);
  const yuan = Math.floor(cents / 100);
  const frac = cents % 100;
  return '¥' + yuan + (frac > 0 ? '.' + String(frac).padStart(2, '0') : '');
}

/** {Percent:0}% — display uses the raw (unclamped) percent. */
export function fmtPercent(percent: number): string {
  return `${Math.round(percent)}%`;
}

export function clampPercent(percent: number): number {
  return Math.min(100, Math.max(0, percent));
}

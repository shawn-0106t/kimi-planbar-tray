import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import './theme.css';
import './skills.css';
import logoUrl from './assets/kimi-logo.png';
import { initTheme } from './common';

// Mirrors skills::SkillInfo (camelCase)
interface SkillInfo {
  id: string;
  name: string;
  description: string;
  source: string; // "Kimi Code" | "Agents" | "Plugin: <name>"
  enabled: boolean;
}

function el<T extends HTMLElement>(id: string): T {
  return document.getElementById(id) as T;
}

/** Render the grouped list. All user-controlled text goes through
    textContent — never innerHTML. */
function render(skills: SkillInfo[]): void {
  const list = el('list');
  list.replaceChildren();
  const enabledCount = skills.filter((s) => s.enabled).length;
  el('summary-text').textContent = skills.length
    ? `${skills.length} skills · ${enabledCount} enabled`
    : 'No skills found';

  let lastSource = '';
  for (const s of skills) {
    if (s.source !== lastSource) {
      lastSource = s.source;
      const group = document.createElement('div');
      group.className = 'group-title';
      group.textContent = s.source;
      list.appendChild(group);
    }
    const item = document.createElement('div');
    item.className = s.enabled ? 'skill' : 'skill disabled';

    const head = document.createElement('div');
    head.className = 'skill-head';
    const name = document.createElement('span');
    name.className = 'skill-name';
    name.textContent = s.name;
    head.appendChild(name);
    if (!s.enabled) {
      const badge = document.createElement('span');
      badge.className = 'skill-badge';
      badge.textContent = 'disabled';
      head.appendChild(badge);
    }

    const desc = document.createElement('div');
    desc.className = 'skill-desc';
    desc.textContent = s.description || '(no description)';
    if (s.description) item.title = s.description;

    item.appendChild(head);
    item.appendChild(desc);
    list.appendChild(item);
  }
}

/** Load from the backend cache; refresh=true forces a rescan (SPEC 12.2). */
async function load(refresh: boolean): Promise<void> {
  try {
    render(await invoke<SkillInfo[]>('get_skills', { refresh }));
  } catch {
    el('summary-text').textContent = 'Failed to load skills';
  }
}

window.addEventListener('DOMContentLoaded', async () => {
  el<HTMLImageElement>('logo').src = logoUrl;

  try {
    await initTheme();
  } catch {
    /* backend unavailable: keep default theme */
  }

  // Event-driven only: the window page loads hidden at app start, so the scan
  // must NOT run here — it runs when open_skills emits 'skills-show'
  // (SPEC 12.2: zero cost until the window is actually opened).
  // NOT awaited: registration must never block the rest of the init.
  listen('skills-show', () => {
    load(false).catch(() => undefined);
  }).catch(() => undefined);

  el('btn-refresh').addEventListener('click', () => {
    load(true).catch(() => undefined);
  });
  el('btn-close').addEventListener('click', () => {
    invoke('close_skills').catch(() => undefined);
  });

  // Custom title bar drag (same pattern as settings.ts)
  el('titlebar').addEventListener('mousedown', (e) => {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest('.chrome-close')) return;
    invoke('start_drag').catch(() => undefined);
  });
});

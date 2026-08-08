import { invoke } from '@tauri-apps/api/core';
import './theme.css';
import './menu.css';
import { initTheme } from './common';

window.addEventListener('DOMContentLoaded', async () => {
  try {
    await initTheme();
  } catch {
    /* backend unavailable: keep default theme */
  }

  // Height is content-sized (WPF SizeToContent): report the real height so the
  // backend can position the menu correctly (flip-up near the screen bottom).
  const report = () => {
    const root = document.getElementById('root');
    if (!root) return;
    const height = root.offsetHeight + 48; // + 2 * 24px margin (shadow fade room)
    invoke('menu_height', { height }).catch(() => undefined);
  };
  report();
  window.setTimeout(report, 100); // re-report after fonts settle

  document.querySelectorAll<HTMLButtonElement>('.menu-item').forEach((btn) => {
    btn.addEventListener('click', () => {
      invoke('menu_action', { action: btn.dataset.action ?? '' }).catch(() => undefined);
    });
  });
});

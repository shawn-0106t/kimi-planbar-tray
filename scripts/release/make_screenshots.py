#!/usr/bin/env python3
"""Regenerate docs/screenshot-*.png for the README from the built frontend.

Renders rust/dist/index.html with a headless Chromium browser - Chrome, or
Edge when Chrome is absent (same engine, same headless flags; the Rust exe
has no --screenshot arg; only the frozen WPF edition does). Injects theme +
mock quota data into a temp copy of the page, screenshots, crops to 424x520.

Browser selection: KPT_CHROME env var wins; otherwise the first existing of
Chrome (machine-wide, 32-bit, per-user) then Edge is used. Each render runs
with a throwaway --user-data-dir in the system temp so an already-running
browser's profile singleton cannot hijack the screenshot.

Prereq: cd rust && npm run build  (dist must be current)

Variants (mirrors the WPF --screenshot [--mock] outputs):
  screenshot-{light,dark}.png            realistic data, Extra = "No data"
  screenshot-extra-mock-{light,dark}.png mock data, Extra = ¥12.34 + monthly bar
"""
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]  # repo root (scripts/release/)
DIST = ROOT / "rust" / "dist" / "index.html"
W, H = 424, 520

# Chromium-family browsers, tried in order: Chrome (machine-wide, 32-bit,
# per-user install) then Edge, which shares the engine and the headless
# flags. KPT_CHROME overrides the list for any other Chromium browser.
BROWSER_CANDIDATES = (
    r"C:\Program Files\Google\Chrome\Application\chrome.exe",
    r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    os.path.expandvars(r"%LOCALAPPDATA%\Google\Chrome\Application\chrome.exe"),
    r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
    r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
)


def find_browser() -> str:
    env = os.environ.get("KPT_CHROME", "").strip().strip('"')
    if env:
        if not Path(env).is_file():
            sys.exit(f"KPT_CHROME is set but not a file: {env} "
                     "(check for stray quotes/whitespace)")
        return env
    for cand in BROWSER_CANDIDATES:
        if Path(cand).is_file():
            return cand
    sys.exit("no Chrome/Edge found - set KPT_CHROME to a Chromium browser exe")


def remove_dir_quietly(path: str) -> bool:
    """Best-effort rmtree. Chromium spawns crashpad_handler-style children that
    can hold handles in the profile for a moment after the browser dies, so
    retry once before giving up."""
    shutil.rmtree(path, ignore_errors=True)
    if not Path(path).exists():
        return True
    time.sleep(0.5)
    shutil.rmtree(path, ignore_errors=True)
    return not Path(path).exists()

MOCK_STANDARD = dict(week="12%", week_w="12%", week_r="Resets in 2d 18h",
                     five="31%", five_w="31%", five_r="Resets in 3h 33m",
                     extra="No data", extra_w="0%",
                     extra_t="Used ¥0 this month / ¥100 limit")
MOCK_EXTRA = dict(week="68%", week_w="68%", week_r="Resets in 3d 23h",
                  five="42%", five_w="42%", five_r="Resets in 3h 29m",
                  extra="¥12.34", extra_w="45.67%",
                  extra_t="Used ¥45.67 this month / ¥100 limit")

JS_TEMPLATE = """
<script>
window.addEventListener('load', () => {
  const t = (id, s) => { document.getElementById(id).textContent = s; };
  const w = (id, p) => { document.getElementById(id).style.width = p; };
  t('week-pct', %(week)r); w('week-fill', %(week_w)r); t('week-reset', %(week_r)r);
  t('five-pct', %(five)r); w('five-fill', %(five_w)r); t('five-reset', %(five_r)r);
  t('extra-balance', %(extra)r);
  document.getElementById('extra-monthly').hidden = false;
  w('extra-fill', %(extra_w)r);
  t('extra-monthly-text', %(extra_t)r);
  t('cli-version', '0.39.1');
  t('last-updated', 'Updated 18:14');
});
</script>
</body>"""


def render(browser: str, theme: str, mock: dict, out: Path) -> None:
    html = DIST.read_text(encoding="utf-8")
    html = html.replace('<html lang="en">',
                        f'<html lang="en" data-theme="{theme}">')
    html = html.replace(" crossorigin", "")
    html = html.replace("<body>",
                        f'<body class="enter" style="width:{W}px;height:{H}px;overflow:hidden">')
    html = html.replace("</body>", JS_TEMPLATE % {k: v for k, v in mock.items()})
    tmp_html = DIST.parent / f"_shot-{out.stem}.html"
    tmp_png = out.with_suffix(".tmp.png")
    profile = None
    try:
        tmp_html.write_text(html, encoding="utf-8")
        profile = tempfile.mkdtemp(prefix="kpt-shot-")
        subprocess.run(
            [browser, "--headless", "--disable-gpu", "--allow-file-access-from-files",
             "--no-first-run", f"--user-data-dir={profile}",
             f"--window-size={W},{H}", "--virtual-time-budget=5000",
             f"--screenshot={tmp_png}", tmp_html.as_uri()],
            check=True, capture_output=True, timeout=120)
        from PIL import Image
        Image.open(tmp_png).crop((0, 0, W, H)).save(out)
        print(f"written: {out.relative_to(ROOT)}")
    finally:
        tmp_html.unlink(missing_ok=True)
        tmp_png.unlink(missing_ok=True)
        if profile is not None and not remove_dir_quietly(profile):
            print(f"warning: temp profile not removed: {profile}", file=sys.stderr)


def main() -> None:
    if not DIST.is_file():
        sys.exit("rust/dist/index.html missing - run `npm run build` in rust/ first")
    browser = find_browser()
    print(f"browser: {browser}")
    for theme in ("light", "dark"):
        render(browser, theme, MOCK_STANDARD, ROOT / "docs" / f"screenshot-{theme}.png")
        render(browser, theme, MOCK_EXTRA, ROOT / "docs" / f"screenshot-extra-mock-{theme}.png")


if __name__ == "__main__":
    main()

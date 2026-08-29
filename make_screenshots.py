#!/usr/bin/env python3
"""Regenerate docs/screenshot-*.png for the README from the built frontend.

Renders rust/dist/index.html with headless Chrome (the Rust exe has no
--screenshot arg; only the frozen WPF edition does). Injects theme + mock
quota data into a temp copy of the page, screenshots, crops to 424x520.

Prereq: cd rust && npm run build  (dist must be current)

Variants (mirrors the WPF --screenshot [--mock] outputs):
  screenshot-{light,dark}.png            realistic data, Extra = "No data"
  screenshot-extra-mock-{light,dark}.png mock data, Extra = ¥12.34 + monthly bar
"""
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIST = ROOT / "rust" / "dist" / "index.html"
CHROME = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
W, H = 424, 520

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


def render(theme: str, mock: dict, out: Path) -> None:
    html = DIST.read_text(encoding="utf-8")
    html = html.replace('<html lang="en">',
                        f'<html lang="en" data-theme="{theme}">')
    html = html.replace(" crossorigin", "")
    html = html.replace("<body>",
                        f'<body class="enter" style="width:{W}px;height:{H}px;overflow:hidden">')
    html = html.replace("</body>", JS_TEMPLATE % {k: v for k, v in mock.items()})
    tmp_html = DIST.parent / f"_shot-{out.stem}.html"
    tmp_png = out.with_suffix(".tmp.png")
    tmp_html.write_text(html, encoding="utf-8")
    try:
        subprocess.run(
            [CHROME, "--headless", "--disable-gpu", "--allow-file-access-from-files",
             f"--window-size={W},{H}", "--virtual-time-budget=5000",
             f"--screenshot={tmp_png}", tmp_html.as_uri()],
            check=True, capture_output=True, timeout=120)
        from PIL import Image
        Image.open(tmp_png).crop((0, 0, W, H)).save(out)
        print(f"written: {out.relative_to(ROOT)}")
    finally:
        tmp_html.unlink(missing_ok=True)
        tmp_png.unlink(missing_ok=True)


def main() -> None:
    if not DIST.is_file():
        sys.exit("rust/dist/index.html missing - run `npm run build` in rust/ first")
    for theme in ("light", "dark"):
        render(theme, MOCK_STANDARD, ROOT / "docs" / f"screenshot-{theme}.png")
        render(theme, MOCK_EXTRA, ROOT / "docs" / f"screenshot-extra-mock-{theme}.png")


if __name__ == "__main__":
    main()

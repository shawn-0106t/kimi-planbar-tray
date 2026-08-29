"""Verify SVG path data inlined in rust/index.html matches the source icon library byte-for-byte."""
import re
import sys

ICONS_DIR = r"C:/Users/rexxa/.kimi-code/skills/rationalism-design/assets/icons"
INDEX = r"rust/index.html"

def paths_of(text):
    return re.findall(r'<path d="([^"]+)"', text)

def btn_svg_paths(html, btn_id):
    m = re.search(r'id="' + btn_id + r'"[^>]*>\s*<svg[^>]*>(.*?)</svg>', html, re.S)
    if not m:
        return None
    return paths_of(m.group(1))

def main():
    html = open(INDEX, encoding="utf-8").read()
    cases = [
        ("btn-console", "Browser.svg"),
        ("btn-refresh", "Refresh.svg"),
        ("btn-settings", "Setting.svg"),
    ]
    ok = True
    for btn_id, svg_file in cases:
        src = open(ICONS_DIR + "/" + svg_file, encoding="utf-8").read()
        expected = paths_of(src)
        actual = btn_svg_paths(html, btn_id)
        if actual == expected:
            print(f"OK   {btn_id} == {svg_file} ({len(expected)} path(s))")
        else:
            ok = False
            print(f"FAIL {btn_id} vs {svg_file}: expected {len(expected)} path(s), got {None if actual is None else len(actual)}")
    # Hand-drawn power glyph: just check it exists and parses as non-empty
    exit_paths = btn_svg_paths(html, "btn-exit")
    if exit_paths and len(exit_paths) == 1 and len(exit_paths[0]) > 50:
        print("OK   btn-exit hand-drawn power glyph present")
    else:
        ok = False
        print("FAIL btn-exit power glyph missing/short")
    sys.exit(0 if ok else 1)

main()

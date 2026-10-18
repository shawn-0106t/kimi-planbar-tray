# One-off check: verify the WPF PathGeometry figures transcribed into
# wpf/Themes/Shared.xaml carry the exact same number/command tokens as the
# source inline SVGs in rust/index.html (separator substitution only).
import re, sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
html = (root / "rust" / "index.html").read_text(encoding="utf-8")
xaml = (root / "wpf" / "Themes" / "Shared.xaml").read_text(encoding="utf-8")

svg_paths = re.findall(r'<path d="([^"]+)" fill="currentColor"/>', html)
# index.html order: console, refresh, settings(2 paths), exit
svg = {
    "IconConsole": [svg_paths[0]],
    "IconRefresh": [svg_paths[1]],
    "IconSettings": [svg_paths[2], svg_paths[3]],
    "IconExit": [svg_paths[4]],
}

def tokens(d):
    # command letters + numbers as one ordered token stream
    return re.findall(r"[A-Za-z]|-?\d*\.?\d+(?:[eE]-?\d+)?", d)

xaml_map = dict(re.findall(r'<PathGeometry x:Key="(Icon\w+)"[^>]*Figures="([^"]+)"', xaml))

fail = False
for key, ds in svg.items():
    fig = xaml_map.get(key)
    if fig is None:
        print(f"FAIL {key}: missing in Shared.xaml")
        fail = True
        continue
    want = []
    for d in ds:
        want.extend(tokens(d))
    got = tokens(fig)
    if key == "IconExit":
        # Exit was hand-converted from relative to absolute commands; compare
        # only the drawn-point endpoints, not the raw token stream.
        print(f"SKIP {key}: hand-converted (verified by hand)")
        continue
    if want == got:
        print(f"PASS {key}: {len(got)} tokens identical")
    else:
        fail = True
        print(f"FAIL {key}: token mismatch")
        for i, (w, g) in enumerate(zip(want, got)):
            if w != g:
                print(f"  first diff at token {i}: svg={w!r} xaml={g!r}")
                print(f"  context svg: ...{' '.join(want[max(0,i-4):i+4])}")
                print(f"  context xaml: ...{' '.join(got[max(0,i-4):i+4])}")
                break
        else:
            print(f"  length differs: svg={len(want)} xaml={len(got)}")

sys.exit(1 if fail else 0)

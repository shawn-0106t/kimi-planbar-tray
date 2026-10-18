# Verify that the WPF PathGeometry figures transcribed into
# wpf/Themes/Shared.xaml draw the same shapes as the source inline SVGs in
# rust/index.html. Both sides are parsed into an absolute-command token
# stream (relative commands expanded, H/V folded into L), so hand
# conversions are checked too, not just verbatim transcriptions.
#
# Usage: PYTHONUTF8=1 python scripts/diagnostics/verify_wpf_icon_paths.py
# Exit 0 = all four icons identical, 1 = any mismatch.
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
svg_text = (ROOT / "rust" / "index.html").read_text(encoding="utf-8")
xaml_text = (ROOT / "wpf" / "Themes" / "Shared.xaml").read_text(encoding="utf-8")

# button id in index.html -> x:Key of the transcribed geometry in Shared.xaml
BUTTONS = {
    "btn-console": "IconConsole",
    "btn-refresh": "IconRefresh",
    "btn-settings": "IconSettings",
    "btn-exit": "IconExit",
}

TOKEN = re.compile(r"([A-Za-z])|(-?\d*\.?\d+(?:[eE]-?\d+)?)")


def tokenize(d):
    """SVG/WPF path markup -> alternating command-letter / float tokens."""
    return [m.group(1) if m.group(1) else float(m.group(0))
            for m in TOKEN.finditer(d)]


def expand_absolute(tokens):
    """Expand relative commands to absolute and normalize into a list of
    (CMD, [floats...]) with uppercase commands and H/V folded into L.
    Handles implicit command repetition (extra coordinate sets repeat the
    last command; after M they repeat as L) and the arc's flag-terminated
    parameter block. Only endpoints are absolutized; radii/rotation/flags
    stay as written."""
    ARITY = {"M": 2, "L": 2, "H": 1, "V": 1, "C": 6, "S": 4, "Q": 4, "T": 2}
    # Known limitation (documented, fails loudly if ever hit): the tokenizer
    # splits on whitespace/commas, so the legal concatenated arc-flag form
    # (`a1 1 0 011 1`) would parse `011` as one number and misalign the
    # parameter block. None of the four audited icons use it; the icons fail
    # the comparison loudly rather than silently passing.
    out = []
    cmd = None
    i = 0
    x = y = 0.0
    n = len(tokens)
    while i < n:
        if isinstance(tokens[i], str):
            cmd = tokens[i]
            i += 1
            if cmd in ("Z", "z"):
                out.append(("Z", []))
                continue
        if cmd is None:
            raise ValueError("coordinates before any command")
        rel = cmd.islower()
        c = cmd.upper()
        # implicit repetition after M continues as L (SVG/WPF semantics)
        repeat_cmd = "L" if c == "M" else c
        while True:
            if c == "A":
                rx, ry, rot, laf, sf = (float(v) for v in tokens[i:i + 5])
                ex, ey = float(tokens[i + 5]), float(tokens[i + 6])
                i += 7
                if rel:
                    ex, ey = x + ex, y + ey
                out.append(("A", [rx, ry, rot, laf, sf, ex, ey]))
                x, y = ex, ey
            elif c in ("H", "V"):
                v = float(tokens[i])
                i += 1
                if c == "H":
                    x = x + v if rel else v
                else:
                    y = y + v if rel else v
                out.append(("L", [x, y]))
            else:
                k = ARITY[c]
                vals = [float(v) for v in tokens[i:i + k]]
                i += k
                if rel:
                    for j in range(0, k, 2):
                        vals[j] += x
                        vals[j + 1] += y
                x, y = vals[-2], vals[-1]
                out.append((c, vals))
            if i >= n or isinstance(tokens[i], str):
                break
            # implicit repetition inherits the original command's relative-ness
            # (SVG/WPF spec: `l1 1 2 2` draws two RELATIVE linetos; the pairs
            # after a lowercase `m` are relative linetos too)
            c = repeat_cmd
            rel = cmd.islower()
    return out


def svg_paths_by_button():
    """button id -> list of `d` attributes of its inline <path> elements."""
    result = {}
    for m in re.finditer(r'<button class="btn" id="(btn-[a-z]+)".*?</button>',
                         svg_text, re.S):
        result[m.group(1)] = re.findall(r'<path d="([^"]+)"', m.group(0))
    return result


def segments_equal(want, got, tol=1e-9):
    """Segment lists equal modulo float representation (relative-command
    accumulation differs from a hand-converted absolute value by ~1e-15)."""
    if len(want) != len(got):
        return False
    for (cw, vw), (cg, vg) in zip(want, got):
        if cw != cg or len(vw) != len(vg):
            return False
        for a, b in zip(vw, vg):
            if abs(a - b) > tol:
                return False
    return True


def main():
    svg_map = svg_paths_by_button()
    figs = {k: fig for k, fig in re.findall(
        r'<PathGeometry x:Key="(\w+)"[^>]*Figures="([^"]+)"', xaml_text)}
    failures = []
    for btn, key in BUTTONS.items():
        ds = svg_map.get(btn)
        if not ds:
            failures.append(f"{key}: no inline svg found for {btn}")
            continue
        fig = figs.get(key)
        if fig is None:
            failures.append(f"{key}: missing in Shared.xaml")
            continue
        want = []
        for d in ds:
            want.extend(expand_absolute(tokenize(d)))
        got = expand_absolute(tokenize(fig))
        if segments_equal(want, got):
            print(f"PASS {key}: {len(got)} absolute segments identical")
        else:
            failures.append(f"{key}: geometry differs "
                            f"({len(want)} vs {len(got)} segments)")
            for idx, (w, g) in enumerate(zip(want, got)):
                if w != g:
                    failures.append(f"  first diff at segment {idx}: "
                                    f"svg={w} xaml={g}")
                    break
    if failures:
        print("\n".join(failures))
        return 1
    print("all four icons verified against rust/index.html")
    return 0


if __name__ == "__main__":
    sys.exit(main())

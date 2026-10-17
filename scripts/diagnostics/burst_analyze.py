"""Analyze burst-capture PNGs: per-frame mean luminance and bright-pixel ratio."""
import sys
from pathlib import Path

from PIL import Image


def main(dir_path: str, prefix: str, threshold: int = 200) -> None:
    files = sorted(Path(dir_path).glob(f"{prefix}_*.png"))
    prev_mean = None
    for f in files:
        im = Image.open(f).convert("L").resize((270, 323))
        px = list(im.getdata())
        mean = sum(px) / len(px)
        bright = 100.0 * sum(1 for v in px if v >= threshold) / len(px)
        marker = ""
        if prev_mean is None or abs(mean - prev_mean) > 0.15 or bright > 3:
            marker = " <=="
            prev_mean = mean
        print(f"{f.name}  mean={mean:6.1f}  bright%={bright:5.2f}{marker}")


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2])

"""Row-band luminance profile of a burst frame: 10 horizontal bands."""
import sys
from pathlib import Path
from PIL import Image

f = Path(sys.argv[1])
im = Image.open(f).convert("L")
w, h = im.size
px = im.load()
band = h // 10
for i in range(10):
    y0, y1 = i * band, min((i + 1) * band, h)
    total = bright = 0
    n = 0
    for y in range(y0, y1):
        for x in range(0, w, 3):
            v = px[x, y]
            total += v
            n += 1
            if v >= 200:
                bright += 1
    print(f"band {i} y[{y0}:{y1}]  mean={total/n:6.1f}  bright%={100.0*bright/n:5.1f}")

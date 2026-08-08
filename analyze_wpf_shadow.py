import sys

from PIL import Image

# Usage: python analyze_wpf_shadow.py <png-path>
# Verifies the drop shadow fully decays inside the window: all outermost
# rows/cols must have alpha 0, and the opaque visual card must keep its
# designed size (MainWindow: 368x456 card inside a 424x512 window).
path = sys.argv[1] if len(sys.argv) > 1 else r"C:/Users/rexxa/AppData/Local/Temp/wpf-panel-dark.png"
img = Image.open(path).convert("RGBA")
w, h = img.size
print("size:", img.size)

# Alpha along the outermost rows/cols (window edge) — nonzero means the
# shadow is still visible when the window clips it (hard-cut frame).
def row_alpha(y):
    return [img.getpixel((x, y))[3] for x in range(0, w, 20)]

def col_alpha(x):
    return [img.getpixel((x, y))[3] for y in range(0, h, 20)]

print("top edge    y=0 :", row_alpha(0))
print("row         y=3 :", row_alpha(3))
print("row         y=6 :", row_alpha(6))
print("row         y=10:", row_alpha(10))
print("left edge   x=0 :", col_alpha(0))
print("right edge  x=w-1:", col_alpha(w - 1))
print("bottom edge y=h-1:", row_alpha(h - 1))

edge_max = max(
    max(img.getpixel((x, 0))[3] for x in range(w)),
    max(img.getpixel((x, h - 1))[3] for x in range(w)),
    max(img.getpixel((0, y))[3] for y in range(h)),
    max(img.getpixel((w - 1, y))[3] for y in range(h)),
)
print("max alpha on any window-edge pixel:", edge_max, "(0 = shadow fully faded)")

# Where does alpha reach full opacity along the top-center vertical line?
cx = w // 2
for y in range(0, 60):
    if img.getpixel((cx, y))[3] == 255:
        print(f"first fully opaque pixel at top center: y={y}")
        break
alphas = [img.getpixel((cx, y))[3] for y in range(0, 12)]
print("alpha ramp top center y=0..11:", alphas)

# Visual card bounds = opaque (alpha 255) region; designed card is 368x456.
xs = [x for x in range(w) for y in range(0, h, 4) if img.getpixel((x, y))[3] == 255]
ys = [y for y in range(h) for x in range(0, w, 4) if img.getpixel((x, y))[3] == 255]
if xs and ys:
    cw = max(xs) - min(xs) + 1
    ch = max(ys) - min(ys) + 1
    print(f"opaque card bounds: x={min(xs)}..{max(xs)} y={min(ys)}..{max(ys)} -> {cw}x{ch} (expect 368x456)")

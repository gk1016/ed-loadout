"""The LOADOUT mark: a bracketed hexagon with a horizontal rule. Original
geometry (no third-party art), on a 64-unit grid; writes icons/loadout.svg and
icons/loadout.ico (16..256) from the same numbers."""
import math
import sys
from PIL import Image, ImageDraw

out = sys.argv[1]
GROUND, SAND = "#070B14", "#E6C27A"
R = 16.0
HEX = [(32 + R * math.cos(math.radians(a)), 32 + R * math.sin(math.radians(a))) for a in range(-90, 270, 60)]
HALF = R * math.cos(math.radians(30))
LEFT = [(15, 9), (6, 9), (6, 55), (15, 55)]
RIGHT = [(49, 9), (58, 9), (58, 55), (49, 55)]
RULE = [(32 - HALF, 32), (32 + HALF, 32)]
STROKE = 4.0


def pts(p):
    return " ".join(f"{x:.2f},{y:.2f}" for x, y in p)


svg = f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64" width="64" height="64">
  <!-- LOADOUT mark: a bracketed hexagon with a horizontal rule. -->
  <rect width="64" height="64" fill="{GROUND}"/>
  <g fill="none" stroke="{SAND}" stroke-width="{STROKE:g}" stroke-linejoin="miter" stroke-linecap="butt">
    <polyline points="{pts(LEFT)}"/>
    <polyline points="{pts(RIGHT)}"/>
    <polygon points="{pts(HEX)}"/>
    <line x1="{RULE[0][0]:.2f}" y1="32" x2="{RULE[1][0]:.2f}" y2="32"/>
  </g>
</svg>
"""
with open(f"{out}/loadout.svg", "w", encoding="utf-8", newline="\n") as f:
    f.write(svg)

S = 16  # supersample: draw at 1024, reduce
img = Image.new("RGBA", (64 * S, 64 * S), GROUND)
d = ImageDraw.Draw(img)
w = int(STROKE * S)


def line(p, closed=False):
    q = [(x * S, y * S) for x, y in p]
    if closed:
        q.append(q[0])
    d.line(q, fill=SAND, width=w, joint="curve")


line(LEFT)
line(RIGHT)
line(HEX, closed=True)
line(RULE)
big = img.resize((256, 256), Image.LANCZOS)
big.save(f"{out}/loadout.ico", sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)])
print("wrote", out)

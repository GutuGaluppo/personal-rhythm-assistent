"""Placeholder menu-bar state icons (64x64 RGBA PNG, supersampled). Pure stdlib.

Until the approved Design System icons arrive these are deliberately plain:
  default  filled circle
  pause    two bars
  on_fire  warm filled circle (a colour, not a flame; nothing animates)
  silent   ring
State is never conveyed by colour alone: the tray tooltip states it in words.
Run from the repo root: python3 scripts/make-state-icons.py
"""
import os, struct, zlib

OUT = "src-tauri/icons/tray"
SIZE, SS = 64, 4
TEAL, WARM, GREY = (79, 127, 138), (217, 138, 61), (120, 130, 135)

def inside(kind, x, y):
    cx = cy = 0.5
    dx, dy = x - cx, y - cy
    r = (dx * dx + dy * dy) ** 0.5
    if kind in ("default", "on_fire"):
        return r <= 0.36
    if kind == "silent":
        return 0.26 <= r <= 0.38
    if kind == "pause":
        return (0.30 <= x <= 0.44 or 0.56 <= x <= 0.70) and 0.24 <= y <= 0.76
    raise ValueError(kind)

def render(kind, color):
    rows = []
    for py in range(SIZE):
        row = bytearray([0])
        for px in range(SIZE):
            hits = sum(
                inside(kind, (px + (sx + 0.5) / SS) / SIZE, (py + (sy + 0.5) / SS) / SIZE)
                for sy in range(SS) for sx in range(SS)
            )
            row += bytes((*color, round(255 * hits / (SS * SS))))
        rows.append(bytes(row))
    def chunk(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(b"".join(rows), 9)) + chunk(b"IEND", b""))

os.makedirs(OUT, exist_ok=True)
for kind, color in [("default", TEAL), ("pause", TEAL), ("on_fire", WARM), ("silent", GREY)]:
    with open(f"{OUT}/{kind}.png", "wb") as f:
        f.write(render(kind, color))
print("wrote", sorted(os.listdir(OUT)))

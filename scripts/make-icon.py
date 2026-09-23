"""Prepares the source app icon (1024x1024, transparent, full-bleed) from the
approved Design System v0.1 art in src-tauri/icons/source/ -- the Normal-state
glass icon (ring + closed "sleepy" eyes, cool Azul Suave -> Menta palette).

Unlike the other scripts in this directory, this one needs Pillow (the source
art is a real raster image, not something worth re-deriving by hand):
    pip install pillow

After running this, feed the output to `npx tauri icon` to (re)generate every
platform size:
    python3 scripts/make-icon.py
    npx tauri icon src-tauri/icons/icon-source.png

Run from the repo root.
"""
from PIL import Image

SRC = "src-tauri/icons/source/dreamy_silent_meditation_app_icon.png"
OUT = "src-tauri/icons/icon-source.png"
SIZE = 1024

if __name__ == "__main__":
    im = Image.open(SRC).convert("RGBA")
    im = im.resize((SIZE, SIZE), Image.LANCZOS)
    im.save(OUT)
    print(f"wrote {OUT} ({SIZE}x{SIZE})")

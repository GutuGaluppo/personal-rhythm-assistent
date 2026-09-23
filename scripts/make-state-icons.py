"""Menu-bar state icons (64x64 RGBA PNG), derived from the approved Design
System v0.1 art in src-tauri/icons/source/. State is never conveyed by color
alone: the tray tooltip states it in words (see src-tauri/src/app/tray.rs).

The source renders are a soft glass panel that reads beautifully at app-icon
size but washes out at 64px against the menu bar, so each is boosted
(saturation/contrast/brightness) before downscaling -- tuned by eye, not a
faithful 1:1 crop of the source art. Needs Pillow: pip install pillow

Run from the repo root: python3 scripts/make-state-icons.py
"""
from PIL import Image, ImageEnhance

SRC = "src-tauri/icons/source"
OUT = "src-tauri/icons/tray"
SIZE = 64

# (source file, saturation, contrast, brightness)
STATES = {
    "default": (f"{SRC}/dreamy_silent_meditation_app_icon.png", 2.2, 1.5, 0.92),
    "pause": (f"{SRC}/dreamy_silent_meditation_app_icon.png", 2.2, 1.5, 0.92),
    "silent": (f"{SRC}/dreamy_glass_silence_icon.png", 1.6, 1.3, 0.95),
    "on_fire": (f"{SRC}/dreamy_glass_on-fire_icon.png", 1.3, 1.15, 0.97),
}


def render(src_path: str, sat: float, contrast: float, brightness: float) -> Image.Image:
    im = Image.open(src_path).convert("RGBA")
    r, g, b, a = im.split()
    rgb = Image.merge("RGB", (r, g, b))
    rgb = ImageEnhance.Color(rgb).enhance(sat)
    rgb = ImageEnhance.Contrast(rgb).enhance(contrast)
    rgb = ImageEnhance.Brightness(rgb).enhance(brightness)
    r2, g2, b2 = rgb.split()
    out = Image.merge("RGBA", (r2, g2, b2, a))
    return out.resize((SIZE, SIZE), Image.LANCZOS)


if __name__ == "__main__":
    for name, (src, sat, contrast, brightness) in STATES.items():
        render(src, sat, contrast, brightness).save(f"{OUT}/{name}.png")
    print("wrote", sorted(f"{k}.png" for k in STATES))

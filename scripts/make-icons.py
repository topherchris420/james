"""Draws the James app icon and writes every packaged size.

The icon is original geometric artwork: a teal "J" hook on a dark rounded
square, in the desktop app's own colors. Regenerate with Pillow 10 or newer:

    python3 scripts/make-icons.py
"""

from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parent.parent
TEAL = (62, 193, 178, 255)
SCALE = 4  # draw large, then downsample for smooth edges
SIZE = 1024


def draw(size: int = SIZE) -> Image.Image:
    big = size * SCALE
    u = big / 100  # one percent of the canvas
    image = Image.new("RGBA", (big, big), (0, 0, 0, 0))
    pen = ImageDraw.Draw(image)

    # Dark rounded square, slightly inset like platform app icons.
    pen.rounded_rectangle(
        (6 * u, 6 * u, 94 * u, 94 * u), radius=20 * u, fill=(16, 33, 37, 255)
    )
    pen.rounded_rectangle(
        (6 * u, 6 * u, 94 * u, 94 * u),
        radius=20 * u,
        outline=(38, 74, 78, 255),
        width=round(1.2 * u),
    )

    stroke = 11 * u
    # Stem of the J.
    stem_x = 67 * u
    pen.rectangle((stem_x - stroke / 2, 24 * u, stem_x + stroke / 2, 58 * u), fill=TEAL)
    # Hook: the lower half of a ring whose right edge continues the stem.
    radius = 18 * u
    cx, cy = stem_x - radius, 58 * u
    pen.arc(
        (cx - radius - stroke / 2, cy - radius - stroke / 2,
         cx + radius + stroke / 2, cy + radius + stroke / 2),
        start=0, end=180, fill=TEAL, width=round(stroke),
    )
    # Rounded ends on the stem top and the hook tip.
    for x, y in ((stem_x, 24 * u), (cx - radius, cy)):
        pen.ellipse((x - stroke / 2, y - stroke / 2, x + stroke / 2, y + stroke / 2), fill=TEAL)
    # A small dot: James notices, and waits for you.
    dot = 4.6 * u
    pen.ellipse((39 * u - dot, 30 * u - dot, 39 * u + dot, 30 * u + dot), fill=TEAL)

    return image.resize((size, size), Image.LANCZOS)


def main() -> None:
    master = draw()
    assets = ROOT / "crates/james-desktop/assets"
    installer = ROOT / "installer"
    assets.mkdir(parents=True, exist_ok=True)
    master.resize((256, 256), Image.LANCZOS).save(assets / "james-256.png", optimize=True)
    master.resize((512, 512), Image.LANCZOS).save(installer / "james-512.png", optimize=True)
    master.save(
        installer / "james.ico",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    master.save(installer / "james.icns")


if __name__ == "__main__":
    main()

"""The mouse pointer while Mundwerk listens: the usual arrow (its tip, the hot spot, at the top left) with a red
microphone badge. Writes the raw pixels the app builds in (32 x 32, BGRA, top row first) and a PNG to look at.

    python3 tools/gen_pointer.py
"""
from pathlib import Path

from PIL import Image, ImageDraw

SIZE = 32
ICONS = Path(__file__).resolve().parent.parent / "src-tauri" / "icons"
# The arrow's outline as Windows draws its standard pointer, tip at (0, 0).
ARROW = [(0, 0), (0, 21), (5, 16), (8, 23), (11, 22), (8, 15), (15, 15)]
BADGE_CENTRE = (23, 23)
BADGE_RADIUS = 8
RED = (229, 72, 77, 255)
WHITE = (255, 255, 255, 255)
BLACK = (0, 0, 0, 255)


def draw() -> Image.Image:
    image = Image.new("RGBA", (SIZE, SIZE), (0, 0, 0, 0))
    pen = ImageDraw.Draw(image)
    pen.polygon(ARROW, fill=WHITE, outline=BLACK)
    x, y = BADGE_CENTRE
    r = BADGE_RADIUS
    pen.ellipse((x - r, y - r, x + r, y + r), fill=RED, outline=WHITE)
    # The microphone: capsule, cradle, stand.
    pen.rounded_rectangle((x - 2, y - 5, x + 2, y + 1), radius=2, fill=WHITE)
    pen.arc((x - 4, y - 3, x + 4, y + 3), start=0, end=180, fill=WHITE)
    pen.line((x, y + 3, x, y + 5), fill=WHITE)
    return image


def main() -> None:
    image = draw()
    image.save(ICONS / "pointer-listening.png")
    red, green, blue, alpha = image.split()
    bgra = Image.merge("RGBA", (blue, green, red, alpha)).tobytes()
    (ICONS / "pointer-listening.bgra").write_bytes(bgra)
    print(f"{len(bgra)} bytes, {ICONS}")


if __name__ == "__main__":
    main()

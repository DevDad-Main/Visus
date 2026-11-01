"""Renders the Visus app icon (assets/visus.ico, assets/tray32.rgba, assets/icon.png). Needs Pillow."""
import math
from PIL import Image, ImageDraw, ImageFilter

S = 2048  # supersampled canvas
OUT = "crates/visus-app/assets/"


def lerp(a, b, t):
    return tuple(int(a[i] + (b[i] - a[i]) * t) for i in range(3))


def gradient(size, c1, c2):
    img = Image.new("RGB", (size, size))
    px = img.load()
    for y in range(size):
        for x in range(size):
            px[x, y] = lerp(c1, c2, (x + y) / (2 * size))
    return img


def bezier(p0, p1, p2, n=60):
    return [((1 - t) ** 2 * p0[0] + 2 * (1 - t) * t * p1[0] + t * t * p2[0], (1 - t) ** 2 * p0[1] + 2 * (1 - t) * t * p1[1] + t * t * p2[1]) for t in [i / n for i in range(n + 1)]]


# tile with a diagonal indigo-to-blue gradient
tile = gradient(S, (104, 72, 240), (10, 150, 255)).convert("RGBA")
glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
gd = ImageDraw.Draw(glow)
gd.ellipse((S * 0.1, -S * 0.35, S * 0.9, S * 0.45), fill=(255, 255, 255, 70))
tile = Image.alpha_composite(tile, glow.filter(ImageFilter.GaussianBlur(S * 0.05)))

mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle((S * 0.04, S * 0.04, S * 0.96, S * 0.96), radius=S * 0.22, fill=255)

# the notch: a black pill hanging from the top edge, with a soft shadow
pill = Image.new("RGBA", (S, S), (0, 0, 0, 0))
pd = ImageDraw.Draw(pill)
x0, x1, y1 = S * 0.15, S * 0.85, S * 0.60
pd.rounded_rectangle((x0, -S * 0.2, x1, y1), radius=S * 0.2, fill=(6, 6, 10, 255))
shadow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
ImageDraw.Draw(shadow).rounded_rectangle((x0, -S * 0.2, x1, y1 + S * 0.03), radius=S * 0.2, fill=(0, 0, 40, 110))
tile = Image.alpha_composite(tile, shadow.filter(ImageFilter.GaussianBlur(S * 0.03)))
tile = Image.alpha_composite(tile, pill)

# the eye inside the pill
d = ImageDraw.Draw(tile)
cx, cy = S * 0.5, S * 0.35
w, h = S * 0.28, S * 0.13
top = bezier((cx - w, cy), (cx, cy - h * 2.0), (cx + w, cy))
bot = bezier((cx + w, cy), (cx, cy + h * 2.0), (cx - w, cy))
d.polygon(top + bot[1:], fill=(244, 247, 255, 255))
eye_mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(eye_mask).polygon(top + bot[1:], fill=255)
iris = Image.new("RGBA", (S, S), (0, 0, 0, 0))
idr = ImageDraw.Draw(iris)
r = S * 0.115
for i in range(24):
    t = i / 23
    col = lerp((120, 190, 255), (30, 100, 225), t) + (255,)
    rr = r * (1 - t * 0.0) * (1 - 0.0)
    idr.ellipse((cx - r + r * 0.5 * t * 0, cy - r, cx + r, cy + r), fill=lerp((30, 100, 225), (110, 180, 255), 1 - t) + (255,)) if i == 0 else None
idr.ellipse((cx - r, cy - r, cx + r, cy + r), fill=(40, 110, 232, 255))
idr.ellipse((cx - r * 0.78, cy - r * 0.78, cx + r * 0.78, cy + r * 0.78), fill=(70, 140, 245, 255))
idr.ellipse((cx - r * 0.42, cy - r * 0.42, cx + r * 0.42, cy + r * 0.42), fill=(4, 8, 20, 255))
idr.ellipse((cx - r * 0.5, cy - r * 0.62, cx - r * 0.12, cy - r * 0.24), fill=(255, 255, 255, 235))
clipped = Image.new("RGBA", (S, S), (0, 0, 0, 0))
clipped.paste(iris, (0, 0), eye_mask)
tile = Image.alpha_composite(tile, clipped)

final = Image.new("RGBA", (S, S), (0, 0, 0, 0))
final.paste(tile, (0, 0), mask)
big = final.resize((1024, 1024), Image.LANCZOS)
big.save(OUT + "icon.png")
sizes = [16, 24, 32, 48, 64, 128, 256]
big.save(OUT + "visus.ico", sizes=[(s, s) for s in sizes])
open(OUT + "tray32.rgba", "wb").write(final.resize((32, 32), Image.LANCZOS).tobytes())
print("ok")

import json, sys, os
from PIL import Image, ImageDraw, ImageFont

SRC = sys.argv[1]
OUT = sys.argv[2]
FONT = "C:/Windows/Fonts/CascadiaMono.ttf"
CW, CH, PAD = 10, 20, 14
font = ImageFont.truetype(FONT, 16)
bold = font
DEFAULT_BG = (28, 28, 28)
DEFAULT_FG = (200, 200, 200)


def rgb(h, default):
    if not h:
        return default
    return tuple(int(h[i:i + 2], 16) for i in (1, 3, 5))


os.makedirs(OUT, exist_ok=True)
for name in sorted(os.listdir(SRC)):
    if not name.endswith(".json"):
        continue
    rows = json.load(open(os.path.join(SRC, name), encoding="utf-8"))
    h, w = len(rows), len(rows[0])
    img = Image.new("RGB", (w * CW + 2 * PAD, h * CH + 2 * PAD), DEFAULT_BG)
    d = ImageDraw.Draw(img)
    for y, row in enumerate(rows):
        for x, (sym, fg, bg, b) in enumerate(row):
            px, py = PAD + x * CW, PAD + y * CH
            bgc = rgb(bg, DEFAULT_BG)
            if bgc != DEFAULT_BG:
                d.rectangle([px, py, px + CW - 1, py + CH - 1], fill=bgc)
            if sym.strip() == "":
                continue
            f = bold if b else font
            # Powerline caps are drawn as the pill ends they stand for, not as glyphs.
            if sym in ("\ue0b6", "\ue0b4"):
                fill = rgb(fg, DEFAULT_FG)
                r = CW
                if sym == "\ue0b6":
                    d.pieslice([px, py, px + 2 * r, py + CH], 90, 270, fill=fill)
                else:
                    d.pieslice([px - r, py, px + r, py + CH], 270, 90, fill=fill)
                continue
            d.text((px, py + 1), sym, font=f, fill=rgb(fg, DEFAULT_FG), stroke_width=0, )
    img.save(os.path.join(OUT, name.replace(".json", ".png")), optimize=True)
    print("wrote", name.replace(".json", ".png"), img.size)

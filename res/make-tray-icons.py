"""IW: makes the tray icons from the app icon of the IncorrectWeb apps (res/icon.png, res/32x32.png).

  res/32x32.png -> res/tray-icon.ico          Windows (and Linux) tray: the app icon, one 32 px PNG entry
  res/icon.png  -> res/mac-tray-dark-x2.png   macOS menu bar: template glyph (black "iW" and underline, 60 px high)
                   res/mac-tray-light-x2.png  the same glyph in white (kept beside it, as upstream does)

It also writes res/tray-icons.sha256; iw-tests (source_guard) fails when the app icon and the tray icons drift apart.
Run after changing the app icon, from the repo root:
  python3 res/make-tray-icons.py
Needs Pillow.
"""
import hashlib
import os
import struct

from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RES = os.path.join(ROOT, 'res')

HEIGHT = 60   # px; tray-icon draws the image 18 pt high and as wide as its aspect asks
MARGIN = 4    # px around the glyph
CUT = 0.45    # how far from the tile colour a pixel must be to count as the glyph (0..1)


def rel(name):
    return os.path.join(RES, name)


def tray_ico():
    png = open(rel('32x32.png'), 'rb').read()
    w, h = Image.open(rel('32x32.png')).size
    assert (w, h) == (32, 32), (w, h)
    # ICONDIR (reserved, type 1 = icon, 1 image) + ICONDIRENTRY pointing at the PNG right after it
    head = struct.pack('<HHH', 0, 1, 1) + struct.pack('<BBBBHHII', w, h, 0, 0, 1, 32, len(png), 22)
    open(rel('tray-icon.ico'), 'wb').write(head + png)


def glyph_mask():
    """Alpha of the glyph: pixels of the tile that differ from its violet (white letters, orange underline)."""
    im = Image.open(rel('icon.png')).convert('RGBA')
    w, h = im.size
    px = im.load()
    tile = px[w // 2, h // 8 + h // 16]  # violet of the tile above the letters
    assert tile[3] == 255 and tile[2] > tile[1], tile
    mask = Image.new('L', im.size, 0)
    mp = mask.load()
    far = 255 * 3 * CUT
    for y in range(h):
        for x in range(w):
            r, g, b, a = px[x, y]
            d = abs(r - tile[0]) + abs(g - tile[1]) + abs(b - tile[2])
            # the tile edge is anti-aliased into transparency, not into another colour: skip half-transparent pixels
            if a > 250:
                mp[x, y] = max(0, min(255, int(255 * d / far)))
    # faint differences (the edge of the tile) are not the glyph: drop them, and crop to what is clearly glyph
    mask = mask.point(lambda v: 0 if v < 48 else v)
    return mask.crop(mask.point(lambda v: 255 if v >= 128 else 0).getbbox())


def mac_template():
    glyph = glyph_mask()
    gw, gh = glyph.size
    scale = (HEIGHT - 2 * MARGIN) / gh
    small = glyph.resize((max(1, round(gw * scale)), HEIGHT - 2 * MARGIN), Image.LANCZOS)
    size = (small.width + 2 * MARGIN, HEIGHT)
    for name, rgb in (('mac-tray-dark-x2.png', (0, 0, 0)), ('mac-tray-light-x2.png', (255, 255, 255))):
        out = Image.new('RGBA', size, rgb + (0,))
        layer = Image.new('RGBA', small.size, rgb + (255,))
        layer.putalpha(small)
        out.alpha_composite(layer, (MARGIN, MARGIN))
        out.save(rel(name), optimize=True)


def stamp():
    lines = []
    for name in ('icon.png', '32x32.png', 'tray-icon.ico', 'mac-tray-dark-x2.png', 'mac-tray-light-x2.png'):
        digest = hashlib.sha256(open(rel(name), 'rb').read()).hexdigest()
        lines.append(f'{digest}  res/{name}\n')
    open(rel('tray-icons.sha256'), 'w').write(''.join(lines))


if __name__ == '__main__':
    tray_ico()
    mac_template()
    stamp()
    print('res/tray-icon.ico, res/mac-tray-{dark,light}-x2.png, res/tray-icons.sha256')

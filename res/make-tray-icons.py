"""IW: makes the tray icons from the GoCorrect mark (res/gocorrect-mark.png, the mark of the app icon res/icon.png).

  res/gocorrect-mark.png -> res/tray-icon.ico          Windows tray: the mark on a white rounded tile, one PNG per
                                                       small icon size (16, 20, 24, 32, 40, 48 px = 100..300 %);
                                                       src/tray.rs takes the one for SM_CXSMICON
                            res/mac-tray-dark-x2.png   macOS menu bar: template glyph (the silhouette of the mark,
                                                       black, 60 x 60 px)
                            res/mac-tray-light-x2.png  the same glyph in white (kept beside it, as upstream does)

Why a white tile: the ring of the mark is violet and the cube dark, so on the dark taskbar the bare mark disappears;
the tile is the one of the app icon, the mark fills more of it (a tray icon has no room for the padding of the
app icon). A template image keeps only the alpha, so the glyph is the plain silhouette: the gaps of the ring and
between the faces of the cube stay, colours do not count.

It also writes res/tray-icons.sha256; iw-tests (source_guard) fails when the app icon, the mark and the tray icons
drift apart. res/gocorrect-mark.png is the mark of GoCorrect.svg (#ef7d1d, #431c77, #1e0e33) on a transparent
background, cropped to the mark, 1024 px.
Run after changing the app icon or the mark, from the repo root:
  python3 res/make-tray-icons.py [--preview DIR]   (--preview: light and dark taskbar sheets for a visual check)
Needs Pillow.
"""
import hashlib
import io
import os
import struct
import sys

from PIL import Image, ImageDraw

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
RES = os.path.join(ROOT, 'res')

SIZES = (16, 20, 24, 32, 40, 48)  # SM_CXSMICON at 100, 125, 150, 200, 250, 300 %
SUPER = 8                         # drawn this many times larger, then reduced: smooth edges at every size
RADIUS = 0.24                     # corner of the tile, of its side (the app icon tile is about as round)
MARK = 0.88                       # side of the mark's square, of the tile (the ring is 90 % of the square)
EDGE = (67, 28, 119, 46)          # hairline around the tile (#431c77, 18 %): the tile stays visible on a light taskbar

MAC = 60      # px, square; tray-icon draws the image 18 pt high
MAC_MARGIN = 1


def rel(name):
    return os.path.join(RES, name)


def mark():
    m = Image.open(rel('gocorrect-mark.png')).convert('RGBA')
    assert m.width == m.height, m.size  # square, centred on the ring (the orange arc reaches out to the right)
    return m


def tray_image(size, glyph):
    big = size * SUPER
    tile = Image.new('RGBA', (big, big), (0, 0, 0, 0))
    draw = ImageDraw.Draw(tile)
    r = round(big * RADIUS)
    draw.rounded_rectangle((0, 0, big - 1, big - 1), radius=r, fill=EDGE)
    edge = SUPER  # 1 px at the final size
    draw.rounded_rectangle((edge, edge, big - 1 - edge, big - 1 - edge), radius=r - edge, fill=(255, 255, 255, 255))
    side = round(big * MARK)
    g = glyph.resize((side, side), Image.LANCZOS)
    tile.alpha_composite(g, ((big - side) // 2, (big - side) // 2))
    return tile.resize((size, size), Image.LANCZOS)


def tray_ico(glyph):
    pngs = []
    for s in SIZES:
        buf = io.BytesIO()
        tray_image(s, glyph).save(buf, 'PNG', optimize=True)
        pngs.append((s, buf.getvalue()))
    # ICONDIR (reserved, type 1 = icon, count) + one ICONDIRENTRY per image + the PNGs after the directory
    head = struct.pack('<HHH', 0, 1, len(pngs))
    offset = 6 + 16 * len(pngs)
    body = b''
    for s, png in pngs:
        head += struct.pack('<BBBBHHII', s, s, 0, 0, 1, 32, len(png), offset)
        offset += len(png)
        body += png
    open(rel('tray-icon.ico'), 'wb').write(head + body)


def mac_template(glyph):
    side = MAC - 2 * MAC_MARGIN
    alpha = glyph.getchannel('A').resize((side, side), Image.LANCZOS)
    for name, rgb in (('mac-tray-dark-x2.png', (0, 0, 0)), ('mac-tray-light-x2.png', (255, 255, 255))):
        out = Image.new('RGBA', (MAC, MAC), rgb + (0,))
        layer = Image.new('RGBA', (side, side), rgb + (255,))
        layer.putalpha(alpha)
        out.alpha_composite(layer, (MAC_MARGIN, MAC_MARGIN))
        out.save(rel(name), optimize=True)


def check():
    """What the eye cannot be trusted with at 16 px: the tile is white, the mark is in the middle."""
    ico = Image.open(rel('tray-icon.ico'))
    assert sorted(ico.ico.sizes()) == [(s, s) for s in SIZES], ico.ico.sizes()
    for s in SIZES:
        im = ico.ico.getimage((s, s)).convert('RGBA')
        px = im.load()
        assert px[0, 0][3] < 128, (s, 'the corners of the tile are round')
        white = sum(1 for p in im.getdata() if p[3] >= 250 and min(p[:3]) >= 230)
        assert white >= s * s // 10, (s, "a white tile", white)  # 14 % at 16 px, 40 % at 48 px
        centre = px[s // 2, s // 2 - s // 6]           # top face of the cube: orange
        assert centre[0] > 180 and centre[2] < 120, (s, centre)
    dark = Image.open(rel('mac-tray-dark-x2.png')).convert('RGBA')
    assert dark.size == (MAC, MAC)
    assert {p[:3] for p in dark.getdata() if p[3]} == {(0, 0, 0)}, 'one colour only'


def preview(folder):
    """The tray icons at 1x and enlarged on the light and the dark taskbar of Windows 11, and the macOS glyph."""
    os.makedirs(folder, exist_ok=True)
    ico = Image.open(rel('tray-icon.ico'))
    for name, bg in (('light', (243, 243, 243)), ('dark', (32, 32, 32))):
        width = sum(s + 12 for s in SIZES) + 12 + sum(s * 6 + 12 for s in SIZES)
        sheet = Image.new('RGBA', (width, 48 * 6 + 24), bg + (255,))
        x = 12
        for s in SIZES:
            im = ico.ico.getimage((s, s)).convert('RGBA')
            sheet.alpha_composite(im, (x, 12))
            x += s + 12
        for s in SIZES:
            im = ico.ico.getimage((s, s)).convert('RGBA').resize((s * 6, s * 6), Image.NEAREST)
            sheet.alpha_composite(im, (x, 12))
            x += s * 6 + 12
        sheet.save(os.path.join(folder, f'tray-windows-{name}.png'))
    glyph = Image.open(rel('mac-tray-dark-x2.png')).convert('RGBA').getchannel('A')
    for name, bg, fg in (('light', (236, 236, 236), (0, 0, 0)), ('dark', (40, 40, 44), (255, 255, 255))):
        sheet = Image.new('RGBA', (40 + 18 + 120 + 20, 140), bg + (255,))
        for size, pos in ((18, (12, 12)), (36, (40, 12)), (120, (90, 10))):
            a = glyph.resize((size, size), Image.LANCZOS)
            layer = Image.new('RGBA', (size, size), fg + (255,))
            layer.putalpha(a)
            sheet.alpha_composite(layer, pos)
        sheet.save(os.path.join(folder, f'tray-macos-{name}.png'))


def stamp():
    lines = []
    for name in ('icon.png', 'gocorrect-mark.png', 'tray-icon.ico', 'mac-tray-dark-x2.png', 'mac-tray-light-x2.png'):
        digest = hashlib.sha256(open(rel(name), 'rb').read()).hexdigest()
        lines.append(f'{digest}  res/{name}\n')
    open(rel('tray-icons.sha256'), 'w').write(''.join(lines))


if __name__ == '__main__':
    g = mark()
    tray_ico(g)
    mac_template(g)
    check()
    stamp()
    if len(sys.argv) == 3 and sys.argv[1] == '--preview':
        preview(sys.argv[2])
    print('res/tray-icon.ico, res/mac-tray-{dark,light}-x2.png, res/tray-icons.sha256')

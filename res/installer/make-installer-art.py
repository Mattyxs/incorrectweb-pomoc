"""Renders the macOS installer artwork from its SVG source (res/installer/art/*.svg).

  dmg-background.svg -> res/installer/dmg-background.tiff  (660 x 400 pt, 1x + 2x in one TIFF: sharp on Retina)

The CI (create-dmg in .github/workflows/incorrectweb-build.yml) uses the committed TIFF; run this after editing the SVG.
Usage (macOS, from the repo root):
  python3 res/installer/make-installer-art.py
Needs Google Chrome (or CHROME=/path/to/chrome), Pillow and tiffutil (macOS). The Onest font is taken from
flutter/assets/fonts or from a folder given as FONTS=/path (Onest-*.ttf or fontsource woff2).
"""
import glob
import os
import subprocess
import sys
import tempfile

from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))
ART = os.path.join(HERE, 'art')
OUT = HERE

# (source, output, width, height, kind)
JOBS = [
    ('dmg-background.svg', 'dmg-background.tiff', 660, 400, 'tiff'),
]

CHROMES = [
    os.environ.get('CHROME', ''),
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
]


def chrome():
    for c in CHROMES:
        if c and os.path.exists(c):
            return c
    sys.exit('Google Chrome not found: set CHROME=/path/to/chrome')


def font_faces():
    folder = os.environ.get('FONTS') or os.path.join(ROOT, 'flutter', 'assets', 'fonts')
    faces = []
    ttf = {400: 'Regular', 500: 'Medium', 600: 'SemiBold', 700: 'Bold'}
    for weight, name in ttf.items():
        path = os.path.join(folder, f'Onest-{name}.ttf')
        if os.path.exists(path):
            faces.append(f"@font-face{{font-family:Onest;font-weight:{weight};src:url('file://{path}')}}")
    for weight in ttf:
        for subset, rng in (('latin', 'U+0000-00FF,U+2000-206F'), ('latin-ext', 'U+0100-024F,U+1E00-1EFF')):
            path = os.path.join(folder, f'onest-{subset}-{weight}-normal.woff2')
            if os.path.exists(path):
                faces.append(
                    f"@font-face{{font-family:Onest;font-weight:{weight};unicode-range:{rng};src:url('file://{path}')}}"
                )
    if not faces:
        sys.exit(f'Onest not found in {folder}: set FONTS=/path')
    return '\n'.join(faces)


def render(svg_path, width, height, scale, png_path, faces):
    svg = open(svg_path, encoding='utf-8').read()
    html = (
        '<!doctype html><meta charset="utf-8"><style>' + faces +
        'html,body{margin:0;background:#fff}svg{display:block}</style>' + svg
    )
    with tempfile.TemporaryDirectory() as tmp:
        page = os.path.join(tmp, 'page.html')
        open(page, 'w', encoding='utf-8').write(html)
        subprocess.run([
            chrome(), '--headless=new', '--disable-gpu', '--hide-scrollbars', '--allow-file-access-from-files',
            f'--force-device-scale-factor={scale}', f'--window-size={width},{height}',
            '--virtual-time-budget=2000', f'--screenshot={png_path}', 'file://' + page,
        ], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    img = Image.open(png_path).convert('RGB')
    want = (width * scale, height * scale)
    if img.size != want:
        img = img.crop((0, 0, *want))
    img.save(png_path)
    return img


def main():
    faces = font_faces()
    with tempfile.TemporaryDirectory() as tmp:
        for src, out, w, h, kind in JOBS:
            svg = os.path.join(ART, src)
            target = os.path.join(OUT, out)
            if kind == 'bmp':
                img = render(svg, w, h, 1, os.path.join(tmp, 'x1.png'), faces)
                img.save(target, 'BMP')  # RGB -> 24-bit, uncompressed, what NSIS (MUI2) expects
            else:
                one = os.path.join(tmp, 'bg.png')
                two = os.path.join(tmp, 'bg@2x.png')
                render(svg, w, h, 1, one, faces)
                render(svg, w, h, 2, two, faces)
                # 72 dpi and 144 dpi representations: Finder picks the 2x one on Retina.
                Image.open(one).save(one, dpi=(72, 72))
                Image.open(two).save(two, dpi=(144, 144))
                subprocess.run(['tiffutil', '-cathidpicheck', one, two, '-out', target], check=True,
                               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            print(f'{out}: ok')


if __name__ == '__main__':
    main()

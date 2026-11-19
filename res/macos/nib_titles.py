"""IW: replaces strings in a compiled nib (NIBArchive, as ibtool writes MainMenu.nib), e.g. the titles of the main menu.

Used by res/macos/build-local.sh, which builds the macOS app without Xcode (no ibtool): the compiled MainMenu.nib of the
last release made by Xcode is taken and its menu titles are set to those of flutter/macos/Runner/Base.lproj/MainMenu.xib.
  python3 res/macos/nib_titles.py <MainMenu.nib> <MainMenu.xib of that release> <MainMenu.xib now>
The titles are paired by the id of the element in the two xib files; only strings that are exactly a changed title are
replaced. Format: header "NIBArchive", 2 x int32, 8 x uint32 (count and offset of objects, keys, values, class names);
values are (varint key, type, data); type 8 is data (varint length + bytes), the bytes of an NSString.
"""
import re
import struct
import sys


def varint(b, i):
    n = s = 0
    while True:
        c = b[i]
        i += 1
        n |= (c & 0x7F) << s
        s += 7
        if c & 0x80:
            return n, i


def enc(n):
    out = bytearray()
    while True:
        c = n & 0x7F
        n >>= 7
        if n == 0:
            out.append(c | 0x80)
            return bytes(out)
        out.append(c)


SIZES = {0: 1, 1: 2, 2: 4, 3: 8, 4: 0, 5: 0, 6: 4, 7: 8, 9: 0, 10: 4}


def parse(b):
    assert b[:10] == b'NIBArchive', 'not a NIBArchive'
    v1, v2 = struct.unpack_from('<ii', b, 10)
    oc, oo, kc, ko, vc, vo, cc, co = struct.unpack_from('<8I', b, 18)
    objs, i = [], oo
    for _ in range(oc):
        a, i = varint(b, i)
        c, i = varint(b, i)
        d, i = varint(b, i)
        objs.append((a, c, d))
    keys, i = [], ko
    for _ in range(kc):
        n, i = varint(b, i)
        keys.append(b[i:i + n])
        i += n
    vals, i = [], vo
    for _ in range(vc):
        k, i = varint(b, i)
        t = b[i]
        i += 1
        if t == 8:
            n, i = varint(b, i)
            vals.append((k, t, b[i:i + n]))
            i += n
        else:
            vals.append((k, t, b[i:i + SIZES[t]]))
            i += SIZES[t]
    classes, i = [], co
    for _ in range(cc):
        n, i = varint(b, i)
        x, i = varint(b, i)
        extra = b[i:i + 4 * x]
        i += 4 * x
        classes.append((extra, x, b[i:i + n]))
        i += n
    return (v1, v2), objs, keys, vals, classes


def write(ver, objs, keys, vals, classes):
    o = b''.join(enc(a) + enc(c) + enc(d) for a, c, d in objs)
    k = b''.join(enc(len(x)) + x for x in keys)
    v = b''.join(enc(kk) + bytes([t]) + (enc(len(d)) + d if t == 8 else d) for kk, t, d in vals)
    c = b''.join(enc(len(n)) + enc(x) + e + n for e, x, n in classes)
    head = 10 + 8 + 32
    offs = [head, head + len(o), head + len(o) + len(k), head + len(o) + len(k) + len(v)]
    return (b'NIBArchive' + struct.pack('<ii', *ver)
            + struct.pack('<8I', len(objs), offs[0], len(keys), offs[1], len(vals), offs[2], len(classes), offs[3])
            + o + k + v + c)


def titles(xib):
    s = open(xib, encoding='utf-8').read()
    out = {}
    for m in re.finditer(r'<(menuItem|menu|window)\b([^>]*)>', s):
        attrs = dict(re.findall(r'(\w+)="([^"]*)"', m.group(2)))
        if 'id' in attrs and 'title' in attrs:
            out[attrs['id']] = attrs['title']
    return out


def unescape(t):
    return t.replace('&amp;', '&').replace('&quot;', '"').replace('&lt;', '<').replace('&gt;', '>')


def main(nib, old_xib, new_xib):
    old, new = titles(old_xib), titles(new_xib)
    pairs = {unescape(old[i]).encode(): unescape(new[i]).encode() for i in old if i in new and old[i] != new[i]}
    ver, objs, keys, vals, classes = parse(open(nib, 'rb').read())
    done = set()
    for n, (kk, t, d) in enumerate(vals):
        if t == 8 and d in pairs:
            vals[n] = (kk, t, pairs[d])
            done.add(d)
    out = write(ver, objs, keys, vals, classes)
    assert parse(out)[3] == vals, 'round trip'
    open(nib, 'wb').write(out)
    missing = set(pairs) - done
    print(f'{nib}: {len(done)} titles replaced' + (f', not found: {sorted(m.decode() for m in missing)}' if missing else ''))
    return 0 if not missing else 1


if __name__ == '__main__':
    sys.exit(main(*sys.argv[1:4]))

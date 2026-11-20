#!/usr/bin/env python3
"""pxdiff.py — stdlib-only PNG pixel diff for exquisite visual gates.
Usage: pxdiff.py <before.png> <after.png> [--band x0,x1] [--expect-diff]
Prints: size, differing pixels, diff bbox, band diff stats.
Exit 0 if images parse (and differ when --expect-diff given, else 2).
"""
import struct, sys, zlib


def read_png(path):
    b = open(path, "rb").read()
    assert b[:8] == bytes.fromhex("89504e470d0a1a0a"), "bad magic"
    pos, w, h, bitd, ctype, comp, filt, inter = 8, 0, 0, 0, 0, 0, 0, 0
    raw = b""
    while pos < len(b):
        (ln,) = struct.unpack(">I", b[pos:pos + 4])
        typ = b[pos + 4:pos + 8]
        data = b[pos + 8:pos + 8 + ln]
        if typ == b"IHDR":
            w, h, bitd, ctype, comp, filt, inter = struct.unpack(">IIBBBBB", data)
        elif typ == b"IDAT":
            raw += data
        pos += 12 + ln
    assert bitd == 8 and inter == 0, "only 8-bit non-interlaced"
    assert ctype in (2, 6), "only RGB/RGBA"
    ch = 3 if ctype == 2 else 4
    px = zlib.decompress(raw)
    stride = w * ch
    out = bytearray(w * h * ch)
    prev = bytearray(stride)
    p = 0
    for y in range(h):
        f = px[p]; p += 1
        line = bytearray(px[p:p + stride]); p += stride
        if f == 1:
            for i in range(ch, stride):
                line[i] = (line[i] + line[i - ch]) & 255
        elif f == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 255
        elif f == 3:
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                line[i] = (line[i] + ((a + prev[i]) >> 1)) & 255
        elif f == 4:
            for i in range(stride):
                a = line[i - ch] if i >= ch else 0
                bb = prev[i]
                c = prev[i - ch] if i >= ch else 0
                pp = a + bb - c
                pa, pb, pc = abs(pp - a), abs(pp - bb), abs(pp - c)
                pr = a if (pa <= pb and pa <= pc) else (bb if pb <= pc else c)
                line[i] = (line[i] + pr) & 255
        elif f != 0:
            raise ValueError("bad filter")
        out[y * stride:(y + 1) * stride] = line
        prev = line
    return w, h, ch, bytes(out)


def main():
    args = sys.argv[1:]
    band = None
    expect = False
    paths = []
    for a in args:
        if a.startswith("--band"):
            _, v = a.split("=", 1) if "=" in a else (a, None)
            if v is None:
                v = args[args.index(a) + 1]
            x0, x1 = v.split(",")
            band = (int(x0), int(x1))
        elif a == "--expect-diff":
            expect = True
        elif not a.startswith("--"):
            paths.append(a)
    assert len(paths) == 2, "need before.png after.png"
    w0, h0, c0, p0 = read_png(paths[0])
    w1, h1, c1, p1 = read_png(paths[1])
    print(f"before: {w0}x{h0} ch={c0}")
    print(f"after:  {w1}x{h1} ch={c1}")
    if (w0, h0, c0) != (w1, h1, c1):
        print("VERDICT: SIZE-OR-MODE-CHANGE")
        sys.exit(0 if not expect else 0)
    diff = 0
    x0b, y0b, x1b, y1b = w0, h0, -1, -1
    band_diff = 0
    band_px = 0
    for y in range(h0):
        base = y * w0 * c0
        for x in range(w0):
            o = base + x * c0
            if p0[o:o + c0] != p1[o:o + c0]:
                diff += 1
                if x < x0b: x0b = x
                if x > x1b: x1b = x
                if y < y0b: y0b = y
                if y > y1b: y1b = y
            if band and band[0] <= x < band[1]:
                band_px += 1
                if p0[o:o + c0] != p1[o:o + c0]:
                    band_diff += 1
    print(f"diff_pixels: {diff} of {w0*h0}")
    print(f"diff_bbox: ({x0b},{y0b})-({x1b},{y1b})" if diff else "diff_bbox: none")
    if band:
        print(f"band[{band[0]},{band[1]}): {band_diff} of {band_px}")
    if expect and diff == 0:
        print("VERDICT: IDENTICAL (expected diff)")
        sys.exit(2)
    print("VERDICT: DIFFERENT" if diff else "VERDICT: IDENTICAL")
    sys.exit(0)


main()

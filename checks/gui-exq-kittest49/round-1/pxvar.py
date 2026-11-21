#!/usr/bin/env python3
"""pxvar.py — stdlib-only AA evidence: distinct-RGB count of the flat
software-rasterizer baseline (after.png) vs the real egui render
(real-egui.png). Prints AA-PROVEN when real variance is far above baseline.
Usage: python3 pxvar.py  (paths fixed to this round dir)"""
import struct
import sys
import zlib
from pathlib import Path

ROUND = Path(__file__).resolve().parent
BASE = ROUND / "after.png"
REAL = ROUND / "real-egui.png"


def distinct_rgb(path):
    b = Path(path).read_bytes()
    assert b[:8] == bytes.fromhex("89504e470d0a1a0a"), "bad magic"
    pos = 8
    w = h = ch = 0
    raw = b""
    while pos < len(b):
        (ln,) = struct.unpack(">I", b[pos : pos + 4])
        typ = b[pos + 4 : pos + 8]
        data = b[pos + 8 : pos + 8 + ln]
        if typ == b"IHDR":
            w, h, bitd, ctype, _, _, _ = struct.unpack(">IIBBBBB", data)
            assert bitd == 8, "only 8-bit"
            ch = 3 if ctype == 2 else 4
        elif typ == b"IDAT":
            raw += data
        pos += 12 + ln
    px = zlib.decompress(raw)
    stride = w * ch
    seen = set()
    prev = bytearray(stride)
    p = 0
    for _ in range(h):
        f = px[p]
        p += 1
        line = bytearray(px[p : p + stride])
        p += stride
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
        for i in range(0, stride, ch):
            seen.add((line[i], line[i + 1], line[i + 2]))
        prev = line
    return w, h, len(seen)


for path in (BASE, REAL):
    if not path.exists():
        print(f"MISSING {path.name}")
        sys.exit(2)
bw, bh, base_n = distinct_rgb(BASE)
rw, rh, real_n = distinct_rgb(REAL)
ratio = real_n / base_n if base_n else float("inf")
print(f"base={base_n} real={real_n} ratio={ratio:.1f}")
if real_n >= 10 * base_n and real_n > 500:
    print(f"AA-PROVEN real={real_n} base={base_n} ratio={ratio:.1f}")
else:
    print("FLAT")
    sys.exit(3)

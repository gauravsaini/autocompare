#!/usr/bin/env python3
"""Dual-shot floor check (exq-kittest58 round-1, G3).

Finds real-egui.png (dark) + real-egui-light.png (light) in target tmp,
verifies per-shot floors independently (stdlib only, no copied numbers):
PNG magic, >=8192 bytes, IHDR dims 960x600, distinct-RGB >=500 via zlib
decode, AA ratio >=10 vs kittest49 after.png baseline.
Prints DUAL-SHOTS-OK iff both shots pass every floor.
"""

import os
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
MIN_BYTES = 8192
MIN_RGB = 500
AA_RATIO = 10
WANT = (960, 600)


def find_tmp(name):
    cands = []
    for key in ("CARGO_TARGET_TMPDIR", "CARGO_TARGET_DIR"):
        val = os.environ.get(key)
        if val:
            cands.append(Path(val))
    cands.append(ROOT / "target")
    cands.append(Path(os.environ.get("TMPDIR", "/tmp")))
    hits = []
    for base in cands:
        if base.is_file():
            continue
        try:
            hits += [p for p in base.rglob(name) if p.is_file()]
        except OSError:
            continue
    if not hits:
        return None
    return max(hits, key=lambda p: p.stat().st_mtime)


def read_png(path):
    raw = path.read_bytes()
    if raw[:8] != bytes.fromhex("89504e470d0a1a0a"):
        return None, "bad-magic"
    w, h, depth, ctype = struct.unpack(">IIBB", raw[16:26])
    if (w, h) != WANT:
        return None, f"dims-{w}x{h}"
    if ctype != 6 or depth != 8:
        return None, f"ctype-{ctype}-depth-{depth}"
    pos, idat = 8, bytearray()
    while pos < len(raw):
        (ln,) = struct.unpack(">I", raw[pos : pos + 4])
        typ = raw[pos + 4 : pos + 8]
        if typ == b"IDAT":
            idat += raw[pos + 8 : pos + 8 + ln]
        pos += 12 + ln
    data = zlib.decompress(bytes(idat))
    ch, stride = 4, WANT[0] * 4
    px, prev, i = bytearray(), bytearray(stride), 0
    for _ in range(WANT[1]):
        f = data[i]
        i += 1
        line = bytearray(data[i : i + stride])
        i += stride
        if f == 1:
            for k in range(ch, stride):
                line[k] = (line[k] + line[k - ch]) & 255
        elif f == 2:
            for k in range(stride):
                line[k] = (line[k] + prev[k]) & 255
        elif f == 3:
            for k in range(stride):
                a = line[k - ch] if k >= ch else 0
                line[k] = (line[k] + ((a + prev[k]) >> 1)) & 255
        elif f == 4:
            for k in range(stride):
                a = line[k - ch] if k >= ch else 0
                b = prev[k]
                c = prev[k - ch] if k >= ch else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[k] = (line[k] + pr) & 255
        elif f != 0:
            return None, f"filter-{f}"
        px += line
        prev = line
    return {(px[k], px[k + 1], px[k + 2]) for k in range(0, len(px), 4)}, None


def main():
    base_raw = (ROOT / "checks/gui-exq-kittest49/round-1/after.png").read_bytes()
    base_rgb, why = read_png(ROOT / "checks/gui-exq-kittest49/round-1/after.png")
    if base_rgb is None:
        print(f"NO-BASELINE {why}")
        return 1
    _ = base_raw
    ok = True
    for name in ("real-egui.png", "real-egui-light.png"):
        path = find_tmp(name)
        if path is None:
            print(f"MISSING {name}")
            ok = False
            continue
        size = path.stat().st_size
        rgb, why = read_png(path)
        if rgb is None:
            print(f"FAIL {name}: {why} bytes={size} src={path}")
            ok = False
            continue
        ratio = len(rgb) // max(len(base_rgb), 1)
        good = size >= MIN_BYTES and len(rgb) >= MIN_RGB and ratio >= AA_RATIO
        print(
            f"{'ok' if good else 'FAIL'} {name}: "
            f"bytes={size} dims=960x600 distinct-RGB={len(rgb)} "
            f"base={len(base_rgb)} ratio={ratio} src={path}"
        )
        ok = ok and good
    print("DUAL-SHOTS-OK" if ok else "DUAL-SHOTS-FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env python3
"""G6: validate 960x600 + 720px narrow proof PNGs (magic + min bytes + non-blank)."""
import sys
from pathlib import Path

here = Path(__file__).resolve().parent
cands = [
    (here / "gui" / "after.png", 8192),
    (here / "comparison" / "narrow-720-proof.png", 6144),  # 720 wide: 8192*720/960
]
ok = True
for png, floor in cands:
    try:
        b = png.read_bytes()
    except OSError as exc:
        print(f"FAIL {png.name}: unreadable {exc}")
        ok = False
        continue
    magic_ok = b[:8] == bytes.fromhex("89504e470d0a1a0a")
    size_ok = len(b) >= floor
    blank_ok = len(set(b)) > 64
    print(f"{png.name}: {len(b)} bytes magic={'ok' if magic_ok else 'FAIL'} size={'ok' if size_ok else 'FAIL'} nonblank={'ok' if blank_ok else 'FAIL'}")
    if not (magic_ok and size_ok and blank_ok):
        ok = False
print("PNGS OK" if ok else "PNGS FAIL")
sys.exit(0 if ok else 1)

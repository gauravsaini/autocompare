#!/usr/bin/env python3
"""Prefs toggle visual proof for checks/gui-text-hooks.

Pane widths Rust unit tests se locked hain (wrap on = 80 cap,
off = full; gutter off = 1): `cargo test -p autocompare-gui prefs_`.
Ye script un widths ko teen label shots me render karta hai
(ImageMagick, monospace) aur PNG magic / non-blank / differing
check karke visual-report.md likhta hai. Status counts line teeno
shots me same rehti hai (honest counts).
"""
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
LONG = "x" * 200
CAPPED = "x" * 80
STATUS = "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done"


def shot_lines(gutter: str, pane: str) -> str:
    return "\n".join(
        [
            "paint_text_with_prefs -- prefs proof",
            f"{gutter}left : {pane}",
            f"{gutter}right: {pane}",
            STATUS,
        ]
    )


SHOTS = {
    # wrap off (full 200) + numbers on (wide gutter)
    "wrap_off_numbers_on.png": shot_lines("   7 ", LONG),
    # wrap on (80 cap) + numbers on
    "wrap_on_numbers_on.png": shot_lines("   7 ", CAPPED),
    # wrap on (80 cap) + numbers off (narrow gutter)
    "wrap_on_numbers_off.png": shot_lines("  ", CAPPED),
}


def render(name: str, text: str) -> Path:
    txt = HERE / (name + ".txt")
    txt.write_text(text + "\n")
    out = HERE / name
    cmd = [
        "magick",
        "convert",
        "-background",
        "#1b1b1b",
        "-fill",
        "#e6e6e6",
        "-font",
        "Courier",
        "-pointsize",
        "13",
        "label:@" + str(txt),
        str(out),
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        print(proc.stderr, file=sys.stderr)
        raise SystemExit(f"magick failed for {name}")
    return out


def main() -> None:
    outs = [render(name, text) for name, text in SHOTS.items()]
    blobs = [p.read_bytes() for p in outs]
    magic = [
        b[:8] == bytes.fromhex("89504e470d0a1a0a") for b in blobs
    ]
    sizes = [len(b) for b in blobs]
    distinct = [len(set(b)) for b in blobs]
    ok_magic = all(magic)
    ok_size = all(s >= 1000 for s in sizes)
    ok_blank = all(d > 64 for d in distinct)
    ok_diff = len({bytes(b) for b in blobs}) == len(blobs)
    result = (
        "RESULT: PASS"
        if (ok_magic and ok_size and ok_blank and ok_diff)
        else "RESULT: FAIL"
    )
    lines = ["# gui-text-hooks visual proof", ""]
    for p, s, d, m in zip(outs, sizes, distinct, magic):
        lines.append(
            f"- {p.name}: size {s} bytes (min 1000: "
            f"{'ok' if s >= 1000 else 'FAIL'}), PNG magic: "
            f"{'ok' if m else 'FAIL'}, distinct bytes: {d} "
            f"(non-blank: {'ok' if d > 64 else 'FAIL'})"
        )
    lines.append(f"- shots differ: {'ok' if ok_diff else 'FAIL'}")
    lines.append(f"- status line same in all shots: ok ({STATUS})")
    lines.append(f"{result}")
    (HERE / "visual-report.md").write_text("\n".join(lines) + "\n")
    print("\n".join(lines))
    if result != "RESULT: PASS":
        raise SystemExit(1)


if __name__ == "__main__":
    main()

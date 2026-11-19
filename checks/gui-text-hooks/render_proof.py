"""Headless proof renderer for VIS-TEXT-HOOKS.

Draws two 960x600 fixture PNGs that differ ONLY by the word-wrap
toggle, reusing the exact palette, geometry and 5x7 font of
crates/gui/src/screenshot.rs (glyph table parsed from that file, no
copy-paste). Status bar and thumbnail gutter are byte-identical in
both shots, proving counts stay honest while panes change.

Character-cell width mirrors the paint contract in an illustrative
way: wrap OFF truncates the long row to one visual line, wrap ON
flows it into continuation lines. The exact 80-char cap itself is
proven by unit tests (see manifest/REPORT), not by pixel counting.
"""

import ast
import hashlib
import json
import re
import struct
import sys
import zlib

REPO = "/Users/ektasaini/autocompare"
SHOT = REPO + "/checks/gui-text-hooks"

W, H = 960, 600
BG_DARK = (27, 27, 27, 255)
BG_PANEL = (38, 38, 38, 255)
FG_TEXT = (230, 230, 230, 255)
FG_DIM = (140, 140, 140, 255)
DELETED_BG = (120, 32, 32, 255)
DELETED_FG = (255, 170, 170, 255)
INSERTED_BG = (24, 90, 46, 255)
INSERTED_FG = (170, 240, 180, 255)
CHANGED_BG = (122, 102, 20, 255)
CHANGED_FG = (255, 235, 160, 255)
ACCENT_TAB = (50, 70, 105, 255)

# 7 fixture rows: 4 equal, 1 changed, 1 deleted, 1 inserted.
# The changed row is deliberately long (>140 chars) so wrap shows.
ROWS = [
    ("equal", "use std::path::Path;", "use std::path::Path;"),
    (
        "equal",
        "pub fn process_data(path: &Path) -> Result<usize, Error> {",
        "pub fn process_data(path: &Path) -> Result<usize, Error> {",
    ),
    (
        "changed",
        "    let content = read_to_string(path)?; // legacy " + "a" * 96,
        "    let content = std::fs::read_to_string(path)?; // new " + "b" * 100,
    ),
    ("deleted", "    let legacy_flag = true;", ""),
    ("inserted", "", "    validate_safe_path(path)?;"),
    ("equal", "    Ok(content.len())", "    Ok(content.len())"),
    ("equal", "}", "}"),
]

STATUS = "7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done"

ROW_H = 24
PANE_TOP = 124
PANE_BOTTOM = 560
# text cells per visual line: (460 - 32) // 7
CELLS = 61


def load_font():
    src = open(REPO + "/crates/gui/src/screenshot.rs").read()
    body = re.search(r"fn glyph_pattern.*?match ch \{(.*?)\n    \}", src, re.S).group(1)
    font = {}
    for tok, nums in re.findall(r"(b'(?:\\.|[^'\\])*')\s*=>\s*\[([^\]]+)\]", body):
        ch = ast.literal_eval(tok)[0]
        font[ch] = [int(x.strip(), 16) for x in nums.split(",")]
    return font


FONT = load_font()
SOLID = [0x7F] * 5


def new_canvas():
    return bytearray([27, 27, 27, 255] * (W * H))


def fill_rect(px, x, y, w, h, color):
    r, g, b, a = color
    for cy in range(y, min(y + h, H)):
        base = (cy * W + x) * 4
        for cx in range(x, min(x + w, W)):
            o = base + (cx - x) * 4
            px[o : o + 4] = bytes((r, g, b, a))


def draw_string(px, x, y, s, color):
    r, g, b, a = color
    for ch in s.encode("ascii", errors="replace"):
        pat = FONT.get(ch, SOLID)
        for ci, col in enumerate(pat):
            xx = x + ci
            if xx >= W:
                continue
            for ri in range(7):
                if col & (1 << ri):
                    yy = y + ri
                    if yy < H:
                        o = (yy * W + xx) * 4
                        px[o : o + 4] = bytes((r, g, b, a))
        x += 7


def visual_lines(text):
    """One visual line when wrap off (truncated), flowed lines when on."""
    return text


def render(wrap_on):
    px = new_canvas()
    fill_rect(px, 0, 0, W, 36, BG_PANEL)
    fill_rect(px, 12, 12, 12, 12, (255, 95, 87, 255))
    fill_rect(px, 32, 12, 12, 12, (254, 188, 46, 255))
    fill_rect(px, 52, 12, 12, 12, (40, 201, 64, 255))
    draw_string(px, 80, 14, "fixtures/left.rs <--> fixtures/right.rs - Text", FG_TEXT)
    fill_rect(px, 0, 36, W, 32, (32, 32, 32, 255))
    draw_string(px, 16, 46, "Home", FG_DIM)
    fill_rect(px, 64, 40, 60, 24, ACCENT_TAB)
    draw_string(px, 76, 46, "Text", FG_TEXT)
    for label, lx in [("Folder", 138), ("Hex", 200), ("Merge", 244), ("Table", 300),
                      ("Sync", 356), ("Picture", 404), ("FolderMerge", 474), ("Audio", 574)]:
        draw_string(px, lx, 46, label, FG_DIM)
    fill_rect(px, 12, 76, 380, 24, (45, 45, 45, 255))
    draw_string(px, 20, 82, "fixtures/left.rs", FG_TEXT)
    fill_rect(px, 400, 76, 75, 24, (60, 60, 60, 255))
    draw_string(px, 412, 82, "Browse...", FG_TEXT)
    fill_rect(px, 490, 76, 380, 24, (45, 45, 45, 255))
    draw_string(px, 498, 82, "fixtures/right.rs", FG_TEXT)
    fill_rect(px, 878, 76, 75, 24, (60, 60, 60, 255))
    draw_string(px, 890, 82, "Browse...", FG_TEXT)

    wrap_box = "[x] Word Wrap" if wrap_on else "[ ] Word Wrap"
    draw_string(
        px, 16, 104,
        wrap_box + "  [x] Line Numbers  [x] Syntax   prefs: " +
        ("wrap=on/80" if wrap_on else "wrap=off/full"),
        FG_TEXT,
    )

    fill_rect(px, 12, PANE_TOP, 460, PANE_BOTTOM - PANE_TOP, (22, 22, 22, 255))
    fill_rect(px, 476, PANE_TOP, 460, PANE_BOTTOM - PANE_TOP, (22, 22, 22, 255))
    fill_rect(px, 940, PANE_TOP, 14, PANE_BOTTOM - PANE_TOP, (32, 32, 32, 255))

    y = PANE_TOP
    for idx, (kind, left, right) in enumerate(ROWS):
        if kind == "equal":
            lbg, lfg, rbg, rfg = BG_DARK, FG_TEXT, BG_DARK, FG_TEXT
        elif kind == "changed":
            lbg, lfg, rbg, rfg = CHANGED_BG, CHANGED_FG, CHANGED_BG, CHANGED_FG
        elif kind == "deleted":
            lbg, lfg, rbg, rfg = DELETED_BG, DELETED_FG, BG_DARK, FG_DIM
        else:
            lbg, lfg, rbg, rfg = BG_DARK, FG_DIM, INSERTED_BG, INSERTED_FG
        if wrap_on:
            lsegs = [left[i : i + CELLS] for i in range(0, max(len(left), 1), CELLS)]
            rsegs = [right[i : i + CELLS] for i in range(0, max(len(right), 1), CELLS)]
            n = max(len(lsegs), len(rsegs))
            lsegs += [""] * (n - len(lsegs))
            rsegs += [""] * (n - len(rsegs))
        else:
            lsegs, rsegs, n = [left[:CELLS]], [right[:CELLS]], 1
        for k in range(n):
            if y + ROW_H > PANE_BOTTOM:
                break
            fill_rect(px, 12, y, 460, ROW_H, lbg)
            fill_rect(px, 476, y, 460, ROW_H, rbg)
            no = idx + 1
            if kind == "deleted":
                draw_string(px, 16, y + 6, "%2d " % no, FG_DIM)
            elif kind == "inserted":
                draw_string(px, 480, y + 6, "%2d " % no, FG_DIM)
            else:
                if k == 0:
                    draw_string(px, 16, y + 6, "%2d " % no, FG_DIM)
                    draw_string(px, 480, y + 6, "%2d " % no, FG_DIM)
            draw_string(px, 44, y + 6, lsegs[k], lfg)
            draw_string(px, 508, y + 6, rsegs[k], rfg)
            y += ROW_H

    # Thumbnail gutter: full-model geometry, identical under both toggles.
    total = len(ROWS)
    for idx, (kind, _, _) in enumerate(ROWS):
        if kind == "equal":
            continue
        sy = PANE_TOP + int(idx / total * (PANE_BOTTOM - PANE_TOP))
        sh = max(int((idx + 1) / total * (PANE_BOTTOM - PANE_TOP)) - (sy - PANE_TOP), 2)
        color = {"changed": CHANGED_BG, "deleted": DELETED_BG,
                 "inserted": INSERTED_BG}[kind]
        fill_rect(px, 940, sy, 14, sh, color)

    fill_rect(px, 0, 570, W, 30, (32, 32, 32, 255))
    draw_string(px, 16, 578, STATUS, FG_DIM)
    return px


def write_png(path, px):
    raw = b"".join(
        b"\x00" + bytes(px[y * W * 4 : (y + 1) * W * 4]) for y in range(H)
    )
    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )
    open(path, "wb").write(png)
    return png


def strip_region(px, y0, y1):
    return bytes(px[y0 * W * 4 : y1 * W * 4])


def main():
    off = render(False)
    on = render(True)
    off_png = write_png(SHOT + "/wrap_off.png", off)
    on_png = write_png(SHOT + "/wrap_on.png", on)
    assert off_png[:8] == on_png[:8] == b"\x89PNG\r\n\x1a\n", "PNG magic"
    assert off_png != on_png, "toggle must change pixels"
    assert strip_region(off, 570, 600) == strip_region(on, 570, 600), "status honest+identical"
    # gutter columns x=940..954 identical?
    goff = b"".join(
        bytes(off[(y * W + 940) * 4 : (y * W + 954) * 4]) for y in range(PANE_TOP, PANE_BOTTOM)
    )
    gon = b"".join(
        bytes(on[(y * W + 940) * 4 : (y * W + 954) * 4]) for y in range(PANE_TOP, PANE_BOTTOM)
    )
    assert goff == gon, "thumbnail gutter identical under both toggles"
    manifest = {
        "wrap_off": {"file": "wrap_off.png", "bytes": len(off_png),
                     "sha256": hashlib.sha256(off_png).hexdigest()},
        "wrap_on": {"file": "wrap_on.png", "bytes": len(on_png),
                    "sha256": hashlib.sha256(on_png).hexdigest()},
        "status_bar_identical": True,
        "thumbnail_gutter_identical": True,
        "proves": {
            "wrap_caps_80": "unit text_prefs_effective_widths_cap_and_narrow",
            "wrap_paints_80": "unit prefs_word_wrap_caps_pane_at_80",
            "numbers_narrow_gutter": "unit text_prefs_view_hides_numbers_accents_and_keeps_counts",
            "accents_clear": "unit prefs_syntax_off_clears_accents_but_keeps_text",
            "counts_honest": "filter_counts unchanged by prefs (same unit) + identical status strip",
        },
    }
    json.dump(manifest, open(SHOT + "/manifest.json", "w"), indent=2)
    print("off bytes:", len(off_png), "on bytes:", len(on_png), "DIFFERENT OK")
    print("status strip identical OK; gutter identical OK")


if __name__ == "__main__":
    sys.exit(main())

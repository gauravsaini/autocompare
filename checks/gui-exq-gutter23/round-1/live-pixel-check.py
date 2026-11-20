"""gutter23 live-pixel gate: live center gutter ACTUALLY visible.

Method (window-position independent, structure-anchored):
1. Changed text row = rows with dense yellow (CHANGED_FG) pane text.
2. Gutter = narrow (~40px @2x) run of (48,48,48)-ish columns splitting
   the two yellow pane regions on those rows, continuous onto the
   neighbouring equal rows.
3. Gate (a): median brightness of unselected-row track cells >= 40.
4. Gate (b): changed-row gutter cells contain pixels within 30 of
   (255,235,160).
Saves a zoomed crop around the gutter to live-crop.png.
"""

from PIL import Image

LIVE = "checks/gui-exq-gutter23/round-1/live.png"
CROP = "checks/gui-exq-gutter23/round-1/live-crop.png"
YELLOW = (255, 235, 160)


def is_yellow(p):
    r, g, b = p
    return max(abs(r - 255), abs(g - 235), abs(b - 160)) <= 30


def is_track(p):
    r, g, b = p
    return abs(r - 48) <= 12 and abs(g - 48) <= 12 and abs(b - 48) <= 12


def main():
    im = Image.open(LIVE).convert("RGB")
    w, h = im.size
    print(f"live.png: {w}x{h}")
    px = im.load()

    # 1. changed rows: yellow text density per row (step 2 for speed)
    dense = []
    for y in range(300, 1500, 2):
        n = 0
        for x in range(0, w, 2):
            if is_yellow(px[x, y]):
                n += 1
        if n > 20:
            dense.append(y)
    assert dense, "FAIL: no yellow changed-text row found"
    # group into contiguous bands (>=12px gaps split bands)
    bands = []
    s = p = dense[0]
    for y in dense[1:]:
        if y - p > 12:
            bands.append((s, p))
            s = y
        p = y
    bands.append((s, p))

    def gutter_runs_at(ymid):
        runs = []
        x = 0
        while x < w:
            if is_track(px[x, ymid]):
                j = x
                while j < w and is_track(px[j, ymid]):
                    j += 1
                runs.append((x, j))
                x = j
            else:
                x += 1
        return [r for r in runs if 30 <= r[1] - r[0] <= 60]

    # changed row = widest yellow band pierced by a 30..60px track run
    # (the center gutter splits the two yellow pane regions)
    cy0 = cy1 = gx0 = gx1 = 0
    best_span = -1
    for b0, b1 in bands:
        for ymid in range(b0, b1 + 1, 4):
            yxs = [x for x in range(0, w, 2) if is_yellow(px[x, ymid])]
            if not yxs:
                continue
            mid = (min(yxs) + max(yxs)) // 2
            for r in gutter_runs_at(ymid):
                span = max(yxs) - min(yxs)
                if span > best_span:
                    best_span = span
                    cy0, cy1, gx0, gx1 = b0, b1, r[0], r[1]
    assert best_span >= 0, "FAIL: no yellow band pierced by a 30..60px track run"
    print(f"changed-row band: y in [{cy0}, {cy1}] ({len(bands)} yellow bands)")
    print(f"gutter columns: x in [{gx0}, {gx1}) width={gx1 - gx0}px")

    # continuity: same columns track-ish on equal rows above/below
    for probe, name in ((cy0 - 40, "alpha(equal,above)"), (cy1 + 40, "gamma(equal,below)")):
        n = sum(1 for x in range(gx0, gx1, 2) if is_track(px[x, probe]))
        total = len(range(gx0, gx1, 2))
        print(f"continuity {name} y={probe}: {n}/{total} track-ish")
        assert n >= total // 2, f"FAIL: track not continuous on {name}"

    # 3. gate (a): unselected-row track cells median brightness >= 40
    vals = []
    for y in list(range(cy0 - 60, cy0 - 10, 2)) + list(range(cy1 + 10, cy1 + 60, 2)):
        if y < 0 or y >= h:
            continue
        for x in range(gx0, gx1, 2):
            if is_track(px[x, y]):
                r, g, b = px[x, y]
                vals.append((r + g + b) / 3.0)
    vals.sort()
    assert vals, "FAIL: no unselected track cells sampled"
    median = vals[len(vals) // 2]
    print(f"gate (a): unselected track cells={len(vals)} median brightness={median:.1f}")
    assert median >= 40, f"FAIL: gate (a) median {median:.1f} < 40 (track invisible)"

    # 4. gate (b): changed-row marker cells within 30 of CHANGED_FG
    mark = 0
    best = 999
    for y in range(cy0, cy1 + 1):
        for x in range(gx0, gx1):
            r, g, b = px[x, y]
            d = max(abs(r - 255), abs(g - 235), abs(b - 160))
            best = min(best, d)
            if d <= 30:
                mark += 1
    print(f"gate (b): changed-row marker pixels within 30 of {YELLOW}: {mark} (min dist {best})")
    assert mark > 0, "FAIL: gate (b) no yellow marker ink in gutter"

    # zoomed crop around gutter + changed row
    crop = im.crop((max(0, gx0 - 120), max(0, cy0 - 110), min(w, gx1 + 120), min(h, cy1 + 110)))
    crop = crop.resize((crop.width * 2, crop.height * 2), Image.NEAREST)
    crop.save(CROP)
    print(f"crop saved: {CROP} {crop.size}")
    print("LIVE-PIXEL GATE: PASS")


main()

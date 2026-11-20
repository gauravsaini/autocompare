"""legend25 live-pixel gate: live egui bottom bar ACTUALLY visible.

Structure-anchored, window-position independent (full-screen scan):
1. Find the legend marker swatches for the 3 rare frozen inks
   (CHANGED 122,102,20 / INSERTED 24,90,46 / DELETED 120,32,32) with a
   display-profile tolerance, cluster each into its solid 8px swatch.
2. The 3 swatches fix the legend band; the 4th (track-gray Same cell) is
   the dense gray cluster on that band. Gate (b) holds iff all 4 cells
   are present left-to-right on one band, each ringed by dark bar-face
   pixels (marker ink distinct vs bar face).
3. Gate (a): the per-pane status strip row above the legend band
   (34.4px stack rhythm) carries grayish label ink in BOTH the left and
   right halves (mirrors the left/right pane cells).
Saves a zoomed crop around the bottom bar to live-crop.png.
"""

from PIL import Image

LIVE = "checks/gui-exq-legend25/round-1/live.png"
CROP = "checks/gui-exq-legend25/round-1/live-crop.png"

RARE_INKS = {
    0: (122, 102, 20),  # CHANGED_BG  (differences cell)
    2: (24, 90, 46),  # INSERTED_BG (insert cell)
    3: (120, 32, 32),  # DELETED_BG  (overview cell)
}
GRAY = (48, 48, 48)  # DARK track (same cell)
TOL = 24  # display-profile slack on the frozen consts


def near(p, ink):
    r, g, b = p
    return max(abs(r - ink[0]), abs(g - ink[1]), abs(b - ink[2])) <= TOL


def is_gray(p):
    r, g, b = p
    return abs(r - 48) <= 12 and abs(g - 48) <= 12 and abs(b - 48) <= 12


def groups_of(pts):
    bins = {}
    for x, y in pts:
        bins.setdefault((x // 8, y // 8), []).append((x, y))
    dense = {b for b, ps in bins.items() if len(ps) >= 6}
    assert dense, "FAIL: no dense 8px bin (no swatch)"
    groups = []
    for b in dense:
        placed = False
        for g in groups:
            if any(abs(b[0] - o[0]) <= 1 and abs(b[1] - o[1]) <= 1 for o in g):
                g.add(b)
                placed = True
                break
        if not placed:
            groups.append({b})
    changed = True
    while changed:
        changed = False
        for i in range(len(groups)):
            for j in range(i + 1, len(groups)):
                if any(
                    abs(a[0] - c[0]) <= 1 and abs(a[1] - c[1]) <= 1
                    for a in groups[i]
                    for c in groups[j]
                ):
                    groups[i] |= groups[j]
                    groups.pop(j)
                    changed = True
                    break
            if changed:
                break
    return groups


def cluster_of(pts):
    best = max(groups_of(pts), key=len)
    xs = [b[0] * 8 + 4 for b in best]
    ys = [b[1] * 8 + 4 for b in best]
    return (min(xs), max(xs), min(ys), max(ys))


def swatch_candidates(pts):
    """Swatch-sized merged groups only (bbox <= 40px/side): text-row
    highlight bands are wider and never qualify."""
    out = []
    for g in groups_of(pts):
        xs = [b[0] * 8 + 4 for b in g]
        ys = [b[1] * 8 + 4 for b in g]
        x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
        if (x1 - x0) <= 40 and (y1 - y0) <= 40:
            out.append((x0, x1, y0, y1))
    return out


def main():
    im = Image.open(LIVE).convert("RGB")
    w, h = im.size
    print(f"live.png: {w}x{h}")
    px = im.load()

    # 1. rare inks -> swatch-sized candidates -> band by agreement
    cands = {}
    for idx, ink in RARE_INKS.items():
        pts = [
            (x, y)
            for y in range(0, h, 2)
            for x in range(0, w, 2)
            if near(px[x, y], ink)
        ]
        print(f"ink {idx} {ink}: {len(pts)} matching pixels")
        assert pts, f"FAIL: no pixels near legend ink {idx} {ink}"
        cands[idx] = swatch_candidates(pts)
        print(f"ink {idx}: {len(cands[idx])} swatch-sized candidates")
        assert cands[idx], f"FAIL: no swatch-sized cluster for ink {idx}"
    swatches = {}
    band_y = None
    for c0 in cands[0]:
        cy0 = (c0[2] + c0[3]) // 2
        pick = {0: c0}
        ok = True
        for idx in (2, 3):
            hit = None
            for c in cands[idx]:
                if abs((c[2] + c[3]) // 2 - cy0) <= 40:
                    hit = c
                    break
            if hit is None:
                ok = False
                break
            pick[idx] = hit
        if ok:
            swatches = pick
            band_y = sum((c[2] + c[3]) // 2 for c in pick.values()) // 3
            break
    assert swatches, "FAIL: no 3-ink band agreement (no legend row)"
    for idx, c in swatches.items():
        print(f"ink {idx}: swatch x[{c[0]},{c[1]}] y[{c[2]},{c[3]}]")
    print(f"legend band y ~ {band_y}")

    # 2. gray Same-cell swatch: dense gray cluster on the band
    gray_pts = [
        (x, y)
        for y in range(max(0, band_y - 60), min(h, band_y + 60), 2)
        for x in range(0, w, 2)
        if is_gray(px[x, y])
    ]
    bins = {}
    for x, y in gray_pts:
        bins.setdefault((x // 8, y // 8), []).append((x, y))
    dense = {b for b, ps in bins.items() if len(ps) >= 6}
    assert dense, "FAIL: no dense gray bin on legend band"
    groups = []
    for b in dense:
        placed = False
        for g in groups:
            if any(abs(b[0] - o[0]) <= 1 and abs(b[1] - o[1]) <= 1 for o in g):
                g.add(b)
                placed = True
                break
        if not placed:
            groups.append({b})
    # keep clusters that look like a 16px swatch near the band
    cands = []
    for g in groups:
        xs = [b[0] * 8 + 4 for b in g]
        ys = [b[1] * 8 + 4 for b in g]
        x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
        cy = (y0 + y1) // 2
        if abs(cy - band_y) <= 40 and (x1 - x0) <= 40 and (y1 - y0) <= 40:
            cands.append((len(g), (x0, x1, y0, y1)))
    assert cands, "FAIL: no gray Same-cell swatch on legend band"
    # the Same cell sits inside the row span: keep candidates between the
    # outer rare-ink swatches, closest to the cell-1 slot (1/3 across)
    span0 = min(c[0] for c in swatches.values())
    span1 = max(c[1] for c in swatches.values())
    expect = span0 + (span1 - span0) / 3.0
    inner = [c for c in cands if span0 <= c[1][0] and c[1][1] <= span1]
    assert inner, "FAIL: no gray Same-cell swatch inside the legend span"
    inner.sort(key=lambda c: abs((c[1][0] + c[1][1]) / 2 - expect))
    swatches[1] = inner[0][1]
    print(f"ink 1 (gray): swatch x[{swatches[1][0]},{swatches[1][1]}]")

    # 3. gate (b): left-to-right order + dark bar-face ring per cell
    cxs = [((swatches[i][0] + swatches[i][1]) // 2) for i in range(4)]
    assert cxs == sorted(cxs), f"FAIL: legend cells out of order {cxs}"
    print(f"legend cell centers x: {cxs}")
    for idx in range(4):
        ink = RARE_INKS.get(idx, GRAY)
        x0, x1, y0, y1 = swatches[idx]
        ring = dark = 0
        for x in range(max(0, x0 - 10), min(w, x1 + 10), 2):
            for y in (max(0, y0 - 10), min(h - 1, y1 + 10)):
                r, g, b = px[x, y]
                d = max(abs(r - ink[0]), abs(g - ink[1]), abs(b - ink[2]))
                if d > 12:
                    ring += 1
                    if (r + g + b) / 3.0 < 110:
                        dark += 1
        for y in range(max(0, y0 - 10), min(h, y1 + 10), 2):
            for x in (max(0, x0 - 10), min(w - 1, x1 + 10)):
                r, g, b = px[x, y]
                d = max(abs(r - ink[0]), abs(g - ink[1]), abs(b - ink[2]))
                if d > 12:
                    ring += 1
                    if (r + g + b) / 3.0 < 110:
                        dark += 1
        print(f"ink {idx}: ring distinct={ring} dark-face={dark}")
        assert ring > 0, f"FAIL: gate (b) ink {idx} not distinct vs bar face"
        assert dark > 0, f"FAIL: gate (b) ink {idx} has no dark bar face around it"
    print("gate (b): 4 legend cells each carry marker ink distinct vs bar face")

    # 4. gate (a): per-pane strip row above the legend band.
    # Scale-free: the strip label row is the grayish-text peak in the
    # 130px above the legend band, spanning the legend width; halves
    # split at the span midpoint (mirrors the left/right pane cells).
    lx0 = min(c[0] for c in swatches.values())
    lx1 = max(c[1] for c in swatches.values())
    mid = (lx0 + lx1) // 2

    def grayish_bright(p):
        r, g, b = p
        return 100 <= (r + g + b) / 3.0 <= 220 and max(r, g, b) - min(r, g, b) <= 25

    rows = []
    for y in range(max(0, band_y - 130), band_y - 8):
        n = sum(1 for x in range(lx0, lx1, 2) if grayish_bright(px[x, y]))
        rows.append((y, n))
    rows.sort(key=lambda t: t[1])
    strip_y, peak = rows[-1]
    print(f"strip row y ~ {strip_y} (peak grayish-bright={peak})")
    print(f"legend span x [{lx0},{lx1}] mid {mid}")

    left = right = 0
    for y in range(max(0, strip_y - 6), min(h, strip_y + 7)):
        for x in range(lx0, mid, 2):
            if grayish_bright(px[x, y]):
                left += 1
        for x in range(mid, lx1, 2):
            if grayish_bright(px[x, y]):
                right += 1
    print(f"gate (a): left-half label ink={left} right-half label ink={right}")
    assert left >= 30, f"FAIL: gate (a) no left pane label ink ({left})"
    assert right >= 30, f"FAIL: gate (a) no right pane label ink ({right})"
    print("gate (a): per-pane label ink present in left+right cells")

    crop = im.crop(
        (
            max(0, lx0 - 60),
            max(0, strip_y - 36),
            min(w, lx1 + 60),
            min(h, band_y + 38),
        )
    )
    crop = crop.resize((crop.width * 2, crop.height * 2), Image.NEAREST)
    crop.save(CROP)
    print(f"crop saved: {CROP} {crop.size}")
    print("LIVE-PIXEL GATE: PASS")


main()

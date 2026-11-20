"""toolbar22 live-pixel gate: live egui toolbar ACTUALLY shows glyph slots.

Method (window-position independent, structure-anchored):
1. Window left edge L = first sustained dark column run (egui window bg
   ~20-27 vs neighbouring app content); all chrome must sit right of L.
2. Button rows = rows with long (>=150px) button-face runs (face ~50-60).
   Row 1 = first such band below y=350 holding a solid ~24px dim block
   (Swap solid mark); row 2 = next band (Prev/Next); find row = first row
   group below row 2 holding small (60-250px) face runs (Find Prev/Next).
3. Gate (a): each of the 6 buttons (Swap/Reload/Prev/Next/FindPrev/
   FindNext) has >= 1 dim-ink (DARK_DIM 150) component in its left slot
   zone [bx0-2, bx0+34] overlapping the row vertically.
4. Gate (b): the Prev/Next components are sparse chevron triples
   (density < 0.55, 6x10..16x24 bbox), >= 2 in the nav row.
Saves a zoomed crop of the toolbar band to live-crop.png.
"""

from PIL import Image

LIVE = "checks/gui-exq-toolbar22/round-1/live.png"
CROP = "checks/gui-exq-toolbar22/round-1/live-crop.png"


def gray(p):
    return max(p) - min(p) <= 12


def br(p):
    return sum(p) / 3.0


def is_face(p):
    return gray(p) and 45.0 <= br(p) <= 95.0


def is_ink(p):
    return gray(p) and 120.0 <= br(p) <= 178.0


def face_runs(y, px, x0, x1, minlen):
    runs = []
    s = None
    for x in range(x0, x1):
        if is_face(px[x, y]):
            if s is None:
                s = x
        elif s is not None:
            if x - s >= minlen:
                runs.append((s, x))
            s = None
    if s is not None and x1 - s >= minlen:
        runs.append((s, x1))
    return runs


def main():
    im = Image.open(LIVE).convert("RGB")
    w, h = im.size
    print(f"live.png: {w}x{h}")
    px = im.load()

    # 1. window anchor: median x of strongly-blue selection pixels
    # (session pill + selected tab — unique to our window's chrome).
    bxs = []
    for y in range(250, 520, 2):
        for x in range(0, w, 2):
            r, g, b = px[x, y]
            if b > 80 and b - r > 40 and b - g > 20:
                bxs.append(x)
    assert bxs, "FAIL: no selection-blue anchor pixels"
    bxs.sort()
    xc = bxs[len(bxs) // 2]
    print(f"blue anchor: n={len(bxs)} x in [{bxs[0]},{bxs[-1]}] median xc={xc}")
    x0 = max(0, xc - 400)
    print(f"scan from x0={x0}")

    # 2. button-row bands (long face runs) right of the anchor
    brow = []
    for y in range(150, 1100, 2):
        if face_runs(y, px, x0, w, 150):
            brow.append(y)
    assert brow, "FAIL: no button rows"
    bands = []
    s = p = brow[0]
    for y in brow[1:]:
        if y - p > 20:
            bands.append((s, p))
            s = y
        p = y
    bands.append((s, p))
    print(f"button-row bands: {bands}")

    # ink components across the chrome (step-2 grid, 8-connectivity)
    ylo = max(0, bands[0][0] - 20)
    pts = set()
    for y in range(ylo, bands[-1][1] + 340, 2):
        for x in range(x0, w, 2):
            if is_ink(px[x, y]):
                pts.add((x, y))
    seen, comps = set(), []
    for p0 in pts:
        if p0 in seen:
            continue
        stack, xs, ys = [p0], [], []
        seen.add(p0)
        while stack:
            cx, cy = stack.pop()
            xs.append(cx)
            ys.append(cy)
            for nx in (cx - 2, cx, cx + 2):
                for ny in (cy - 2, cy, cy + 2):
                    q = (nx, ny)
                    if q in pts and q not in seen:
                        seen.add(q)
                        stack.append(q)
        comps.append({"x0": min(xs), "x1": max(xs), "y0": min(ys),
                      "y1": max(ys), "w": max(xs) - min(xs) + 2,
                      "h": max(ys) - min(ys) + 2, "n": len(xs)})
    glyphs = [c for c in comps
              if 4 <= c["w"] <= 30 and 4 <= c["h"] <= 30 and c["n"] >= 6]

    def slot_hit(bx0, yc):
        return [c for c in glyphs
                if bx0 - 2 <= c["x0"] <= bx0 + 34
                and c["y0"] - 20 <= yc <= c["y1"] + 20]

    def overlap(a, b):
        return max(0, min(a[1], b[1]) - max(a[0], b[0]))

    # row 1 = band below y=350 holding the solid ~24px dim block
    solids = [c for c in glyphs
              if 18 <= c["w"] <= 32 and 18 <= c["h"] <= 32
              and c["n"] / (c["w"] * c["h"] / 4) >= 0.6
              and c["y0"] >= 350]
    assert solids, "FAIL: no solid glyph block (Swap mark)"
    solids.sort(key=lambda c: (c["y0"], c["x0"]))
    solid = solids[0]
    print(f"solid block: x=[{solid['x0']},{solid['x1']}] "
          f"y=[{solid['y0']},{solid['y1']}]")
    row1 = next(b for b in bands if b[0] - 8 <= solid["y0"] <= b[1] + 8)
    row2 = bands[bands.index(row1) + 1]
    print(f"action row1 y={row1}, nav row2 y={row2}")

    # Swap = row1 run containing the solid; Reload = next run right
    y1m = (row1[0] + row1[1]) // 2
    runs1 = face_runs(y1m, px, x0, w, 150)
    print(f"row1 runs: {runs1}")
    swap = next(r for r in runs1 if r[0] <= solid["x0"] + 12 <= r[1])
    right = [r for r in runs1 if r[0] > swap[1]]
    assert right, "FAIL: row1 needs Reload right of Swap"
    reload = right[0]
    y1c = (solid["y0"] + solid["y1"]) // 2
    print(f"row1 center y1c={y1c} (from solid block)")

    # Prev/Next row = one row pitch below row1: 24pt button + 8pt gap
    # at the 2x backing scale = 64px (geometry first, validated below).
    y2c = y1c + 64
    chk = [y for y in range(y2c - 20, y2c + 20, 2)
           if any(b > swap[0] and a < swap[1]
                  for a, b in face_runs(y, px, x0, w, 100))]
    assert len(chk) >= 5, "FAIL: no nav row at one pitch below action row"
    print(f"nav center y2c={y2c} (face rows: {len(chk)} hits in +-20)")
    # x-intervals from the text-free top strip of the nav row
    runs2 = face_runs(y2c - 16, px, x0, w, 150)
    print(f"row2 runs: {runs2}")
    prev = max(runs2, key=lambda r: overlap(r, swap))
    nxt = max(runs2, key=lambda r: overlap(r, reload))
    assert overlap(prev, swap) > 50, "FAIL: Prev column misaligned"
    assert overlap(nxt, reload) > 50, "FAIL: Next column misaligned"
    # find row: first row below nav with >= 2 small runs whose slots
    # BOTH hold real ink (untouched filter buttons have no glyphs)
    fprev = fnext = None
    fy = None
    for y in range(y2c + 40, y2c + 340, 2):
        small = [a for a in face_runs(y, px, x0, w, 60)
                 if a[1] - a[0] <= 250]
        if len(small) < 2:
            continue
        cand = [(small[0][0], "FindPrev"), (small[1][0], "FindNext")]
        hits = [slot_hit(bx0, y) for bx0, _ in cand]
        if all(h and sum(hh["n"] for hh in h) >= 10 for h in hits):
            fprev, fnext, fy = small[0], small[1], y
            break
    assert fprev is not None, "FAIL: no find-row small buttons with glyphs"
    print(f"find row y~{fy}: {fprev} {fnext}")

    buttons = [(swap[0], y1c, "Swap"), (reload[0], y1c, "Reload"),
               (prev[0], y2c, "Prev"), (nxt[0], y2c, "Next"),
               (fprev[0], fy, "FindPrev"), (fnext[0], fy, "FindNext")]
    ok = 0
    for bx0, yc, name in buttons:
        hits = slot_hit(bx0, yc)
        print(f"  {name}: bx0={bx0} yc={yc} slots={len(hits)} "
              f"{[(h['x0'], h['y0'], h['w'], h['h'], h['n']) for h in hits]}")
        assert hits, f"FAIL: gate (a) no glyph slot on {name}"
        ok += 1
    print(f"gate (a): glyph slots on {ok}/6 toolbar buttons "
          f"(ink ~150 vs face ~50, delta >= 40)")

    # gate (b): sparse chevron triples in the nav row
    chev = [c for c in slot_hit(prev[0], y2c) + slot_hit(nxt[0], y2c)
            if 4 <= c["w"] <= 16 and 6 <= c["h"] <= 24
            and c["n"] / (c["w"] * c["h"] / 4) < 0.55]
    print(f"gate (b): chevron triples in nav row: {len(chev)}")
    for c in chev:
        print(f"  chev x=[{c['x0']},{c['x1']}] y=[{c['y0']},{c['y1']}] "
              f"w={c['w']} h={c['h']} pts={c['n']}")
    assert len(chev) >= 2, \
        f"FAIL: gate (b) only {len(chev)} chevrons, need >= 2"

    y0 = max(0, row1[0] - 60)
    y1 = min(h, fy + 60)
    crop = im.crop((x0, y0, w, y1))
    if crop.width < 2000:
        crop = crop.resize((crop.width * 2, crop.height * 2), Image.NEAREST)
    crop.save(CROP)
    print(f"crop saved: {CROP} {crop.size[0]}x{crop.size[1]}")
    print("LIVE-PIXEL GATE: PASS")


main()

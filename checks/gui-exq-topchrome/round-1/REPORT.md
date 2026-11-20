# TOPCHROME REPORT (round-1) — Hinglish-Roman

BC top chrome vs humara gap: BC me toolbar row + folder wali path bar + date/size/rule
file-info line + 3-section status hai; humare paas sirf patli tab strip aur
cramped "[ ] ... V Browse..." header tha. Is wave me geometry pehle, typo baad me,
colors frozen.

## Kya kiya (OWNS only: home.rs + screenshot.rs)

- `home.rs` me naya TOPCHROME headless contract: toolbar 28px row, button floor
  96x24 (min 24px hit-target), 10 generic actions kul 996px; path bar dual flex
  (960 -> 466, 700 -> 336) + folder icon 16 + Browse 75; file-info Small line;
  status 3 sections (960 -> 320/320/320, origins 0/320/640).
- `screenshot.rs` me fixture proof: text fixture me button boxes wala toolbar
  (visible 9 + ">>" overflow marker 960 par), row 114..138, panes 142 se.
  Status segmented strip barkarar (honest row counts).
- Edge fixture: `comparison/topchrome-narrow-empty.json` — narrow <720px
  (700 par 6 visible + 4 overflow) + empty-vs-nonempty header hint
  ("pick both paths above, then Compare" vs "").

## Geometry (pehle)

- Toolbar: H 28 >= button 24; har button >= 96x24; kul 996 > avail 936 (960 par)
  isliye 9 visible + 1 overflow (scroll, clip nahi).
- Narrow: 700 par avail 676, 6 fit + 4 overflow — wrap nahi, scroll marker.
- Path: side pad 12 x2 + gap 4; field = (total-28)/2; tang par 0 floor.
- Status: 3 sections x total/3; OOB index par kul width.

## Typography (baad me)

- Toolbar Body 13, path Mono 12, info/status Small 11 — sab TypoRole reuse,
  koi naya size nahi. Hierarchy: Body 13 > Small 11, Mono content, baaki chrome.

## Colors

- Frozen: zero naya Color32/Visuals (harness count 33, pehle jitna hi).

## Gates

- cargo test -p autocompare-gui: 804 passed, 0 failed
- clippy -D warnings: 0
- rustfmt --edition 2024 --check: clean
- banned non-test: 0 (unsafe/unwrap/expect/panic/todo/unimplemented/indexing)
- harness: RESULT PASS, after.png 10401B magic-ok distinct-256

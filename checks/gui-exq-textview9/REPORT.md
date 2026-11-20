# TEXTVIEW9 REPORT — panes plus density (Hinglish-Roman)

Round: `checks/gui-exq-textview9/round-1` — harness 1 round (max6 me se 1 hi laga).

## Kya gap tha (BC vs ours, apne shabdon me)

BC ke text panes dense hain: line numbers gutter me right-aligned, badli hui
shabdon par inline block+underline, aur neeche har pane ke liye alag cursor
detail (`1:1` jaisa per-pane readout). Ours me top par 3 rows ka cursor strip
tha (cursor line + left preview + right preview stacked) jo panes se height
khaata tha, aur neeche pane status halves me sirf nange `L3` / `R-` labels —
koi line detail nahi. Matlab upar hawa, neeche khaali, beech me panes sikude.

## Kya kiya (sirf `crates/gui/src/views.rs`, geometry FIRST)

Geometry first (koi naya size/token nahi, sab reuse):
- Top cursor strip 3 rows se 1 compact row: `paint_text_cursor_strip` se do
  stacked preview `ui.label` hata diye (single `horizontal` + `TIGHT_GAP`
  wahi). 2 rows ki chrome panes ko wapas — neeche void kam, panes lambe.
- Bottom detail dense: `paint_text_pane_status` halves ab
  `text_pane_detail_label` se `L3 | content` dikhate hain (missing side ya
  no-selection/OOB par label akela, koi fake preview nahi). `truncate` clip
  se bar hamesha single `HEADER_H` row me lock — lambi line wrap karke bar
  ko lamba nahi karti. Halves ab bhi `pane_status_half_width` + `TIGHT_GAP`
  par (geometry wahi, content hi badla).
- Naye pure helpers (headless-tested): `text_pane_detail_budget` (80 =
  `TYPO_WRAP_PANE_CHARS` reuse, pane paint width parity),
  `text_pane_preview_side` (trimmed side text, missing/OOB par khali),
  `text_pane_detail_label` (budget par `...` samet char-clip).

Typo second (koi naya size/weight nahi):
- Detail halves `TypoRole::Mono` reuse (code content hai, chrome nahi) —
  cursor meta line `chrome_status_role` par hi hai. `.size` naya kuch nahi
  (report me sirf purana `14.0` dikhta hai).

Colors frozen: diff me 0 naya `Color32`/`Visuals` (report count 33, pichhle
wave jaisa, growth nahi).

## Edge fixture (detail density)

`round-1/comparison/edge-textview9.json`: avail 936 par half 466.0
(`(936-4)/2`), budget 80, 200-char preview `L9 | yyy...` + `...` par clip,
missing side `R-` akela; strip 3 rows se 1 (2 rows panes ko wapas). Naya
headless test `textview9_pane_detail_bottom_density` yehi gate lock karta hai;
purane tests (`pane_status_halves_share_avail_evenly`, cursor-skip,
trailing/filler) rhythm pehle se lock rakhte hain.

## Natija

- PNG 10401B, magic ok, distinct 256 (blank nahi) — `round-1/after.png`
  (fixture frozen hai, isliye pichhle waves jaisa size; density live paint +
  headless gates me hai).
- 864 tests green (naya textview9 test samet), clippy 0, fmt clean (sirf
  owned `views.rs`), banned 0 (non-test scope), colors 0 growth.
- Harness `RESULT: PASS` (1 round).

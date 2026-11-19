# BT-DIFF REPORT — P0-2 selective word highlight (views.rs only)

Scope: BS-LOCALRUN P0-2 — BC me sirf changed WORD par selective highlight +
khaali jagah hatched filler, ours me full-row bands (fixture-text row 3/4/5).
Fix `crates/gui/src/views.rs` paint_text row/accent fns me: highlight changed
span tak, equal plain, filler geometry. Colors FROZEN — koi naya `Color32` nahi.

## Geometry table before/after

| Row kind | Pehle (full-row band) | Ab (selective + filler) |
|---|---|---|
| Equal | plain TRANSPARENT (same) | plain TRANSPARENT (same, lock test) |
| Changed text side | poora-row CHANGED_BG band, accent inverted light block | plain TRANSPARENT base, sirf accent span CHANGED_FG/BG band + 1px underline |
| Deleted left / Inserted right | text side band (same) | text side band (same, lock test) |
| Deleted right / Inserted left (khaali) | khaali side bhi band (full-row) | filler plain TRANSPARENT — hatch `filler_hatch_step/count` geometry se (paint wiring agle wave, screenshot frozen) |
| Filler pad | `pane - visible` inline har jagah | `filler_pad_width` single helper (pane/find/wrap 3 sites reuse) |
| Find current selective base par | inversion over TRANSPARENT (toota text) | light block (inverted changed band, no underline) — accent (dark+line) se alag |
| Selection | full-row CHANGED_BG (same) | full-row CHANGED_BG (same, per-side override) |

## Exact lines (views.rs)

- `selective_row_is_plain` :3499 — Changed/Equal plain, Deleted/Inserted band.
- `selective_side_palette` :3507 — per-side pair; khaali side filler plain
  (`text_side_is_filler` reuse), Changed/Equal plain, Deleted/Inserted band.
- `selective_span_format` :3529 — plain passthrough, accent CHANGED_FG/BG +
  `INTRA_SPAN_UNDERLINE_W`; same mono size.
- `selective_find_format` :3549 — Plain/Other `find_segment_format` reuse,
  Accent selective reuse, Current selective light block / band par old hit.
- Filler geometry :3573-3598 — `text_side_is_filler`, `filler_pad_width`,
  `filler_hatch_step` (TIGHT_GAP reuse), `filler_hatch_count`; hatch
  contract-only (screenshot.rs KEVL frozen, paint wiring future).
- Paint loop: wrap list (~:2591) + nowrap list (~:2800) per-side palette +
  selection override; 4 side-job fns selective format par.
- Tests :6891-7030 — 5 naye (plain/side/span/find/filler); 2 purane selective
  band par update (`side_job_accents...`, `intra_spans_visible...`).

## Gates evidence

- G1 baseline: `round-1/baseline-tests.out` — 623 passed, 0 failed (pre-edit).
- G2 OWNS-only: staged sirf `views.rs` + `checks/gui-diff-bt/**`;
  KEVL-CLEAN (sibling dirty screenshot.rs/viewmodel.rs stage nahi);
  NO-COLOR-ADD (`+` lines me from_rgb/Visuals nahi).
- G3: `cargo test -p autocompare-gui` — 629 passed (623 + 5 naye + 1 sibling?),
  0 failed; selective/filler/span filter green.
- G4: clippy `--all-targets -- -D warnings` zero; `rustfmt --check views.rs`
  0; non-test me unwrap/expect/panic/todo nahi; forbid(unsafe) kept.
- G5: harness `run-geo-typo.sh checks/gui-diff-bt/round-1` RESULT:PASS;
  after.png 9765B, magic 89504e470d0a1a0a, distinct 256.
- G6: artifacts REPORT + GATES + round-1/ (after.png, geo-typo-report.md,
  tests.out, baseline-tests.out, screenshot.out); local commit, NO push.

## Follow-up

- Hatch paint wiring: `filler_hatch_count` ticks ko overview track par khinchna
  (screenshot.rs KEVL wave ke saath, taaki fixture bhi selective dikhe).
- Fixture-text row 3/4/5 screenshot ab bhi full-row band dikhata hai (KEVL
  frozen) — paint parity verify ke liye live egui capture chahiye.
- Deleted/Inserted text-side band vs selective word: BC pattern par ek aur
  chhota wave ho sakta hai (is wave me text side band rakha, filler sirf
  khaali side par).

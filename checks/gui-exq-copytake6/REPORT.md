# REPORT — exq-copytake6 round-1 (copy/take actions plus nav)

Wave: COPTAKE6 exquisite, headless geometry + typo (paint-neutral).
Repo: `/Users/ektasaini/autocompare`, branch `main`. NO PUSH.

## Kya kiya (geometry FIRST)

- `copy.rs`: inline arrow glyph `copy_arrow_label` (`>`/`<`, text precedent);
  nav pair `copy_nav_pair_width` = 52 (2x24 + 4 gap, hit floor parity);
  filter `copy_filter_options` = All/Differences/Same + `copy_filter_short_label`
  (All/Diffs/Same, BC parity), segment h 24, gap 0.0 (adjacent segmented).
- `take.rs`: side widths `take_side_button_width` (Left 79 / Center 93 /
  Right 10-char 86, paint `take_side_label` se — Center "Base" nahi);
  uniform `take_bar_button_width` = 93 + gap 4 = total `take_bar_total_width`
  287; `take_bar_needs_hscroll` (240 -> true, 700 -> false, NaN/Inf -> false);
  stepper h 24 + gap 4 + honest `take_stepper_counter_text`
  ("No conflicts" / "Conflict N of M" 1-based saturating / "{M} conflicts").
- 10 naye tests (5 copy + 5 take); edge fixture
  `comparison/copytake-narrow-240.json` (bar 287 vs avail 240 h-scroll,
  nav pair 52 fit, counter honest).

## Phir typography SECOND

- Copy action text: `copy_action_text_role` = `chrome_status()` (Small 11
  weak, status-line precedent) + size/line-height/weight/tracking accessors.
- Take bar text: `take_action_text_role` = Body 13 (button precedent);
  counter: `take_counter_text_role` = Small 11 (status precedent).
- Koi naya size, koi color move nahi (banned grep 0).

## Harness

`tools/exquisite/run.sh checks/gui-exq-copytake6/round-1` -> RESULT: PASS
(PNG 10401 bytes >= 8192, magic ok, non-blank; colors touches 33, unchanged).
Full suite 840 passed. Clippy lib 0 warnings. rustfmt clean.

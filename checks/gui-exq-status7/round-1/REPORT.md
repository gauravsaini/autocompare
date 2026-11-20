# STATUS7 exquisite report — statusbar plus cursor (Hinglish-Roman)

## Kya tha
BC ka neeche wala hissa 3 cheez dikhata hai: har pane ke neeche `1:1 | Default text`,
beech me focused verdict (`Same`), aur sabse neeche `Unimportant differences | Same |
Insert | Load` wali patti. Ours me sirf ek single top line (`cursor | rules`) aur ek
lambi counts line thi — per-pane readout nahi tha, Unimportant/Insert/Load zones nahi the.

## Kya kiya (sirf `crates/gui/src/app.rs`, geometry pehle)
- Top row ab split-aligned per-pane hai: `layout_pane_widths` se left/right width,
  dono me shared row cursor (`1:1`, bina selection `-`) + rules (`Text: exact`, khali
  par `Default text`). Truncate clip, `HEADER_H` height, `SECTION_GAP` spacing.
- Bottom row ab 5 tukdon me: `Unimportant differences: shown|hidden` (Text filter /
  Folder `hide_orphans` se, real state), verdict Body role me (`Same`/`Different`/
  focused kind, bina model `-`), `Insert` (content loaded par), `Load: idle|working|NN%`
  (background job + fraction se), aur aakhir me maujooda counts+status line truncate ke saath.
- Naya size/color kuch nahi: `status_side/verdict/chrome_status` roles, `HEADER_H`,
  `SECTION_GAP`/`action_bar_row_gap` reuse. `views.rs`/theme frozen, untouched.
- Edge fixture `comparison/left-empty.txt` (khali) vs `right-basic.txt` (3 lines):
  headless test compare karke verdict/cursor/unimportant ki honesty prove karta hai.

## Natija
Harness `RESULT: PASS` (PNG 10401B, magic ok, 256 distinct). `cargo test` 847 green,
clippy `-D warnings` clean, rustfmt clean, banned 0, colors untouched (0 naya Color32).

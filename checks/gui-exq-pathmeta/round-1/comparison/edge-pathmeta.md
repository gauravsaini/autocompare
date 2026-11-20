# Edge — pathmeta empty-vs-long (exq-pathmeta round-1)

BC behavior (own words): har pane ka path bar apna path + dropdown caret dikhata hai,
uske neeche second line me us pane ka timestamp/bytes/format/rules aata hai; file
na ho to second line khali rehti hai (koi fake size nahi); bahut lamba path ellipsis
se kat-ta hai, aadhe glyph se nahi.

Is fixture ka proof headless hai (screenshot paint + viewmodel unit):

- Empty left: `PathPaneMeta::default()` -> `path_pane_meta_line` = `""`,
  `path_history_label("")` = `"history"`. Paint second line skip karta hai.
- Long right: `fixtures/very/long/nested/dir/structure/that/keeps/going/right.rs`
  ko `clip_to_width(path, 300, text_width)` se kaato -> `"..."` suffix, `text_width <= 300`.
- Segmented edge: `path_segmented_status_for("L- R5","","","")` -> `"L- R5"` (koi fake segment nahi).

Tests: `viewmodel::pathmeta_edge_empty_vs_long_clips_honestly`,
`screenshot::pathmeta_strip_geometry_holds`. Dono `cargo test -p autocompare-gui` me green.

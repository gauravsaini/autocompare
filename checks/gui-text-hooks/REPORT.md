# GUI-TEXT-HOOKS P0 prefs plumbing — REPORT (2026-11-20)

Mission: `views::paint_text` me prefs hook taaki word-wrap / line-numbers /
syntax toggles panes me dikhe. Scope: owned files hi chhue —
`crates/gui/src/views.rs`, `gutter.rs`, `viewmodel.rs`,
`checks/gui-text-hooks/**`. `app.rs` / `screenshot.rs` read-only rakhe.

## Kya change hua

1. `viewmodel.rs` me central prefs hook (headless, no duplication):
   - `TextViewPrefs { word_wrap, show_line_numbers, syntax_highlight }`,
     `Default` App defaults se match (wrap off, numbers on, syntax on).
   - `effective_pane_chars_for` (wrap on => min 80), `effective_gutter_chars_for`
     (numbers off => 1).
   - `view_text_with_prefs(model, prefs)`: numbers off => `left_no`/`right_no`
     `None`, syntax off => accents clear, wrap on => pane 80 cap; selection +
     filter passthrough taaki `filter_counts` honest rahe.
2. `views.rs` me paint-side hook:
   - `paint_text_with_prefs(ui, model, prefs)`: naya entry point; `wrap_mode`
     Wrap/Extend set karta, widths `view_text_with_prefs` se leta, counts
     `filter_counts` se (prefs se counts nahi badalte), thumbnail gutter
     `gutter_bar_width` se.
   - `paint_text(ui, model)` purana signature wrapper banke default prefs se
     delegate karta — App call-site compile hota rahe.
   - `TextPaneLayout { pane_chars, gutter_chars, prefs }` + `side_job_with_prefs`
     (6 args, lint-clean); `prefs_pane_segments` syntax off par accents clear
     karke `pane_segments` par delegate karta.
3. `gutter.rs`: `gutter_bar_width(show)` — numbers on 12.0, off 6.0; geometry
   abhi bhi `gutter_geometry` se, counts honest.

## Proof

- Unit: `cargo test -p autocompare-gui` — 107 passed / 0 failed.
  Prefs tests 7 (3 viewmodel naye + 3 views naye + shell ka 1 purana):
  pane 80 cap, gutter 1 narrow, accents clear, selection/filter/counts passthrough.
- Visual: `python3 checks/gui-text-hooks/proof-wrap.py` — RESULT: PASS.
  Teen shots (wrap_off_numbers_on 6353B, wrap_on_numbers_on 5537B,
  wrap_on_numbers_off 5471B): PNG magic ok, non-blank, teeno bytes alag,
  status line teeno me same (`7 rows: 4 equal, 1 changed, 1 deleted,
  1 inserted`). Pane widths Rust tests se locked (200 vs 80), shots sirf demo.
  Note: `screenshot.rs` read-only hai aur uska fixture fixed hai, isliye
  toggle shots alag harness me banaye.
- Clippy: owned `gutter.rs` / `viewmodel.rs` me zero warnings; `views.rs` me
  mere naye code me zero — bache 3 dead-code warnings (`paint_folder`,
  `glyph_color`, `folder_columns`) sibling VIS-FOLDER WIP se hain (usne
  `app.rs` me folder paint call hataya; `app.rs`/`screenshot.rs` uske owned,
  maine chhua nahi). `screenshot.rs` ke 8 `expect_used` deny bhi usi WIP ke.
- Fmt: `rustfmt --edition 2024 --check` owned teen files par clean.

## App wiring (maine `app.rs` nahi chhua — shell/folder agent ke liye)

1. `App::view_text_for_paint` ka body `viewmodel::view_text_with_prefs(model,
   &TextViewPrefs { word_wrap: self.word_wrap, show_line_numbers:
   self.show_line_numbers, syntax_highlight: self.syntax_highlight })`
   par delegate kare (logic ab viewmodel me hai, clone-hook duplicate hat jayega).
2. Text paint call-site: `views::paint_text(ui, &mut view)` ki jagah
   `views::paint_text_with_prefs(ui, &mut view, &prefs)` jahan `prefs` upar
   jaisa App toggles se bane. `paint_text` wrapper tab tak kaam karta rahega.

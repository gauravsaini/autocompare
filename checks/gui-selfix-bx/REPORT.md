# BX-SELFIX REPORT — P0-2 selective highlight fixture visibility

Scope: BU-LOCALRUN2 P0-2 — selective highlight live hai (`f4fb03f`) par
default text fixture me selective style visible nahi. Is wave ka kaam tha:
scene/fixture adjust karke highlight fixtures me VISIBLE karna (ponytail
laziest, production logic change avoid), phir harness se prove karna.
OWNS: `crates/gui/src/views.rs` + `checks/gui-selfix-bx/`.

## Natija (1 line)

Fixture scene trigger nahi hua — shift nahi: accent rect pixel-aligned hai
(expected ax 597 = measured 597). Selective style fixture me laane ke liye
`screenshot.rs` change chahiye jo KEVL MANA hai — isliye is wave me views.rs
me ZERO badlav, aur proof + exact follow-up spec lock kiya. Harness artifacts
+ pixel evidence `round-1/` me, full suite me 1 pre-existing KEVL fail hai.

## Before/after geometry

| Cheez | Pehle (before.png) | Ab (after.png) |
|---|---|---|
| Changed row base (default text, y 172..196) | full-row CHANGED_BG band dono panes (211/230 samples) | SAME — pixel-diff 0 |
| Accent block right (`accent_b` (18,27)) | CHF block x 597..641, y 176..192 | SAME — visible, aligned |
| Accent block left (`accent_a` empty) | koi block nahi (scene data khaali) | SAME — trigger nahi hua |
| Accent scene (after-accent.png, 8734B) | — (naya capture) | Changed rows full-band + dono panes me blocks (L x~61..172, R x~525..644) |
| Live selective model (views.rs, unchanged) | plain TRANSPARENT base + sirf accent span par CHANGED band + 1px underline | SAME — tests lock karte hain |

Selective (plain base + accent-only band) fixture ke kisi scene me NAHI hai —
fixture purane full-band style me paint hota hai.

## Visibility proof (pixel-scan, `round-1/pixel-scan.out`)

- `after.png`: 9765B, magic `89504e470d0a1a0a`, 960x600, distinct 256.
- Changed row band: y 173/178/184/190/195 par L ~191-230/230, R ~173-208/230
  samples CHANGED_BG — full-row band, dono panes.
- Accent block: y 190 par CHF run x 597..641 (45px) — `std::fs::` span visible.
- Alignment: expected ax = 508 + text_width(18-char prefix `"    let content = "`)
  = 508 + 89 = 597; measured min 597. Expected end 597 + 45 = 642, measured
  max 641. Reflow consistent — **shift hypothesis refuted**.
- `cmp before.png after.png` → identical (PIXEL-DIFF-0). Rationale: fixture
  bytes `screenshot.rs` + `viewmodel` se bante hain; `render_text_fixture_png`
  `views::` ka koi fn call nahi karta — views.rs se fixture invariant hai.
- `after-accent.png`: alag scene (4 rows, dono Changed rows me dono taraf
  accents), blocks dono panes me visible — selective TARGET spans wahi hain.

## Tests / lints (gates evidence)

- `cargo test -p autocompare-gui selective` → 4 passed, 0 failed.
- `... filler` → 2 passed; `... span` → 22 passed, 0 failed.
- Full suite: 641 passed, 1 failed — fail pre-existing hai clean HEAD par:
  `app::tests::folder_double_click_stages_file_and_routes_to_text`
  (app.rs:7413, `stage_folder_double_click` Folder deta hai Text ki jagah;
  KEVL MANA isliye no-fix). Harness isi se exit 4 deta hai — geo-typo-report
  nahi banta; `tests.out` + PNGs + `pixel-scan.out` is wave ke artifacts hain.
- `cargo clippy --all-targets -- -D warnings` → 0; `rustfmt --check views.rs`
  → 0; non-test banned (unwrap/expect/panic/todo) → ZERO-BANNED;
  `forbid(unsafe_code)` kept; NO-COLOR-ADD.
- TypoRole reuse: live paint pehle se `TypoRole::Mono.resolved_font_id`
  (views.rs:2575) use karta hai — koi gap nahi, koi tweak nahi.

## views.rs diff kyon zero hai

Selective ke saare headless primitives pehle se pub + tested hain:
`selective_row_is_plain`, `selective_side_palette`, `selective_span_format`,
`selective_find_format`, `changed_row_word_spans`, `normalize_intra_spans`,
filler geometry. Screenshot wave ko adopt karne ke liye kuch missing nahi —
duplicate helper banana YAGNI hota. Ponytail laziest = no-op + proof.

## Follow-up (screenshot/paint wave ke liye, exact spec)

1. `crates/gui/src/screenshot.rs` `render_text_fixture_png` match arms
   (~:301-306): Changed => base `(BG_DARK, FG_TEXT)` plain karo (full band
   hatao), accent spans par `selective_*` jaisa band + 1px underline khincho;
   `render_text_accent_fixture_png` me bhi wahi. Geometry: ax = pane_x +
   text_width(prefix), aw = text_width(span).max(4) — already aligned (597).
2. Fixture Changed row left `accent_a` khaali hai — scene data me left span
   bharo taaki left highlight trigger ho (jaise accent scene me hai).
3. `app.rs:7413` fail (folder double-click routing) owner wave fix kare —
   tabhi full harness RESULT:PASS banega. Ye fail is wave se pehle ka hai.

# Wave U TYPO-EXQUISITE — REPORT (round-U-1)

Scope: `crates/gui/src/theme.rs` + `checks/gui-typo-u/**` ONLY.
`views.rs` / `app.rs` read-only (role usage shape checked via grep only, no edits).
Colors FROZEN, geometry FROZEN. No new deps. Clean-room: no strings/assets copied.

## Baseline -> After

- `cargo test -p autocompare-gui`: 231 passed -> 233 passed, 0 failed
  (+2: `typo_exquisite_tokens_carry_hierarchy`, `typo_exquisite_zoom_clamps_every_role`).
- Harness `checks/gui-typo-u/run-typo-u.sh -> round-U-1/typo-u-report.md`: RESULT PASS.
- PNG: 9914 bytes, magic ok, distinct 256, BIT-IDENTICAL to
  `checks/gui-zoom-r/round-R-1/after.png` (`cmp` clean — paint untouched by design).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0).
- `cargo fmt -- --check`: clean.
- MS guidelines: `#![forbid(unsafe_code)]` kept; zero banned hits in non-test
  code (`unwrap/expect/panic/todo!` grep on pre-`#[cfg(test)]` slice = 0);
  all new fns `const fn`, headless (no `Ui`/`Context`).

## Diff (theme.rs +175/-0, smallest that works)

New headless intent tokens (paint still uses `rich()`/`rich_zoomed()` unchanged,
`extra_letter_spacing(TYPO_LETTER_SPACING)` 0.0 x2, shared measured mono row height):

- Stacks: `TYPO_UI_SANS_STACK` ("system-ui, sans-serif") for body/small/chrome,
  `TYPO_MONO_STACK` ("ui-monospace, monospace") for hex/text-cells/table-cells/gutter;
  `font_stack(role)` returns mono stack only for `Mono`, UI sans otherwise.
  Existing `family()` ("proportional"/"monospace") untouched for egui compat.
- Weights: `TYPO_WEIGHT_REGULAR 400` (Body/Mono/HomeDesc),
  `TYPO_WEIGHT_MEDIUM 500` (Heading/HomeTitle/HomeButton = headers/chrome),
  `TYPO_WEIGHT_SEMIBOLD 600` (Small = badge+counts, sparingly);
  `weight_value(role)`; existing `weight()` strong/weak string untouched.
- Line-height: `TYPO_LINE_HEIGHT_CHROME 1.2` (Heading/HomeTitle/HomeButton/Small),
  `TYPO_LINE_HEIGHT_BODY 1.45` (Body/HomeDesc),
  `TYPO_LINE_HEIGHT_MONO 1.35` (Mono); `line_height(role)` + `line_height_px(role)`
  (= size * multiplier); existing `line_height_source()` + `TYPO_LINE_HEIGHT_SOURCE`
  untouched (paint still measures mono row height).
- Tracking/tabular: `TYPO_TRACKING_HEADING_EM 0.02`;
  `letter_spacing_em(role)` = 0.02 for Heading/HomeTitle else 0.0;
  `is_tabular(role)` = true for Mono (gutter/hex/cells) + Small (counts), false otherwise.
  `TYPO_LETTER_SPACING 0.0` kept so `rich()` output is unchanged.
- Zoom: no resolver change (Wave-R `zoomed_size` already correct); new
  `typo_exquisite_zoom_clamps_every_role` proves every role x every `ZOOM_LEVELS`
  x extreme ±100 stays within `ZOOM_MIN_SIZE..ZOOM_MAX_SIZE` and level 0 == base.

## Role table (pane -> role -> exquisite shape, paint call sites unchanged)

| Pane | Role | Stack / weight / LH / tracking+tabular |
|---|---|---|
| App header title | Heading | UI sans / 500 medium / 1.2 chrome / 0.02em track, non-tabular |
| Home card title | HomeTitle | UI sans / 500 medium / 1.2 chrome / 0.02em track, non-tabular |
| Home card desc | HomeDesc | UI sans / 400 regular / 1.45 body / no track, non-tabular |
| Home card button (chrome) | HomeButton | UI sans / 500 medium / 1.2 chrome / no track, non-tabular |
| Diff panes / summaries / status / gutter / hex / cells | Mono | mono stack / 400 regular-mono / 1.35 mono / no track, TABULAR |
| Hints / subtitles / counts (badge) | Small | UI sans / 600 semibold / 1.2 chrome / no track, TABULAR |
| Plain labels / buttons | Body | UI sans / 400 regular / 1.45 body / no track, non-tabular |

Read-only shape refs (not edited): `views.rs:141` measures mono row height;
`views.rs:147,152,157` read home sizes; `gutter.rs:100-106` Mono passthrough;
`home.rs:122-149` HomeTitle/HomeDesc/HomeButton passthroughs.

## Frozen proof

- `git diff` on theme.rs: +175/-0; zero `-` lines matching
  `SPACE_|GEO_|GUTTER_|Color32::from_rgb|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_`.
- No rectangle/position/size edits, no color-token value changes.

## Files

- `crates/gui/src/theme.rs` (+175: exquisite consts + 5 methods + 2 tests + doc lines).
- `checks/gui-typo-u/GATES.md`, `run-typo-u.sh`, `round-U-1/` (tests.out, screenshot.out, after.png, typo-u-report.md), `REPORT.md`.
- Untouched: views/app/gutter/home/hex/table/tree/viewmodel/main/screenshot, STATUS.md.
  `checks/gui-run-u/` is a sibling wave dir — not touched, not committed.
- No push (orchestrator verifies+pushes).

# Wave AG-CHROME — REPORT (Gap #18: header align / line-height / chrome hierarchy)

OWNS: `crates/gui/src/theme.rs` + `crates/gui/src/views.rs` +
`checks/gui-chrome-ag/**`. KEVL files untouched (read-only grep only).
Colors FROZEN — koi hue/token change nahi, sirf type scale/spacing/alignment.
Commit locally, NO push (orchestrator pushes).

## Bahari observation (apne shabdon me, clean-room)

Bahar se dekhe program me path fields ek global header bar me hain jabki neeche
diff panes visual split par hain — header bar pane-split se offset lagta hai.
Status/count lines mono family me paint hoti hain, jabki session tabs aur
headers proportional chrome hain — poori chrome hierarchy flat lagti hai.
Line-height paint me shared measured mono row par chalta hai, par ye intent
kahin document nahi tha. Koi string/asset copy nahi, sirf layout ka bahari
mushahida.

## Kya badla

`theme.rs` (+110):
- Chrome hierarchy tokens (koi naya point size nahi, sab existing TYPO_* alias):
  `CHROME_TAB_SIZE` 13 (Body), `CHROME_SECTION_SIZE` 18 (Heading),
  `CHROME_STATUS_SIZE` 11 (Small), `CHROME_HEADER_BAR_H` 28 (paging-bar rhythm),
  `CHROME_HEADER_LEAD` "  " (pane-content origin). Ladder documented:
  Heading 18 > HomeTitle 14 > Body 13 > Mono 12 (content only) > Small 11.
- `TypoRole::is_chrome` (Mono ke siva sab chrome), `chrome_tab` (Body),
  `chrome_section` (Heading), `chrome_status` (Small, tabular intent, weak
  tone same — koi hue move nahi).
- Tests: `chrome_sizes_match_typo_scale`, `chrome_roles_carry_hierarchy`.

`views.rs` (+169/-28):
- Pure helpers: `header_bar_height` (= HEADER_H, paging-bar rhythm),
  `header_content_lead` (= CONTENT_LEAD), `header_note_line` (lead + text),
  `chrome_tab/section/status_role`, `text_row_height_intent`
  (= `TYPO_LINE_HEIGHT_SOURCE`). `text_row_height` me `debug_assert_eq` intent
  lock — multipliers headless intent rehte hain, paint shared measured row hi
  leta hai.
- Paint routing (tone same, family sirf status chrome par mono->proportional):
  header title `chrome_section` (18 strong), header note `chrome_status` +
  shared lead par (offset fix: note ab table rules/placeholders wali baseline
  se start hota hai; bar `header_bar_height` reserve karta hai), Home section
  header `chrome_section`, 10 tab labels `chrome_tab` (Body 13 default),
  8 summary/status lines (hex/table-keyed/over-cap, folder x2, picture,
  sync/folder-merge/audio checklists) `chrome_status` (11 weak, tabular).
  Mono content rows, folder/table headers (cell-aligned mono), weak tones —
  sab untouched.
- Tests: `header_bar_tokens_align_with_paging_bars`,
  `header_helpers_use_shared_tokens`, `text_row_height_intent_is_measured_mono`.

## Gates saboot

- G1 baseline: 339 passed / 0 failed (`round-1/baseline-tests.out`).
- G2 OWNS: diff sirf `theme.rs` + `views.rs` + `checks/gui-chrome-ag/**`;
  KEVL-CLEAN; NO-COLOR-ADD (koi `from_rgb`/`Visuals::` add nahi).
- G3: `cargo test -p autocompare-gui` → **344 passed / 0 failed**
  (339 + 2 theme + 3 views naye).
- G4: `clippy -p autocompare-gui --all-targets -- -D warnings` → 0 warnings;
  `rustfmt --edition 2024 --check` OWNS files → clean; non-test me
  ZERO-BANNED (`unwrap()`/`expect()`/`panic!`/`todo!` nahi); `forbid` dono
  files me; indexing nahi (`.get()` only; ek `#[default]` attribute hit jo
  pre-existing hai, indexing nahi).
- G5: `run-geo-typo.sh checks/gui-chrome-ag/round-1` → **RESULT: PASS**
  (PNG 9624 bytes, magic `89504e470d0a1a0a`, distinct 256).
- G6: ye commit (sirf OWNS paths staged), NO push.

## Bacha hua / note

- Folder/table column headers mono hi rahe (cell-aligned hain — family change
  se alignment toot-ti, isliye chrome exception document hai, paint untouched).
- Per-role line-height multipliers ab bhi headless intent hain; paint adoption
  agli wave me (is wave ka contract: intent document + shared measured row).

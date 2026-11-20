# BU-PATHMETA round-1 — P1-1 path meta row (timestamp/bytes/rules/flags/history)

## 1. Summary

`checks/gui-localrun-bs/REPORT.md` section 5 ke P1-1 ka fix is wave me land hua: BC path row ke neeche meta row hota hai (timestamp, byte size, rules label, flags, history dropdown), ours me sirf fields + Browse the. Ab har compare tab ke path row ke neeche ek compact meta row paint hota hai — left metas left se, history dropdown beech me (sirf labels, selection kabhi nahi badalti), aakhri meta right-end dock. History data App state me capped/dedupe rehta hai aur compare start par record hota hai. Harness `round-1/` me `RESULT: PASS`. Colors zero new, banned zero, STATUS untouched.

## 2. Unlazy (GATES pehle, implement baad me)

`checks/gui-pathmeta-bu/GATES.md` me CHECK/EXPECT table pehle likha (G1-G8), phir code implement kiya. Ponytail laziest: koi naya token/size/color nahi — geometry sirf `TIGHT_GAP` spacing + row gap ka reuse, typo sirf `TypoRole::caption()` (Small 11 weak proportional) ka reuse, cap `home::HOME_RECENT_MAX` ka reuse.

## 3. Harness + gates evidence (round-1, SKILL loop)

- `sh tools/gui-polish/run-geo-typo.sh checks/gui-pathmeta-bu/round-1/` -> exit 0, `geo-typo-report.md` me `RESULT: PASS` (round 1/6).
- `after.png`: PNG image data, 960x600, RGBA, 9765 bytes (>= 8192 ok), magic `89504e470d0a1a0a` ok, distinct 256 (> 64 ok).
- `cargo test -p autocompare-gui`: 634 passed, 0 failed (mere 5 naye tests samet: views me 2, app me 3).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: exit 0, zero warnings (beech me ek test-indexing warning aaya tha, `.first().is_some_and(...)` se fix).
- `cargo fmt -- --check` OWNS par clean (`views.rs`, `app.rs`).
- Banned grep (non-test): sirf 2 `#![forbid(unsafe_code)]` header lines; koi unwrap/expect/panic/todo/unsafe usage nahi.
- Colors: `git diff` me `Color32`/`Visuals` zero new.
- `git diff --name-only | grep STATUS` khali — STATUS untouched. `home.rs` me sibling wave ka uncommitted change dikha, stage/touch nahi kiya.

## 4. Geometry table — before/after

| Row | Before | After |
|---|---|---|
| Path row | fields + Browse + Compare ek `horizontal` me, `item_spacing.x = TIGHT_GAP`, uske baad `add_space(TIGHT_GAP)` | same, uske neeche meta row judta hai |
| Meta row (`views.rs:391 paint_path_meta_row`) | nahi thi | ek `horizontal`: pehle metas left se (`path_meta_role` labels), beech me `ComboBox::from_id_salt("path_meta_history")` (options sirf `ui.label`, koi `selectable_value` nahi — selection untouched), aakhri meta `with_layout(right_to_left)` se right-dock; row ke baad `add_space(TIGHT_GAP)` |
| Spacing tokens | `TIGHT_GAP` | `TIGHT_GAP` hi (koi naya gap nahi) |
| Typo roles | path fields default, buttons Body | meta labels `TypoRole::caption()` = Small 11 weak proportional (mono nahi — BC chrome jaisa) |
| Triple tabs (Merge/FolderMerge) | 3 fields, meta nahi | ek meta row: left slot me `base || left`, right slot me `right` (`app.rs:608 active_path_meta`) |

## 5. Exact lines (OWNS only)

- `crates/gui/src/views.rs:368 path_meta_role` — caption reuse.
- `crates/gui/src/views.rs:374 format_path_meta` — `timestamp | bytes | rules | flags`, khali parts girte hain.
- `crates/gui/src/views.rs:391 paint_path_meta_row` — compact meta row + history dropdown (labels only).
- `crates/gui/src/views.rs:7183/7189` — 2 pure tests.
- `crates/gui/src/app.rs:186 path_history` field + `app.rs:333` Default.
- `crates/gui/src/app.rs:513 path_history_items`, `:520 record_path_history` (dedupe + cap 8), `:534 path_byte_label`, `:547 path_time_label` (missing par khali, koi fake "0 B"/"just now" nahi), `:566 path_rules_label`, `:583 path_flags_label`, `:596 path_meta_for`, `:608 active_path_meta`.
- `crates/gui/src/app.rs:1145/1147` — compare start par history record (status line untouched).
- `crates/gui/src/app.rs:4836+` — 8 tab arms (Text/Folder/Hex/Merge/Table/Sync/Picture/FolderMerge/Audio) me `paint_paths`/`paint_triple_paths` ke baad `paint_path_meta_row` call; borrow sequential (pehle owned Strings, phir paint), koi signature change nahi.
- `crates/gui/src/app.rs:5841+` — 3 headless tests (dedupe/cap, honest labels, rules+flags+triple join).

## 6. Follow-up (orchestrator ke liye)

1. Meta row me live timestamp dikhega jab file exist karegi (headless `--screenshot` me paths khali hain, isliye meta khali + rules/flags only — ye honest-empty hai, fake data nahi).
2. History dropdown abhi empty-state me hai jab tak koi compare na chale; `record_path_history` sirf `run_active_tab` (job start) par record karta hai — purane sessions se backfill chahiye to alag P-proposal banao.
3. Cursor-detail strip (P1-3) aur 3-zone status bar (P1-2) is wave ke bahar hain — BT waves ke OWNS hain, haath nahi lagaya.

# Wave Q FIND report (text find-box + count badge)

## Scope
- Owns: `crates/gui/src/app.rs`, `checks/gui-find-q/**`.
- No touch: views.rs/theme/gutter/hex/table/tree/viewmodel/main/screenshot, STATUS.md. No push.

## Gap #12 fix
- Text view me find-box nahi tha, N/M badge nahi tha, hex find UI nahi tha.
- Table Find (`app.rs:table_find_next_cell`) + folder Find pattern reuse kiya; text F3 loop (`views.rs:712-731`) untouched rakha.

## Model (app.rs file:line)
- State fields `app.rs:101-105`: `text_find_needle`, `text_find_cursor`, `text_find_ignore_case`, `hex_find_needle`, `hex_find_cursor`. Default `app.rs:155-159`.
- Text match fns `app.rs:823-945`: `text_find_contains`, `text_find_match_rows`, `text_find_matches`, `text_find_total`, `text_find_position`, `text_find_badge` ("N/M", empty par "0/0"), `text_find_set_needle`, `text_find_next_row`, `text_find_prev_row` (selection+status update, wrap).
- Hex fns `app.rs:947-1046`: `parse_hex_needle` (hex jode else raw utf-8), `hex_find_count`, `hex_find_next_from`/`hex_find_prev_from` (views_hex reuse), `hex_find_advance`/`hex_find_retreat` (cursor+status).
- Paint: Text tab me Find row `app.rs:~2455` (TextEdit 120px + Ignore case + Prev/Next + badge label). Koi color/token change nahi.

## INTEGRATION.patch (views.rs hook, apply-only)
- File: `checks/gui-find-q/INTEGRATION.patch` (2058 bytes, 1 hunk, append-only file end par).
- Jodta hai: `text_find_badge_label(position, total)` pure N/M label + `paint_text_find_bar(ui, needle, ignore_case, badge)` (TextEdit+checkbox+Prev/Next+badge, existing tokens only).
- Check: `git apply --check .../INTEGRATION.patch` -> PASS. views.rs working tree ko haath nahi lagaya.

## Tests
- `cargo test -p autocompare-gui`: 218 passed, 0 failed (3 naye: badge/nav, wrap/case-flag, hex parse/reuse).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: PASS (owned app.rs zero; beech me sibling views.rs dead_code red tha, retry par green).
- `rustfmt --edition 2024 --check crates/gui/src/app.rs`: PASS.
- Harness `tools/gui-polish/run-geo-typo.sh checks/gui-find-q/round-Q-1`: RESULT PASS (PNG 9914 bytes, magic ok).

## Sibling note
- Round beech me `views.rs` (grid agent owned) red tha: pehle syntax break, phir clippy dead_code (`hunk_covers_offset`, `format_hex_grid_row`, `hex_grid_rows_for_hunk` +2). Owned gates tab bhi green rakhe; wait+retry par full green. views.rs ko fix nahi kiya (ownership).
- Commit: local only, no push, STATUS.md untouched.

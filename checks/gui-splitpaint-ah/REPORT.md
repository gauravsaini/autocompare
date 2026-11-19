# Wave AH-SPLITPAINT — REPORT (Gap #1 paint half: splitter drag-handle paint)

## Summary

`views.rs` ka shared `paint_split_handle` ab Wave AG ke landed geometry model ko consume karta hai: drag-delta helper, resize-column cursor affordance, a11y hover label, hit-width parity. Natija: text path row + picture side-by-side + dual-tree — teeno ke drag handle dikhte hain aur drag se split badalta hai. Koi signature change nahi, App untouched.

## Key metrics

- Tests: 344 passed (baseline, `main@9cc197a`) → 355 passed, 0 failed (delta: +5 is wave ke, +6 sibling audio-ah ke)
- New tests: 5 (`cargo test -p autocompare-gui split_handle` green)
- Clippy: 0 warnings (`--all-targets -- -D warnings`)
- fmt: 0 (`cargo fmt -- --check crates/gui/src/views.rs`)
- Banned (non-test): ZERO-BANNED, NO-UNSAFE, `forbid(unsafe_code)` kept
- Harness: `RESULT: PASS` — PNG 9624 bytes, magic `89504e470d0a1a0a`, distinct 256

## Highlights

- ✅ `paint_split_handle` upgrade: `split_handle_drag_fraction` (model `split_drag_delta_fraction` + shared clamp) se drag; `on_hover_cursor(ResizeColumn)` + `on_hover_text(split_accessibility_label)` — model ids se mapped, colors frozen (`bg_fill` hi rehta hai)
- ✅ Pure wrappers: `split_handle_cursor` / `split_handle_label` / `split_handle_drag_fraction` — sab unit-tested
- ✅ Hit-width parity: `debug_assert_eq!(SPLIT_HIT_W, SPLIT_HANDLE_HIT_W)` + `rect.width()` assert + test (8.0 strip, 1px centered visual, 3.5 inset)
- ⚙️ KEVL untouched: `viewmodel.rs` / `app.rs` sirf read-only use (model fns call, koi edit nahi); teeno call-sites (`paint_paths`, picture-SideBySide, `paint_folder_dual`) bina signature change ke faida uthate hain

## Blockers

- 🚨 Beech me sibling wave (audio-ah) ke uncommitted `app.rs` edit se `cargo test` compile fail hua tha (missing struct fields, non-test code) — meri file se unrelated. Sibling ne khud fix kiya; dobara green hone ke baad hi aage badha. Koi file touch nahi ki, koi stash/push nahi.

## Risks / decisions needed

- [ ] Text diff panes ke beech abhi bhi static `show_split_content` hai (App-owned, KEVL) — wahan drag wiring orchestrator/sibling scope me hai; is wave ne path-row handle wahi chhoda jahan App pehle se `&mut split_fraction` deta hai

## Notes

- OWNS: `crates/gui/src/views.rs` + `checks/gui-splitpaint-ah/**` — commit me sirf yehi staged; sibling paths (`app.rs`, `checks/gui-audio-ah/`) untouched/unstaged
- Geometry first, typography second, colors frozen — handle visual me koi naya hue/size nahi

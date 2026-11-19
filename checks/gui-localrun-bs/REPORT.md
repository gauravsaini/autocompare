# BS-LOCALRUN round-1 — live app + BC visual gap report (geometry + typography only)

## 1. Summary

`target/debug/autocompare-gui` binary launch kiya (PID 21062, dev build), Home window live capture kiya, BC Text Compare window ke saath side-by-side observe kiya. Crates me ZERO edits — sirf `checks/gui-localrun-bs/` add hua. Natija: Home live solid hai (9 cards, 3-col grid sahi), Text tab ka diff layout aur path-row meta BC se sabse bade gaps hain (P0/P1). Colors out of scope — report me note only, koi palette spec nahi.

## 2. Live run spec

- Branch: main, `git fetch origin` me kuch naya nahi (local 1 commit ahead hai — STATUS wali, usko touch nahi kiya).
- Build: `cargo build -p autocompare-gui` green (Finished dev profile, binary 75MB arm64).
- Launch: binary direct background me (`nohup ... > round-1/cargo-run.log`), PID 21062; `cargo run` wrapper use nahi kiya isliye log khaali hai — launch proof PID + window frame se hai.
- App window frame (osascript, observe-only): pos 320,91 size 800x632. BC window frame: pos 720,31 size 720x869.
- Headless renders: `--screenshot` filename-keyed scenes (home/text/folder/table), sab 960x600.
- Probe strategy: one-shot capture, action ke baad sleep, koi tight loop nahi. Koi window close/move/click nahi, sirf capture + observe. Tab switch nahi kiya (observe-only), isliye text/folder/table ke live captures nahi — unke liye headless fixtures evidence hain.
- Tests: `cargo test -p autocompare-gui` — 611 passed, 0 failed.

## 3. Shot map (sab round-1/ me, file magic ok)

| File | Size | Magic | Kya hai |
|---|---|---|---|
| live-desktop.png | 2880x1800 | PNG RGBA | Full side-by-side: Home front, BC peeche |
| live-home.png | 1600x1264 (@2x = 800x632) | PNG RGBA | `-R320,91,800,632` region crop, full Home window title-bar samet |
| live-bc.png | 1440x1738 | PNG RGBA | `-R720,31,720,869` region crop, BC Text Compare (left half Home se occluded, as-is) |
| fixture-home.png | 960x600 | PNG RGBA | headless Home scene |
| fixture-text.png | 960x600 | PNG RGBA | headless Text scene (visually verified) |
| fixture-folder.png | 960x600 | PNG RGBA | headless Folder scene |
| fixture-table.png | 960x600 | PNG RGBA | headless Table scene |
| cargo-run.log | — | text (empty) | direct binary launch tha, isliye khaali; launch proof section 2 me |

Gate check (har PNG): magic-ok + size>=8192 + distinct>64 — 7/7 PASS (detail GATES.md me).

## 4. BC side-by-side observation (BC 5, Text Compare `t_left<-->t_right`)

BC chrome (live-bc.png se, apne shabdon me):
- Session tab bar: 3 open sessions (text, folder, table CSV), har tab par close-x. Hamare paas single-row 9 mode tabs hain, session concept nahi.
- Toolbar: Rules / Format (dropdown) / Copy icon buttons. Hamare paas text buttons Swap Sides + Reload/Recompare.
- Path row: left/right path fields + dropdown + icons. Uske neeche meta row: timestamp, byte count, rules label, flags. Hamare path row me sirf fields + Browse hain, meta row nahi.
- Diff area: line numbers, sirf changed word par selective highlight, equal rows plain, khaali jagah hatched filler. Hamara full-row band layout hai (fixture-text me verified: row 3 poori chaudai band + right pane me inverted word block).
- Bottom: cursor strip (`1:1`, focused line preview rows), 3-zone status bar (left flags, mid Same, right hints). Hamari single counts line hai.
- Typography: BC chrome proportional font, mono sirf diff text me. Hamara UI mono-heavy hai (54 monospace uses).

## 5. Gap table — geometry + typography only (ranked)

### P0 (user-visible, pehle fix)

| ID | Observed | Exact follow-up spec (file/fn) |
|---|---|---|
| P0-1 | Session tab bar nahi: BC ek window me multiple closable sessions, hamara ek active Tab + 9 mode tabs | `crates/gui/src/app.rs` (Tab enum, per-session Paths model), `crates/gui/src/views.rs` (tab strip paint fn) |
| P0-2 | Text diff layout: BC selective word highlight + hatched filler, hamara full-row bands (fixture-text row 3/4/5 me verified) | `crates/gui/src/views.rs` (paint_text row/accent fns) |

### P1 (parity, agle round)

| ID | Observed | Exact follow-up spec (file/fn) |
|---|---|---|
| P1-1 | Path-row meta missing: timestamp, byte size, rules label, flags, history dropdown | `crates/gui/src/views.rs` (path row fn), `crates/gui/src/app.rs` (paths state) |
| P1-2 | Status bar one-sided: BC 3 zones + cursor `1:1`, hamari single counts line | `crates/gui/src/views.rs` (status paint fn), `crates/gui/src/viewmodel.rs` (counts/summary) |
| P1-3 | Cursor-detail strip missing: BC focused-line preview, hamare paas nahi | `crates/gui/src/views.rs`, `crates/gui/src/viewmodel.rs` (cursor model) |
| P1-4 | Home card CTA wrap: live-home me "Open Text / Compare" jaise labels 2 line me wrap hote hain — button width budget tight | `crates/gui/src/home.rs` (card layout fns), `crates/gui/src/views.rs` (CTA paint fn) |

### P2 (polish / harness-only)

| ID | Observed | Exact follow-up spec (file/fn) |
|---|---|---|
| P2-1 | Toolbar icon parity: BC Rules/Format/Copy icons vs hamare text buttons (geometry: icon-size budget vs text-button width) | `crates/gui/src/views.rs` (toolbar fn) |
| P2-2 | Fixture renderer artifact: fixture-text toolbar line me glyph overlap/blocks — live egui me nahi, sirf `screenshot.rs` ka issue | `crates/gui/src/screenshot.rs` (draw_string spacing fn) |

## 6. Colors note (scope bahar, spec nahi)

Live window Light Mode me khula, fixtures dark frozen palette par hain. Koi palette change propose nahi kar rahe — ye note sirf context ke liye hai. Carried-over open item (BO round se): live Light Mode vs `dark_mode=true` default ka mismatch, root cause abhi bhi open — follow-up `crates/gui/src/app.rs` (Default), `crates/gui/src/theme.rs`.

## 7. Geometry/typography signals (read-only grep, headless-first)

- Grid: `Grid::new` x10 (views.rs 8, views_table.rs 2). `text_style_height` x4 (theme.rs 2, views.rs 2).
- Typography: monospace x54, strong x23, small x56, `.size(14.0` + bare `.size(` uses.
- `add_space(` calls sab multi-line hain (koi inline numeric literal nahi) — spacing values code me distributed hain.
- Tests: 611 passed, 0 failed (harness-equivalent: build + screenshot + test + PNG gate, fix zero).

## 8. craft mapping + loop suggestion

- Study: BC ko sirf observe kiya (session tabs, toolbar, meta row, status zones), koi decompile/copy nahi.
- Build: is run me ZERO edits — crates doosre waves ke OWNS hain.
- Check: build + `--screenshot` fixtures + one-shot screencapture + `file` magic gate (GATES.md).
- Loop (agle round ke liye): parity fixtures BC me + hamare app me same paths (observe-only) -> test + captures round-N me -> gap table se ek P0 uthao, single-OWNS minimal diff -> repeat. Har round REPORT+GATES Hinglish-Roman me.

## 9. Notes

- Is run me crates/**, scripts/**, tools/**, skills/**, baaki checks dirs ko touch nahi kiya — sirf `checks/gui-localrun-bs/`.
- Cleanup: apna instance (PID 21062) kill kiya; BC (PID 23316) aur doosre waves ke processes ko haath nahi lagaya.
- PUSH MANA tha — commit local rahega.

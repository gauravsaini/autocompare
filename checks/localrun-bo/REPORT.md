# BO-LOCALRUN round-1 — live app vs Beyond Compare visual gap report

## 1. Summary

`cargo run -p autocompare-gui` background me launch kiya (PID 6133, dev profile, exit nahi — run karta raha), Home window live capture kiya, BC Text Compare window ke saath side-by-side observe kiya. Crates me ZERO edits. Result: Home live parity solid hai, Text tab ka visual language BC se sabse bada gap hai (P0), path-row meta + status bar P1 hain.

## 2. Live run spec

- Branch: main, commit 8b5ff4b (pull fresh, already up to date)
- Command: `cargo run -p autocompare-gui` (background, log: round-1/cargo-run.log — `Finished dev profile`, `Running target/debug/autocompare-gui`)
- App PID 6133, window "Home - autocompare - autocompare", frame 320,91 800x632 (osascript, observe-only)
- Headless renders: `target/debug/autocompare-gui --screenshot <fixture-*.png>` (4 scenes, 960x600)
- Probe strategy: one-shot capture after action + sleep 1-2s settle, koi tight timer loop nahi. Koi window close/move/click nahi, sirf capture+observe.
- Shared box note: PID 1059 (doosre wave ka Table session, parity-harness fixtures) same box par live tha — usko touch nahi kiya. BC window usi wave ka session dikha raha tha.

## 3. Evidence (sab round-1/ me, file magic ok)

| File | Size | Magic | Kya hai |
|---|---|---|---|
| live-desktop.png | 2880x1800 | PNG RGBA | Full side-by-side: Home front, BC peeche |
| live-home.png | 1600x1264 | PNG RGBA | `-R320,91,800,632` region crop, full Home window title-bar samet |
| live-bc.png | 1440x1738 | PNG RGBA | `-R720,31,720,869` region crop, BC Text Compare (left half Home se occluded — BC front me lana manipulation hota, isliye as-is) |
| fixture-home.png | 960x600 | PNG RGBA | headless Home scene |
| fixture-text.png | 960x600 | PNG RGBA | headless Text scene (visually verified) |
| fixture-folder.png | 960x600 | PNG RGBA | headless Folder scene |
| fixture-table.png | 960x600 | PNG RGBA | headless Table scene |
| cargo-run.log | — | text | dev build + Running proof |

Limit: observe-only rule ki wajah se tab switch nahi kiya, isliye text/folder/table ke live captures nahi — unke liye headless fixture renders evidence hain (same frozen palette, same 960x600 canvas).

## 4. BC side-by-side observation (BC 5, Text Compare: t_left.txt <--> t_right.txt)

BC chrome (live-bc.png + live-desktop.png se):
- Session tab bar: 3 open sessions (text `t_left<-->t_right`, folder `left<-->right`, table `tab CSV`), har tab par close-x. Hamare paas single active Tab hai, session tabs nahi.
- Toolbar: Rules / Format (dropdown) / Copy icon buttons. Hamare paas text buttons Swap Sides + Reload/Recompare.
- Path row: left/right path dropdowns + browse icons. Uske neeche meta row: timestamp, `17 bytes`, `Everything Else` (rules), `U U` flags. Hamare path row me sirf fields + Browse hain, meta row nahi.
- Diff area: light theme, line numbers, sirf changed word par blue/red selective highlight (`BETA` blue), equal rows plain, khali jagah hatched filler. Hamara dark theme + full-row bands (changed yellow, deleted red, inserted green) — fixture-text.png me verified.
- Bottom: cursor strip (`1:1 | Default text`, focused line preview `→alpha / ←alpha`), right minimap rail. Hamare paas overview-ruler gutter + N/M find strip hai.
- Status bar: left `Unimportant differences`, mid `Same`, right action hints (`Insert`, `Load...`). Hamara single counts line (`7 rows: ... | ... done`).

## 5. Gap table (ranked)

### P0 (user-visible, pehle fix)

| # | Gap | Exact file OWNS (future wave edit kare) |
|---|---|---|
| P0-1 | Session tab bar nahi: BC multiple closable sessions ek window me, hamara ek active Tab | crates/gui/src/app.rs (Tab enum, per-tab Paths, session model), crates/gui/src/views.rs (tab strip paint), crates/gui/src/home.rs (cards-to-tabs bridge) |
| P0-2 | Text diff visual language: BC selective word highlight + hatched filler (light), hamara full-row bands (dark) | crates/gui/src/views.rs (paint_text), crates/gui/src/theme.rs (palette), crates/gui/src/gutter.rs (overview ruler) |

### P1 (parity, agle round)

| # | Gap | Exact file OWNS |
|---|---|---|
| P1-1 | Path-row meta missing: timestamp, byte size, rules label, U/U flags, path history dropdown | crates/gui/src/views.rs (path row), crates/gui/src/app.rs (paths state), crates/gui/src/home.rs (recent/history) |
| P1-2 | Status bar one-sided: BC left/mid/right zones + cursor `1:1`, hamari single counts line | crates/gui/src/views.rs (status paint), crates/gui/src/viewmodel.rs (counts/summary) |
| P1-3 | Cursor-detail strip missing: BC focused-line preview (`→alpha/←alpha`), hamare paas nahi | crates/gui/src/views.rs, crates/gui/src/viewmodel.rs (cursor model) |
| P1-4 | Theme default anomaly: live window Light Mode me khula jabki `App::default` me `dark_mode=true` — root cause open (eframe persistence ya toggle default?), follow-up chahiye | crates/gui/src/app.rs (Default, dark_mode), crates/gui/src/theme.rs, crates/gui/src/views.rs (Light Mode button) |

### P2 (polish / harness-only)

| # | Gap | Exact file OWNS |
|---|---|---|
| P2-1 | Toolbar icon parity: BC Rules/Format/Copy icons vs hamare text buttons | crates/gui/src/views.rs (toolbar), crates/gui/src/theme.rs |
| P2-2 | Fixture renderer artifact: fixture-text.png ki toolbar line me glyph overlap (block-font artifact) — live egui me nahi hai, sirf screenshot.rs ka issue | crates/gui/src/screenshot.rs (draw_string spacing) |

## 6. craft-anything Study / Build / Check mapping (craft.json)

- Study: craft.json (`mode clean-room`, target BC 5.2.6.32774, `outside-observation-only`, record docs/study.md) — ye run Study hai: BC ko sirf observe kiya (session tabs, toolbar, meta row, status zones), koi decompile/copy nahi.
- Build: crates/gui/src waves (theme/views/gutter/home edits, doosre subagents ke OWNS) — is run me ZERO edits, sirf observe.
- Check: tools/parity-harness/run-round.sh + `cargo test -p autocompare-gui` + `--screenshot` fixture PNGs + screencapture live PNGs + `file` magic gate (GATES.md).

## 7. Recurring loop suggestion (comparison files -> test -> fix -> repeat)

Har round ye outer loop chalao, inner me koi tight timer loop nahi:
1. Comparison files: tools/parity-harness/fixtures/* (t_left/t_right.txt, tab_left/tab_right.csv, left-dir/right-dir) BC me kholo + hamare app me same paths — observe-only.
2. Test: harness run-round.sh + cargo test + `--screenshot` renders + one-shot screencapture (action ke baad sleep 1-2s settle) — PNGs round-N/ me, `file` magic gate.
3. Fix: gap table se ek P0 uthao, single-OWNS minimal diff (ek wave = ek file set), clippy+fmt green.
4. Repeat: next round-N+1 dir me same captures, gap table update — P0 zero hone tak. Har round REPORT+GATES Hinglish-Roman me.

## 8. Notes

- Is run me crates/**, scripts/**, baaki checks dirs ko touch nahi kiya — sirf checks/localrun-bo/.
- Cleanup: report ke baad apna instance (PID 6133 + cargo run wrapper) kill kiya; PID 1059 (doosra wave) ko haath nahi lagaya.
- PUSH MANA tha — commit local raha.

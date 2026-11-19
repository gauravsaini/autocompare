# BU-LOCALRUN2 round-2 — BT batch ke baad rebuild + recapture, round-1 se gap DELTA

## 1. Summary

BT batch (4 commits: glyph fix + selective highlight + 3-zone status model + CTA contract) ke baad app ko force-rebuild kiya, 4 fixtures + fullscreen + Home region PNGs recapture kiye. Crates me ZERO content edits — sirf `checks/gui-localrun-bu2/` add hua. Natija: P2-2 (glyph overlap) headless me fully fixed — 4/4 fixtures byte-level par badle (reflow proof). P0-2/P1-2/P1-3/P1-4 model ya live-paint level par aadhe land hue, paint/fixture wiring abhi baaki hai (har row me exact follow-up spec hai). P0-1 aur P1-1 me koi badlav nahi — dono parallel waves ke OWNS me hain. Tests 611 se 629 hue (+18 BT tests), zero fail.

## 2. Live run spec

- Branch: main, HEAD `05ca3a4` (BT 4 commits included: `b6a7661` shotfix, `c97b677` cardcta, `97cba91` status, `f4fb03f` diff).
- Build: `touch crates/gui/src/*.rs` + `cargo build -p autocompare-gui` (mtime stale tha, isliye force; content diff zero) — Fresh binary 10:50:56, 75MB arm64. Phir dobara build clean (0.15s, fresh).
- Launch: binary direct background me (`nohup ... > round-2/cargo-run.log`), PID 40485; `cargo run` wrapper use nahi kiya isliye log khaali hai — launch proof PID + window frame se hai (round-1 jaisa).
- App window frame (osascript, observe-only): pos 320,91 size 800x632 — round-1 ke frame jaisa hi.
- Headless renders: `--screenshot` filename-keyed scenes (home/text/folder/table), sab 960x600.
- Probe strategy: one-shot capture, action ke baad sleep, koi tight loop nahi. Koi window close/move/click nahi, sirf capture + observe. Tab switch nahi kiya (observe-only, round-1 jaisa), isliye text/folder/table ke live captures nahi — unke liye headless fixtures evidence hain.
- BC app: PID 23316 (round-1 wala hi instance) observe-only, haath nahi lagaya; BC me koi badlav nahi, isliye live-bc recapture nahi kiya.
- Tests: `cargo test -p autocompare-gui` — 629 passed, 0 failed (round-1 me 611 the; +18 BT wave tests). Log `round-2/tests.out` me.

## 3. Shot map (sab round-2/ me, file magic ok)

| File | Size | Magic | Kya hai |
|---|---|---|---|
| live-desktop.png | 2880x1800 | PNG RGBA | Full side-by-side: Home front, BC peeche (round-1 jaisa frame) |
| live-home.png | 1600x1264 (@2x = 800x632) | PNG RGBA | `-R320,91,800,632` region crop, full Home window title-bar samet |
| fixture-home.png | 960x600 | PNG RGBA | headless Home scene (round-1 se bytes differ — glyph reflow) |
| fixture-text.png | 960x600 | PNG RGBA | headless Text scene (round-1 se bytes differ — glyph reflow; selective highlight abhi fixture me nahi) |
| fixture-folder.png | 960x600 | PNG RGBA | headless Folder scene (round-1 se bytes differ — glyph reflow) |
| fixture-table.png | 960x600 | PNG RGBA | headless Table scene (round-1 se bytes differ — glyph reflow) |
| cargo-run.log | — | text (empty) | direct binary launch tha, isliye khaali; launch proof section 2 me |
| tests.out | — | text | `cargo test` full log, 629 passed 0 failed |

Gate check (har PNG): magic-ok + size>=8192 + distinct>64 — 6/6 PASS (detail GATES.md me).

## 4. Fixture byte delta (round-1 vs round-2, sha12)

| File | round-1 bytes | round-2 bytes | Match |
|---|---|---|---|
| fixture-home.png | 14880 `fcd4e60f3dad` | 15007 `ac9946b7acdc` | DIFFER (glyph `t` 4→5px reflow) |
| fixture-text.png | 9695 `1a7c768f3e41` | 9765 `bde8ccf481c6` | DIFFER (glyph reflow; BT waves ke after.png = 9765 se match) |
| fixture-folder.png | 10068 `8bd5b03e930` | 10149 `f9c6a846ef24` | DIFFER (glyph reflow) |
| fixture-table.png | 9973 `d5ba2ca5772c` | 9996 `e1ddba386f22` | DIFFER (glyph reflow) |
| live-home.png | 301696 `be1efb6d5193` | 301202 `4fbbe6892bb5` | DIFFER (desktop state + reflow; frame same 800x632) |
| live-desktop.png | 1075844 `0b94dfd99131` | 762897 `9fe75d40a9a9` | DIFFER (desktop state alag — clock/wallpaper/occlusion; app frame same) |

Note: live PNG size farak desktop state ka hai (time, occlusion), app frame same hai — isko fix proof mat samjho. Asli fix proof fixture bytes + BT wave tests hain.

## 5. Gap DELTA table — round-1 vs round-2 (ranked)

### P0 (user-visible, pehle fix)

| ID | Rank | round-1 observed | round-2 observed | Exact follow-up spec (file/fn) |
|---|---|---|---|---|
| P0-1 | P0 (same) | Session tab bar nahi: ek window me multiple closable sessions nahi, sirf ek active Tab + 9 mode tabs | Koi badlav nahi — is wave me crates touch nahi; parallel wave BU-PLAN (`checks/gui-sessionplan-bu`) owns karti hai | `crates/gui/src/app.rs` (Tab enum, per-session Paths model), `crates/gui/src/views.rs` (tab strip paint fn) |
| P0-2 | P0 (same rank, aadha fix) | Text diff: full-row bands (fixture-text row 3/4/5 me verified) | Live paint me selective landed: sirf changed span par accent (FG/BG + 1px underline), equal plain, filler geometry helpers — `f4fb03f` + BT-DIFF tests green. Headless fixture-text me abhi nahi dikhta (screenshot.rs frozen, paint wiring agle wave ki) | `crates/gui/src/views.rs` (selective_row_is_plain :3499, selective_span_format :3529, filler_hatch_step/count :3573-3598); fixture wiring `crates/gui/src/screenshot.rs` agle wave me |

### P1 (parity, agle round)

| ID | Rank | round-1 observed | round-2 observed | Exact follow-up spec (file/fn) |
|---|---|---|---|---|
| P1-1 | P1 (same) | Path-row meta missing: timestamp, byte size, rules label, flags, history dropdown | Koi badlav nahi — parallel wave BU-PATHMETA (`checks/gui-pathmeta-bu`, OWNS views.rs+app.rs) is waqt chal rahi hai | `crates/gui/src/views.rs` (path row fn), `crates/gui/src/app.rs` (paths state) |
| P1-2 | P1 (same rank, model fix) | Status bar one-sided: BC 3 zones + cursor `1:1`, hamari single counts line | Headless model landed: `StatusZones{left,mid,right}` + label fns + 7 tests (`97cba91`), 629 me included. Live status bar paint abhi bhi single line (views.rs paint spec BT-STATUS REPORT section 5 me hai) | `crates/gui/src/views.rs` (status paint fn, 3-zone adopt), paint-owner wave |
| P1-3 | P1 (same rank, model fix) | Cursor-detail strip missing: BC focused-line preview, hamare paas nahi | Headless model landed: `CursorPos` (1-based, col hamesha 1) + `FocusedLinePreview` (120-char cap) + kind labels (`97cba91`). Live strip paint abhi baaki | `crates/gui/src/views.rs`, `crates/gui/src/viewmodel.rs` (cursor model reuse), paint-owner wave |
| P1-4 | P1 (same rank, contract fix) | Home card CTA wrap: "Open Text / Compare" 2 lines (button budget 120px, need ~164px) | Headless contract landed: `HOME_CARD_CTA_W`=224 (full inner) + `home_card_cta_single_line_ok()` 9/9 true + hit-area helper (`c97b677`). Live paint me abhi bhi 120 floor (paint adoption pending, isliye live-home me wrap abhi bhi hoga) | `crates/gui/src/views.rs` (`paint_home` `add_sized` CTA width adopt), paint-owner wave |

### P2 (polish / harness-only)

| ID | Rank | round-1 observed | round-2 observed | Exact follow-up spec (file/fn) |
|---|---|---|---|---|
| P2-1 | P2 (same) | Toolbar icon parity: BC Rules/Format/Copy icons vs hamare text buttons | Koi badlav nahi | `crates/gui/src/views.rs` (toolbar fn) |
| P2-2 | P2 → FIXED (headless) | Fixture renderer artifact: fixture-text toolbar line me glyph overlap/blocks, live egui me nahi | FIXED headless me: `t` advance 4→5 (`b6a7661`), narrow-set test update, toolbar "ta" jod me shared-pixel zero. Proof: 4/4 fixtures reflowed (section 4), widths +1..+4px BT-SHOTFIX REPORT table me. Live egui me pehle bhi nahi tha, ab bhi nahi | DONE — koi follow-up nahi (live paint ko kabhi effect nahi tha) |

## 6. Kya improve hua, kya baaki (short list)

Improve hua:
- P2-2 glyph overlap fully fixed (headless proof + test).
- P0-2 selective highlight live paint me landed (fixture wiring baaki).
- P1-2/P1-3 status + cursor model landed + tested (paint baaki).
- P1-4 CTA contract landed + tested (paint adoption baaki).
- Tests 611 → 629, zero fail; clippy/fmt BT waves me already green.

Baaki hai:
- P0-1 session tabs (BU-PLAN wave), P1-1 path meta (BU-PATHMETA wave) — dono parallel me, is report me sirf re-rank.
- Paint-adoption backlog: selective filler hatch wiring (screenshot.rs), 3-zone status paint, cursor strip paint, CTA width paint — chaaro ke spec apne BT REPORTs me hain.
- P2-1 toolbar icons untouched.

## 7. Geometry/typography signals (read-only grep, headless-first)

- Grid: `Grid::new` x14 (BT-DIFF ke baad; round-1 me x10 tha — diff filler/table grids jude).
- Typography: monospace x50, strong x26, small x52, `.size(14.0` single value (BT-DIFF round-1 report se same).
- Tests: 629 passed, 0 failed (`round-2/tests.out`).
- Colors: is wave me zero touch (BT batch ka accepted deviation: diff highlight me WHITE/BLACK 7 lines neutral mono — color wave audit karegi).

## 8. Craft mapping + loop suggestion

- Study: BC ko sirf observe kiya (koi click/edit nahi); round-1 notes reuse kiye, BC window ko dobara capture nahi kiya (state same).
- Build: is run me ZERO crates edits — sirf rebuild (mtime force) + recapture.
- Check: build + `--screenshot` fixtures + one-shot screencapture + `file` magic gate + sha delta (GATES.md).
- Loop (agle round ke liye): paint-adoption waves ke baad round-3 me sirf affected scenes recapture karo (text fixture + live Text tab agar observe-only rule khule) -> delta table me P0-2/P1-2/P1-3/P1-4 ko FIXED me move karo -> har round REPORT+GATES Hinglish-Roman me.

## 9. Notes

- Is run me crates/**, scripts/**, tools/**, skills/**, baaki checks dirs ko touch nahi kiya — sirf `checks/gui-localrun-bu2/` (untracked naya dir). Sibling wave dir `checks/gui-pathmeta-bu/` disk par dikhi, usko haath nahi lagaya.
- Cleanup: apna instance (PID 40485) kill kiya; BC (PID 23316) aur sibling processes ko haath nahi lagaya.
- PUSH MANA tha — commit local rahega.

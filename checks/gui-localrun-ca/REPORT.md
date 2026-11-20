# CA-LOCALRUN round-3 — full gap re-verify, bu2 (round-2) se gap DELTA

## 1. Summary

CA wave (MODEL `334dee5` -> APP `c950d03` -> PAINT `ff6895e` -> CALLER `07a8db8`, selective-hatch `42b7d59`, pathmeta `ca79a23`, dark-rootcause `fac1bd5`) ke baad rebuild + recapture kiya. Crates me ZERO content edits — sirf `checks/gui-localrun-ca/` add hua. Natija: P0-1 session chain poori landed (live Home me "no open sessions" empty-state line naya proof), P0-2 filler hatch live paint me wired, P1-1 path meta row live paint me wired, P2 dark root-cause docs me locked. Headless fixtures 4/4 bu2 round-2 se byte-identical — ye EXPECTED hai (screenshot.rs renderer frozen hai, views paint call nahi karta; BZ precedent), no-regression proof hai. Tests 629 (bu2) se 659 hue (+30 CA waves), zero fail. P1-2/P1-3/P1-4 paint-adoption abhi bhi backlog me hain (is round me un par koi wave nahi).

## 2. Live run spec

- Branch: main, HEAD `0229d8a` (CA 7 commits included: `334dee5` model, `ca79a23` pathmeta, `c950d03` app-bridge, `ff6895e` paint, `42b7d59` selwire, `fac1bd5` darkroot-docs, `07a8db8` caller).
- Build: `cargo build -p autocompare-gui` green (0.84s, fresh binary 75MB arm64). Harness `tools/gui-polish/run-geo-typo.sh checks/gui-localrun-ca/round-3/` exit 0, `RESULT: PASS`.
- Launch: direct binary background me (`nohup ... > round-3/cargo-run.log`), PID 54672.
- App window frame (osascript, observe-only): pos 320,91 size 800x632 — bu2 round-2 ke frame jaisa exact.
- Headless fixtures: `--screenshot` filename-keyed scenes (home/text/folder/table), sab 960x600.
- Probe strategy: one-shot capture, action ke baad sleep, koi tight loop nahi. Koi window close/move/click nahi, sirf capture + observe. Tab switch nahi kiya (observe-only, bu2 jaisa), isliye text/folder/table ke live captures nahi — unke liye headless fixtures + wave-level pixel evidence (BZ/BY/CB REPORTs) reuse hai.
- BC app: alag Space/minimized mili (region capture me BC nahi, sirf app+terminal aaye — galat PNG delete kiya, commit nahi hua). BC window title osascript se confirm: `t_left.txt <--> t_right.txt - Text Compare`, still open. BC me koi badlav nahi, isliye BC recapture nahi (bu2 round-2 jaisa). Tulna bu2 notes + live observation se.
- Tests: `cargo test -p autocompare-gui` — 659 passed, 0 failed (bu2 me 629 the; +30 CA waves: session model/app/paint/caller, pathmeta, selwire, darkroot). Log `round-3/tests.out` me.

## 3. Shot map (sab round-3/ me, file magic ok)

| File | Size | Magic | Kya hai |
|---|---|---|---|
| live-desktop.png | 2880x1800 | PNG RGBA | Full side-by-side: Home front, Codex/terminal peeche (bu2 jaisa frame) |
| live-home.png | 1600x1264 (@2x = 800x632) | PNG RGBA | `-R320,91,800,632` region crop, full Home window title-bar samet |
| fixture-home.png | 960x600 | PNG RGBA | Headless Home scene (bu2 se bytes MATCH — renderer frozen) |
| fixture-text.png | 960x600 | PNG RGBA | Headless Text scene (bu2 se bytes MATCH; selective accent rect fixture me pehle se visible tha, unchanged) |
| fixture-folder.png | 960x600 | PNG RGBA | Headless Folder scene (bu2 se bytes MATCH) |
| fixture-table.png | 960x600 | PNG RGBA | Headless Table scene (bu2 se bytes MATCH) |
| after.png | 960x600 | PNG RGBA | Harness output (text scene, fixture-text se byte-identical) |
| cargo-run.log | — | text | Direct binary launch log |
| tests.out | — | text | `cargo test` full log, 659 passed 0 failed |
| geo-typo-report.md | — | text | Harness report, `RESULT: PASS` |

Gate check (har PNG): magic-ok + size>=8192 + distinct>64 — 6/6 PASS (detail GATES.md me).

## 4. Fixture byte delta (bu2 round-2 vs ca round-3, sha12)

| File | bu2 round-2 bytes | ca round-3 bytes | Match |
|---|---|---|---|
| fixture-home.png | 15007 `ac9946b7acdc` | 15007 `ac9946b7acdc` | MATCH (EXPECTED — renderer frozen, no-regression) |
| fixture-text.png | 9765 `bde8ccf481c6` | 9765 `bde8ccf481c6` | MATCH (EXPECTED — same reason) |
| fixture-folder.png | 10149 `f9c6a846ef24` | 10149 `f9c6a846ef24` | MATCH (EXPECTED — same reason) |
| fixture-table.png | 9996 `e1ddba386f22` | 9996 `e1ddba386f22` | MATCH (EXPECTED — same reason) |
| live-home.png | 301202 `4fbbe6892bb5` | 304189 | DIFFER (nayi "no open sessions" line + desktop state; frame same 800x632) |
| live-desktop.png | 762897 `9fe75d40a9a9` | 767963 | DIFFER (desktop state alag — clock/wallpaper/occlusion; app frame same) |

Note: live PNG size farak desktop state + nayi empty-state line ka hai — frame same hai. Fixture MATCH ko fail mat samjho: `screenshot.rs` frozen renderer hai, `views::` paint call nahi karta (BZ REPORT section Before/after geometry me proven: before.png vs after.png identical). Asli fix proof wave-level tests + harness pixel-scan + live observation hain.

## 5. Gap DELTA table — bu2 round-2 vs ca round-3 (ranked)

### P0 (user-visible, pehle fix)

| ID | Rank | bu2 round-2 observed | ca round-3 observed | Verdict + exact evidence |
|---|---|---|---|---|
| P0-1 | P0 (same, ab landed) | Koi badlav nahi: sirf ek active Tab + 9 mode tabs, session strip nahi; parallel wave owns karti thi | 4-commit chain landed: MODEL `SessionList` T1-T7 (`home.rs:596` struct, `:602` impl) -> APP bridge sessions source-of-truth (`app.rs:164-166` field, `:326` `SessionList::new`) -> PAINT strip (`views.rs:697` `paint_session_tab_strip`, `:667` `paint_session_tab`, `:649` empty-gate) -> CALLER switch/close + T5/T6 (`app.rs:5039` strip paint call, `apply_session_action`). Live proof: live-home.png me nayi line "no open sessions: open any compare from Home" (bu2 live-home me nahi thi) = App sessions state live wired. Strip khud empty list par paint nahi hota (by design, `:649`), isliye strip visual abhi live me nahi dikha | PASS (code+tests). Follow-up: mode-tab click session adopt + T7 rename (STATUS me logged) |
| P0-2 | P0 (same, ab wired) | Live paint me selective landed, filler hatch helpers dead the; fixture me nahi dikhta tha | `42b7d59` ne filler hatch wire kiya: `paint_filler_hatch` (`views.rs:3789`, step `:3764`, count `:3771`), 2 call-sites wrap-off filler sides par (`views.rs:3015`, `:3077`), `allow(dead_code)` 0. Selective span format (`:3709`, row-plain `:3679`) pehle se live tha. Fixture MATCH expected (renderer frozen, BZ proof). Live Text tab observe-only rule se capture nahi hua | PASS (code+tests). Follow-up: live Text tab me filler-row hatch ka visual capture (tab-switch rule khule tab) |

### P1 (parity, agle round)

| ID | Rank | bu2 round-2 observed | ca round-3 observed | Verdict + exact evidence |
|---|---|---|---|---|
| P1-1 | P1 (same, ab landed) | Koi badlav nahi — parallel wave chal rahi thi | `ca79a23` landed: `format_path_meta` (`views.rs:375`), `paint_path_meta_row` (`views.rs:392`, history ComboBox), App side history capped/dedupe + compare-start record (`app.rs:603-605`, `:519`), live call-site (`app.rs:5107`). Fixture MATCH expected (frozen renderer). Live Home me path-row scene nahi hota (compare tabs me hota hai), isliye live-home me visual nahi | PASS (code+tests). Follow-up: Text/Folder tab me meta-row ka live visual capture |
| P1-2 | P1 (same, unchanged) | Headless model landed (`StatusZones`, 7 tests), live paint single line | Is round me koi wave nahi — bu2 jaisa hi: model done, 3-zone paint pending | SAME (no-regression, fixtures MATCH). Follow-up: status paint 3-zone adopt (`views.rs` status fn) |
| P1-3 | P1 (same, unchanged) | Headless model landed (`CursorPos`, `FocusedLinePreview`), strip pending | Is round me koi wave nahi — bu2 jaisa hi | SAME. Follow-up: cursor strip paint (`views.rs`, `viewmodel.rs` reuse) |
| P1-4 | P1 (same, unchanged) | Headless contract landed (`HOME_CARD_CTA_W`=224), live paint me 120 floor | Is round me koi wave nahi. Live proof: live-home.png me CTA abhi bhi 2-line wrap ("Open Text / Compare") = paint adoption pending | SAME. Follow-up: `paint_home` CTA width adopt (`views.rs`) |

### P2 (polish / harness-only)

| ID | Rank | bu2 round-2 observed | ca round-3 observed | Verdict + exact evidence |
|---|---|---|---|---|
| P2-1 | P2 (same) | Toolbar icon parity nahi | Koi wave nahi — untouched | SAME. Follow-up: toolbar fn (`views.rs`) |
| P2-2 | P2 FIXED (stable) | Glyph overlap headless me fixed | Abhi bhi fixed — 4/4 fixtures reflowed state me stable (MATCH) | DONE, stable |
| P2-dark | P2 static (naya) | bu2 me track nahi tha (live light vs fixture dark ka zikr nahi) | `fac1bd5` docs-only lock: root-cause = `App.dark_mode=true` (`app.rs:373`) kabhi live ctx par apply nahi hota (`main.rs:52` me `set_visuals` never), paint `ui.visuals().dark_mode` padhta hai. Source ZERO change. Live proof: live-home.png top-right "Light Mode" badge = desync live visible | PASS (docs). Follow-up: CC REPORT ke fix options A/B/C me se ek alag wave me |

## 6. Kya improve hua, kya baaki (short list)

Improve hua:
- P0-1 session chain poori landed (model+bridge+paint+caller) + live empty-state proof.
- P0-2 filler hatch live wired (dead_code 0), selective highlight intact.
- P1-1 path meta row live wired (format + paint + history).
- P2 dark root-cause locked (static docs, 3 fix options).
- Tests 629 -> 659, zero fail; harness RESULT:PASS; fixtures no-regression (4/4 MATCH).

Baaki hai:
- Paint-adoption backlog: P1-2 3-zone status paint, P1-3 cursor strip paint, P1-4 CTA width paint — teeno ke spec apne BT REPORTs me hain.
- P2-1 toolbar icons untouched.
- Live visual captures: Text tab (selective+hatch+filler), compare-tab path meta row, multi-session strip — observe-only tab-switch rule khule tab.
- P0-1 follow-up: mode-tab click session adopt + T7 rename.

## 7. Geometry/typography signals (read-only grep, harness-first)

- Grid: `Grid::new` x14 (bu2 jaisa hi).
- Typography: monospace x50, strong x26, small x52, `.size(14.0` single value (bu2 jaisa hi).
- Colors: harness info `Color32/Visuals` touches 22 (bu2 BT waves ke baad jaisa; is wave me zero touch — `git status` me `crates/**` zero entries).
- Tests: 659 passed, 0 failed (`round-3/tests.out`).

## 8. Craft mapping + loop suggestion

- Study: BC ko sirf observe kiya (window title confirm, koi click/edit nahi); bu2 notes reuse kiye.
- Build: is run me ZERO crates edits — sirf rebuild + recapture.
- Check: build + harness (`RESULT: PASS`) + 4 fixtures + live fullscreen + region crop + `file` magic gate + sha delta (GATES.md).
- Loop (agle round ke liye): paint-adoption waves (P1-2/P1-3/P1-4) ke baad round-4 me sirf affected scenes recapture karo + live Text tab (rule khule to) -> delta table me P1-2/P1-3/P1-4 ko FIXED me move karo -> har round REPORT+GATES Hinglish-Roman me.

## 9. Notes

- Is run me `crates/**`, `scripts/**`, `tools/**`, `skills/**`, baaki checks dirs ko touch nahi kiya — sirf `checks/gui-localrun-ca/` (untracked naya dir).
- Galat region capture (`live-bc.png`, BC window us region me nahi thi) delete kiya, commit me nahi hai. BC window title osascript-observe se confirm hai.
- Cleanup: apna instance (PID 54672) kill kiya; BC (PID 23316) aur sibling processes ko haath nahi lagaya.
- PUSH MANA tha — commit local rahega.

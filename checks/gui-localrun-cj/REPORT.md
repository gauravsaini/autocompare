# CJ-LOCALRUN round-4 — ca round-3 se gap DELTA (dark C + P1/P2 closure re-verify)

## 1. Summary

CA round-3 (HEAD `0229d8a`) ke baad 6 waves land hui: CD status-paint `162069c`
(P1-2 3-zone + P1-3 cursor strip), CE startup-dark `9e7b25b` (option A),
CF CTA-width `aabc07c` (P1-4), CG mode-T7 `799c949`, CH toolbar-contract `68620d1` (P2-1),
CI persist+fixture-light `262eb58` (option C full). Is run me rebuild + recapture kiya,
crates me ZERO content edits — sirf `checks/gui-localrun-cj/` add hua. Natija: fixtures 4/4
ca se byte-identical (default-dark stability hold), tests 669 green, harness `RESULT: PASS`,
P1-4 CTA live me single-line dikha (wrap khatm). Par live app ab bhi LIGHT khula —
CE+CI ke baad bhi dark visible nahi hua, isliye item (1) FAIL hai (evidence neeche).
P1-2/P1-3/pathmeta/selective-hatch/session-strip live Home scene me expected hi nahi the
(compare/Text tabs observe-only rule se khole nahi) — code landed, live-visual pending.

## 2. Live run spec

- Branch main, HEAD `eb3471e` (ca ke 2 docs commits samet 8 commits aage).
- Build: `cargo build -p autocompare-gui` green (0.94s, cached). Harness
  `tools/gui-polish/run-geo-typo.sh checks/gui-localrun-cj/round-4/` exit 0, `RESULT: PASS`.
- Launch: direct binary background me (`nohup ... > round-4/cargo-run.log`, cargo-run.log 0 bytes — GUI stdout silent), PID 62872 (launch waqt eklauta instance).
- App window frame (osascript, observe-only): pos 320,91 size 800x632 — ca round-3 jaisa exact.
- Headless fixtures: `--screenshot` filename-keyed scenes (home/text/folder/table), sab 960x600, exit 0, screenshot*.out khali.
- Probe: one-shot capture, action ke baad sleep, koi loop nahi. Tab switch nahi kiya
  (observe-only, ca jaisa), isliye Text/Folder/Table ke live captures nahi — unke liye
  wave-level evidence (CD/CF/CH/CI REPORTs) reuse hai.
- BC app: alag Space me mili, window title osascript se confirm:
  `t_left.txt <--> t_right.txt - Text Compare`. BC me koi badlav nahi, recapture nahi.
- Tests: `cargo test -p autocompare-gui` — 669 passed, 0 failed (ca me 659 the; +10 waves ke).
- Cleanup: apna instance (PID 62872) kill kiya, confirm gone; sibling processes ko haath nahi lagaya.

## 3. Shot map (sab round-4/ me, `file` magic ok)

| File | Size | Magic/dims | Kya hai |
|---|---|---|---|
| live-desktop.png | 757096 | PNG RGBA 2880x1800 | Full side-by-side: Home front, terminal peeche |
| live-home.png | 295501 | PNG RGBA 1600x1264 (@2x = 800x632) | `-R320,91,800,632` region crop, title-bar samet |
| fixture-home.png | 15007 | PNG RGBA 960x600 | Headless Home (ca se bytes MATCH) |
| fixture-text.png | 9765 | PNG RGBA 960x600 | Headless Text (ca se bytes MATCH) |
| fixture-folder.png | 10149 | PNG RGBA 960x600 | Headless Folder (ca se bytes MATCH) |
| fixture-table.png | 9996 | PNG RGBA 960x600 | Headless Table (ca se bytes MATCH) |
| after.png | 9765 | PNG RGBA 960x600 | Harness output (CE baseline + ca se byte-identical) |
| cargo-run.log | 0 | text (khali) | Direct binary launch log (GUI stdout silent) |
| tests.out | 848 | text | `cargo test` full log, 669 passed 0 failed |
| geo-typo-report.md | 374 | text | Harness report, `RESULT: PASS` |

## 4. Fixture byte delta (ca round-3 vs cj round-4, sha256-12 + `cmp`)

| File | ca round-3 | cj round-4 | Match |
|---|---|---|---|
| fixture-home.png | `ac9946b7acdc` | `ac9946b7acdc` | MATCH (default-dark stable) |
| fixture-text.png | `bde8ccf481c6` | `bde8ccf481c6` | MATCH (default-dark stable) |
| fixture-folder.png | `f9c6a846ef24` | `f9c6a846ef24` | MATCH (default-dark stable) |
| fixture-table.png | `e1ddba386f22` | `e1ddba386f22` | MATCH (default-dark stable) |
| after.png | 9765 B | 9765 B | MATCH ca + MATCH CE baseline |
| live-home.png | 304189 | 295501 | DIFFER (desktop state; dono light — Sec 5 item 1 dekho) |

Fixture MATCH ko fail mat samjho: default path `FixtureMode::Dark` hai (CI test
`fixture_default_path_is_byte_stable_dark` green) — no-regression proof hai.

## 5. Gap DELTA table — ca round-3 vs cj round-4

### Item 1 — live app dark (CE option A + CI persist ke baad)

| Check | ca round-3 observed | cj round-4 observed | Verdict + exact evidence |
|---|---|---|---|
| Live dark | Light khulta tha, top-right "Light Mode" badge (root-cause docs-only `fac1bd5`) | Ab bhi LIGHT khula: live-home.png white bg + top-right "☀ Light Mode" badge | FAIL — badge semantics `views.rs:487-489`: `dark_mode=true` par label "☀ Light Mode" hota hai, matlab flag true par ctx light = desync barkarar. `main.rs:52-66` me CE+CI code maujood hai (storage load + `set_visuals`), par is machine par visible effect nahi. Disk par eframe storage dir nahi mili (storage likely None → default dark expected tha), phir bhi light — isliye persist-restore theory bhi fit nahi hoti. Follow-up: Saved-Application-State persist check + eframe 0.37 theme-override root-cause re-check alag wave me |

### Item 2 — P1/P2 live visibility

| ID | ca round-3 observed | cj round-4 observed | Verdict + exact evidence |
|---|---|---|---|
| P1-4 CTA | Live me 2-line wrap ("Open Text / Compare") | 9 cards ke CTA single-line ("Open Text Compare", "Open Folder Compare"...) — live-home.png me saaf dikhe | PASS (live-visible, `aabc07c`) |
| P0-1 session strip | "no open sessions..." empty-state line nayi thi, strip empty par by-design paint nahi | Wahi line live-home.png me dikhi ("no open sessions: open any compare from Home"); strip ab bhi empty (koi session khola nahi — observe-only) | SAME. CG mode-tab→session adopt (`799c949`) code me landed, click-proof tab-switch rule se pending |
| P1-1 pathmeta | Live wired, Home scene me expected nahi | Waisa hi: path-row compare tabs me hota hai, live-home me expected nahi; code unchanged | SAME (code PASS ca se, live-visual pending) |
| P1-2 status zones | Model-only, paint pending | Paint landed (`162069c`) par status bar Text/Folder tabs me hai — live Home me scene nahi | SAME (live). Code-proof CD REPORT me |
| P1-3 cursor strip | Strip pending | Paint landed (`162069c`) par cursor strip compare tabs me hai — live Home me scene nahi | SAME (live). Code-proof CD REPORT me |
| P0-2 selective hatch/filler | Filler wired, live Text capture nahi hua tha | Waisa hi: Text tab live khola nahi; fixture-text byte-stable | SAME (live). Code-proof BZ/CA me |
| P2-1 toolbar | Untouched | Contract landed (`68620d1`); header row live dikhi (title + note + mode button) par icon-parity ka pixel-verify nahi kiya | SAME/partial (live). Code+headless proof CH REPORT me |

### Item 3 — fixture light-mode band proof + default-dark stability

| Check | Verdict + exact evidence |
|---|---|
| Light-mode band | PASS (test-level) — `screenshot::tests::fixture_mode_param_switches_hues` green: Dark vs Light bytes differ, dono valid PNG (>=8192), guard-restore ke baad default dobara dark-stable. Binary `--screenshot` CLI sirf dark render karta hai, isliye light PNG artifact is run me nahi (CLI me mode flag nahi hai) |
| Persist roundtrip | PASS (test-level) — `dark_mode_storage_parse_trims_and_defaults_dark` + `dark_mode_storage_roundtrip_through_save` green; unknown/khali par declared intent dark |
| Default-dark stability | PASS — 4/4 fixtures ca se byte-identical + after.png CE baseline se identical (Sec 4) |

## 6. Kya improve hua, kya baaki (short list)

Improve hua:
- P1-4 CTA wrap khatm — live photo-proof samet FIXED.
- P1-2/P1-3 paint, P2-1 toolbar, CG mode-T7, CI persist+light-mode — code + tests + harness green (live-visual alag-alag scenes me pending).
- Fixtures 4/4 byte-stable, tests 659 -> 669 zero fail, harness PASS.

Baaki hai (follow-up waves ke liye):
1. Live-dark FAIL — sabse badi gap. Saved-Application-State check + eframe 0.37 startup/follow-theme root-cause re-check; phir Light-Mode OS par live re-capture.
2. Live Text-tab capture (selective hatch + filler + status zones + cursor strip + pathmeta row) — tab-switch rule khule tab; P1-2/P1-3/P1-1/P0-2 ko live me FIXED me move karega.
3. Multi-session strip live visual (session kholna padega — interactive wave).
4. P2-1 toolbar icon-parity live pixel-verify.
5. Light fixture PNG artifact (CLI me `--screenshot-mode light` jaisa flag chahiye to alag wave me).

## 7. Geometry/typography signals (read-only grep, harness-first)

- Grid: `Grid::new` x14, monospace x50, strong x26, small x52, `.size(14.0` single value (ca jaisa exact).
- Width calls 28 (ca jaisa). Colors info-only 27 (ca me 22; farak landed waves ka, is run me zero touch).
- Tests 669 passed, 0 failed (`round-4/tests.out`).

## 8. Notes

- Is run me `crates/**`, `scripts/**`, `tools/**`, baaki checks dirs ko touch nahi kiya — sirf `checks/gui-localrun-cj/` (untracked naya dir). Repo me STATUS.md maujood nahi hai, isliye usme change ka sawal hi nahi.
- BC window ko sirf observe kiya (title confirm), koi click/edit nahi.
- PUSH MANA tha — commit local rahega.

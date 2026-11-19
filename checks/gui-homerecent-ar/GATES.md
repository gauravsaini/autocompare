# GATES — Wave AR-HOMERECENT (Home Recent Polish)

OWNS: crates/gui/src/home.rs + crates/gui/src/app.rs + checks/gui-homerecent-ar/**

Scope: Session restore wave ke baad recent list ka geometry + typography refinement — row rhythm, timestamps, empty/missing states. Geometry first (recent rows/list), typography second (TypoRole reuse, koi naya size nahi), colors frozen. Clean-room: Beyond Compare bahar se observe only, apne shabdon me. MS rust guidelines: non-test code me `unsafe` / `unwrap` / `expect` / `panic` / `todo` mana hai.

## Gate list

- [x] G1: Recent polish model OWNS me hai, graceful states ke saath.
  CHECK: `cargo test -p autocompare-gui homerecent -- --nocapture`
  EXPECT: sab homerecent tests PASS, 0 failed.
  EVIDENCE: 8 passed, 0 failed (home 6 + app 2).

- [x] G2: Full gui suite green hai.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: 0 failed.
  EVIDENCE: 466 passed, 0 failed.

- [x] G3: Clippy zero warnings hai (OWNS scope; repo-level KEVL block note karo).
  CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  EXPECT: exit 0; agar KEVL file block kare to OWNS-filtered proof (`--message-format short` me `home.rs` / `gui/src/app.rs` par zero diagnostic).
  EVIDENCE: exit 0 repo-level green, koi warning nahi (KEVL block is wave me nahi aaya).

- [x] G4: Format sirf apne OWNS files par check hota hai.
  CHECK: `cargo fmt -- --check crates/gui/src/home.rs crates/gui/src/app.rs`
  EXPECT: exit 0. Fix ho to sirf `cargo fmt -- <apne OWNS files>` chalana hai.
  EVIDENCE: PASS (`cargo fmt -- <owns files>` ke baad `--check` clean).

- [x] G5: Banned patterns non-test code me nahi hain.
  CHECK: `rg -n "unwrap\(|expect\(|panic!|todo!|unsafe" crates/gui/src/home.rs crates/gui/src/app.rs`
  EXPECT: sirf `#![forbid(unsafe_code)]` header lines match hon, bahar koi match nahi (test module ke andar allowed).
  EVIDENCE: sirf 2 match — dono `#![forbid(unsafe_code)]` header lines; koi unwrap/expect/panic/todo/unsafe usage nahi (tests me `unwrap_or_default` hai, jo banned list me nahi).

- [x] G6: Geo-typo harness headless PASS deta hai.
  CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-homerecent-ar/round-1`
  EXPECT: `round-1/geo-typo-report.md` me `RESULT: PASS`.
  EVIDENCE: RESULT: PASS (PNG 9624 bytes, magic ok, distinct 256).

## DoD

- Recent list polished hai: do-line row rhythm (title + timestamp/meta), list height/offset helpers, timestamps pure relative labels, empty/missing states honest (koi blank view nahi, koi panic nahi).
- Unit tests naye behaviour ko cover karte hain.
- G2–G6 sab PASS (G3 KEVL-block ho to OWNS-filtered proof + REPORT me note).
- `REPORT.md` + `round-1/` proof ke saath commit hota hai, sirf apne OWNS paths `git add` hote hain, `git stash/push` kabhi nahi.

# GATES — Wave AP-SESSION (Session Restore)

OWNS: crates/gui/src/home.rs + crates/gui/src/app.rs + checks/gui-session-ap/**

Scope: Recent session list se purana session reopen — last state (tab, paths, cursor) restore ho, empty/missing paths graceful ho. Geometry first (recent list rows), typography second, colors frozen. Clean-room: Beyond Compare bahar se observe only, apne shabdon me. MS rust guidelines: non-test code me `unsafe` / `unwrap` / `expect` / `panic` / `todo` mana hai.

## Gate list

- [x] G1: Session restore model + App wiring OWNS me hai, graceful errors ke saath.
  CHECK: `cargo test -p autocompare-gui session -- --nocapture`
  EXPECT: sab session_restore tests PASS, 0 failed.
  EVIDENCE: 15 passed, 0 failed (session filter); naye 7 tests green.

- [x] G2: Full gui suite green hai.
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: 0 failed, 0 ignored-fail.
  EVIDENCE: 440 passed, 0 failed.

- [ ] G3: Clippy zero warnings hai. BLOCKED (dusre agent ki file).
  CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  EXPECT: exit 0, koi warning nahi.
  EVIDENCE: repo-level FAIL: `crates/gui/src/views.rs:26` unused import `gutter_jump_target` (KEVL owner, parallel wave dirty — chhua nahi). OWNS scope clean: message-format short me `home.rs` / `gui/src/app.rs` par zero diagnostic.

- [x] G4: Format sirf apne OWNS files par check hota hai.
  CHECK: `cargo fmt -- --check crates/gui/src/home.rs crates/gui/src/app.rs`
  EXPECT: exit 0 (diff nahi). Fix ho to sirf `cargo fmt -- <apne OWNS files>` chalana hai.
  EVIDENCE: PASS (ek baar diff tha, `cargo fmt -- <owns files>` se fix, phir clean).

- [x] G5: Banned patterns non-test code me nahi hain.
  CHECK: `rg -n "unwrap\(|expect\(|panic!|todo!|unsafe" crates/gui/src/home.rs crates/gui/src/app.rs`
  EXPECT: sirf `#[cfg(test)]` block ke andar match ho, bahar koi match nahi.
  EVIDENCE: sirf 2 match — dono `#![forbid(unsafe_code)]` header lines; koi unwrap/expect/panic/todo/unsafe usage nahi.

- [x] G6: Geo-typo harness headless PASS deta hai.
  CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-session-ap/round-1`
  EXPECT: `round-1/geo-typo-report.md` me `RESULT: PASS`.
  EVIDENCE: RESULT: PASS (PNG 9624 bytes, magic ok, distinct 256).

## DoD

- session restore kaam karta hai (tab + paths + cursor wapas), empty/missing paths par graceful message + model clear, koi panic nahi.
- unit tests naye behaviour ko cover karte hain.
- G2–G6 sab PASS.
- `REPORT.md` + `round-1/` proof ke saath commit hota hai, sirf apne OWNS paths `git add` hote hain, `git push` kabhi nahi.

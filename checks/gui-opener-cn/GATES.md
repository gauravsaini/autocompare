# GATES — Wave CN-OPENER (non-interactive --open flag)

OWNS: `crates/gui/src/main.rs`, `crates/gui/src/app.rs`,
`checks/gui-opener-cn/` only. OFF-LIMITS (untouched):
`views.rs`, `theme.rs`, `home.rs`, `screenshot.rs`, `STATUS.md`.

- [x] G1: Full GUI test suite green (2 naye opener tests: flag parse
  + preload, defaults/rejects/help).
  CHECK: `cargo test -p autocompare-gui`
  EXPECT: `test result: ok`
  EVIDENCE: `test result: ok. 673 passed; 0 failed` (2026-11-20).
  Naye: `app::tests::opener_flag_parses_and_preloads_text_tab`,
  `app::tests::opener_flag_defaults_rejects_and_help` — dono ok.

- [x] G2: Clippy strict clean (indexing-slicing deny samet).
  CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings`
  EXPECT: zero warnings, `Finished`
  EVIDENCE: `Finished dev profile`, zero warnings (2026-11-20).
  Slice-pattern loop + `std::slice::from_ref` se 4 lints fix hue.

- [x] G3: Rustfmt clean on OWNS rs files.
  CHECK: `rustfmt --edition 2024 --check crates/gui/src/main.rs crates/gui/src/app.rs`
  EXPECT: no diff
  EVIDENCE: FMT-CLEAN, no diff (2026-11-20).

- [x] G4: Banned-grep clean (non-test me unwrap/expect/panic/todo zero).
  CHECK: OWNS diff ki `+` lines me
  `unwrap(|expect(|panic!|todo!|unreachable!|Color32|Visuals::` scan
  EXPECT: zero new hits
  EVIDENCE: zero new hits (2026-11-20). Ek `Visuals::dark` hit sirf
  context line hai (pre-existing, `+` nahi).

- [x] G5: Colors zero new + fixture PNGs byte-stable.
  CHECK: `+` lines me `Color32` count; `file` on PNGs; sha256
  before vs after
  EXPECT: 0 new colors; `PNG image data`; identical hashes
  EVIDENCE: 0 new `Color32` (2026-11-20). `file`: dono
  `PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced`.
  sha256 dono `bde8ccf481c6...f61815dc` — identical. Renderer
  (`screenshot.rs`, `views*.rs`) diff me absent; `--screenshot`
  opener code se pehle exit karta hai, isliye opener render nahi
  badal sakta.

- [x] G6: OWNS pathspec only + STATUS-untouched.
  CHECK: `git show --stat HEAD` (commit ke baad); `git status`
  EXPECT: sirf OWNS files; `checks/opencode-waves/STATUS.md` clean
  EVIDENCE: commit `feat(gui): non-interactive --open flag with tab
  preload (CN-OPENER)` — stat: `crates/gui/src/app.rs (+264)`,
  `crates/gui/src/main.rs (+23/-1)`, `checks/gui-opener-cn/`
  (REPORT.md, GATES.md, before.png, after.png). STATUS.md untouched.
  NO PUSH (local commit only). Koi live process nahi chhoda
  (`--help`/`--screenshot` runs exit ho gaye).

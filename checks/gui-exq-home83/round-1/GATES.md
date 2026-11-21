# GATES — exq-home83 round-1: home geometry (leaf)

OWNS: `crates/gui/src/home.rs` ONLY. FROZEN: `theme.rs`, `app.rs`, `views.rs`,
`main.rs`, `copy.rs`, `take.rs`, `gutter.rs`. Colors/typography FROZEN
(zero `Color32`/`Visuals`, zero naya point size — sirf `SPACE_*`/`GEO_*` reuse).

## Gates

- [ ] G1: GATES.md implementation se PEHLE likha gaya
  - CHECK: `test -f checks/gui-exq-home83/round-1/GATES.md && echo GATES-FIRST-OK`
  - EXPECT: `GATES-FIRST-OK`
- [ ] G2: baseline `cargo test -p autocompare-gui` green tha
  - CHECK: `cargo test -p autocompare-gui 2>&1 | grep -E "test result: (ok|FAILED)" | sort | uniq -c`
  - EXPECT: `ok`
- [ ] G3: sirf OWNS file badla, frozen files untouched
  - CHECK: `git status --porcelain | grep -v "^??" | awk '{print $2}'`
  - EXPECT: `crates/gui/src/home.rs`
- [ ] G4: `cargo test -p autocompare-gui` green (HOME83 test samet)
  - CHECK: `cargo test -p autocompare-gui home83 2>&1 | grep -E "test result:"`
  - EXPECT: `ok`
- [ ] G5: fmt clean
  - CHECK: `cargo fmt -p autocompare-gui -- --check 2>&1 && echo FMT-OK`
  - EXPECT: `FMT-OK`
- [ ] G6: banned 0 non-test (sirf `#[cfg(test)]` se PEHLE wala code gina jata hai;
    `//` comment prose ("zero Color32/Visuals" docs) bahar rakha gaya)
  - CHECK: `sed '/#\[cfg(test)\]/,$d' crates/gui/src/home.rs | sed 's|//.*||' | grep -qE "unwrap|expect\(|panic!|todo!|unsafe[^_ ]|Color32|Visuals::" && echo BANNED-FOUND || echo BANNED-0`
  - EXPECT: `BANNED-0`
- [ ] G7: commit bana, push nahi hua
  - CHECK: `git log --oneline -1 && git status --porcelain | head -5`
  - EXPECT: `feat(gui): exq-home83 home geometry`

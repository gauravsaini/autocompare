# Wave Y-FONT — GATES (write-first, Gap #9 font zoom Cmd+/-)

Scope: `crates/gui/src/theme.rs` + `crates/gui/src/app.rs` + `checks/gui-font-y/**` ONLY.
`views.rs` / `viewmodel.rs` / `screenshot.rs` / `tree.rs` kabhi mat chhoona (sibling Y-SPLIT ka OWNS).
Geometry first, typography second, colors FROZEN. No new deps. Clean-room: bahar se observe only.

## G1 — Baseline green (koi edit se pehle)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `271 passed; 0 failed`

## G2 — Code lands, paint-neutral, frozen tokens untouched

- CHECK: `git status --short` me sirf OWNS paths
  (`crates/gui/src/theme.rs`, `crates/gui/src/app.rs`, `checks/gui-font-y/**`);
  `git diff -- crates/gui/src/theme.rs crates/gui/src/app.rs | grep -E "^-" | grep -vE "^---" | grep -E "SPACE_|GEO_|GUTTER_|Color32::from_rgb|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_" || echo NO-FROZEN-DIFF`
- EXPECT: sirf `+` lines (naya factor helper + key resolver + tests + doc lines);
  geometry consts / color tokens me zero `-` lines; `rich()` / `rich_zoomed()` paint path unchanged;
  `views.rs` / `viewmodel.rs` / `screenshot.rs` / `tree.rs` me zero diff.

## G3 — Zoom in/out/reset kaam karta hai + typo hierarchy barkarar

- CHECK: `cargo test -p autocompare-gui zoom 2>&1 | tail -n 3`
  + `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: sab green, including naye tests
  `zoom_scale_stays_in_factor_band_and_keeps_hierarchy` (har role x har ZOOM_LEVELS:
  scale factor 0.8..=2.0 ke andar, Small < Mono < Body < HomeTitle < Heading order barkarar,
  level 0 par factor 1.0)
  + `font_zoom_keys_resolve_and_apply` (In/Out/Reset/None resolve, reset-wins,
  apply se level badalta hai, None no-op).

## G4 — MS Rust guidelines + headless-first clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- --check; echo fmt:$?`
  + non-test slice par banned grep (dono files):
  `for f in crates/gui/src/theme.rs crates/gui/src/app.rs; do sed '/#\[cfg(test)\]/,$d' $f | grep -nE "unwrap\(|expect\(|panic!|todo!|unreachable!|unimplemented!" || echo "$f:ZERO-BANNED"; done`
  + `grep -n "forbid(unsafe_code)" crates/gui/src/theme.rs crates/gui/src/app.rs`
- EXPECT: `clippy:0`, `fmt:0`, dono files ZERO-BANNED (non-test), `forbid(unsafe_code)` dono me.

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-font-y/round-1; echo harness:$?`
  + `zsh checks/gui-font-y/run-font-y.sh checks/gui-font-y/round-1; echo fonty:$?`
  + `python3 -c "d=open('checks/gui-font-y/round-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
- EXPECT: dono harness `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`, size >= 8192, distinct > 64.

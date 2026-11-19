# Wave BI-HEXTEXT — GATES (one observable outcome per gate)

OWNS: `crates/gui/src/views_hex.rs` (grid-text scope only) + `checks/gui-hextext-bi/**` ONLY.
`views.rs` callsite FROZEN (zero edit), `app.rs` snap FROZEN, `screenshot.rs` READ-ONLY
(paint logic nahi), `tree.rs` / `theme.rs` / `views_table.rs` READ-ONLY,
`checks/opencode-waves/STATUS.md` untouched (orchestrator-owned). Colors FROZEN.

1. Real-digit grid cells — covered bytes par real hex digits, `??` nahi.
   CHECK: `cargo test -p autocompare-gui hex_grid_cells_with_bytes_show_real_digits` EXPECT: pass
   (covered row par `00`/`0f` lowercase digits, `??` zero, printable ASCII seedha,
   non-printable par `.`, join shape Grid jaisa).

2. Fallback + shape parity — insertion hunks aur blank rows honest, chaudai stable.
   CHECK: `cargo test -p autocompare-gui hex_grid_cells_with_bytes_fallback_and_shape_parity` EXPECT: pass
   (left-missing par right fallback, `hex.len()` purane shape se barabar, uncovered/past-end
   row blank, `hex_grid_ascii_mark` printable/control contract).

3. Full suite green — koi regression nahi.
   CHECK: `cargo test` (full workspace) EXPECT: exit 0, koi FAILED suite nahi
   (gui crate 591 passed: 589 purane + 2 naye).

4. Hygiene — MS guidelines + frozen colors + OWNS discipline.
   CHECK: `git diff HEAD -- crates/gui/src/views_hex.rs | grep -c '^-[^-]'` EXPECT: 0
   (sirf insertions, koi existing line rewrite nahi);
   banned `grep -nE 'unwrap\(|\.expect\(|panic!|todo!|unreachable!'` apne diff me EXPECT: zero;
   `Color32|Visuals` apne diff me EXPECT: zero (comment me bhi nahi);
   `cargo clippy -p autocompare-gui --all-targets` me views_hex hits EXPECT: zero;
   `rustfmt --edition 2024 --check crates/gui/src/views_hex.rs` EXPECT: clean
   (bare `cargo fmt` MANA, chalaya nahi);
   `git status --short` me apni files ke bahar EXPECT: kuch nahi
   (is wave ne views.rs/app.rs/screenshot.rs ko chhua nahi).

5. Harness proof — headless-first round passes with real PNGs.
   CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-hextext-bi/round-1` EXPECT: `RESULT: PASS`,
   `after.png` + `after-hex.png` magic-ok (`89504e470d0a1a0a`), size >= 8192 B, distinct > 64.

6. Callsite spec — agle paint wave ke liye exact wiring REPORT me, is wave me koi paint edit nahi.
   CHECK: `git diff HEAD --stat -- crates/gui/src/views.rs crates/gui/src/app.rs crates/gui/src/screenshot.rs`
   is wave ke commit me EXPECT: absent (REPORT §5 me fn + args + insert lines documented).

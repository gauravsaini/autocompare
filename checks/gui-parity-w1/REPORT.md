# GUI parity wave W1 — REPORT (Hinglish-Roman)

## Kya kiya (2 rounds)
- Round-1: Folder drill-down geometry — indent 14 -> 12 canonical,
  fixed 16px expander cell, file/dir row alignment, fixture indent fix.
- Round-2: Home geometry — flex cards (306 @960, floor 240), CTA
  150 -> 224 centered, fixture-live mirror test.

## Geometry table (pehle -> ab)
| Jagah | Pehle | Ab | Source |
|---|---|---|---|
| Tree indent / depth | 14px (paint-only) | 12px (`GEO_TREE_INDENT_STEP`) | round-1 |
| Expander cell | variable / kuch nahi | fixed 16px + `folder_row_x_offset` | round-1 |
| Folder fixture indent | 28px / depth | 14px / depth (live 12 nearest) | round-1 |
| Home card width (live) | fixed 240 | flex `(avail-16)/3`, floor 240 | round-2 |
| Home CTA (fixture) | 150x28 left-aligned | 224x28 centered (live parity) | round-2 |
| Home card h | 100 (ceiling 99.6) | same (lock test se) | round-2 |

## Verify
- `CARGO_BUILD_JOBS=2 cargo test -p autocompare-gui`: 699 -> 703 green.
- `cargo clippy -p autocompare-gui --all-targets`: zero warnings.
- `cargo fmt -p autocompare-gui --check`: clean.
- Screenshots (960x600, PNG magic ok): home 14991 B, folder 10153 B,
  text 9765 B (shape regression nahi).

## Rules compliance
- Colors frozen: koi Color32/Visuals/theme color value change nahi
  (sirf geometry consts + layout math + fixture pixels).
- MS rust: non-test code me unwrap/expect/panic/todo/unreachable/index
  nahi — naya code `saturating_*`, `clamp`, `max`, `is_finite` guards
  par hai. (`read_hex_input_capped` me pehle se `unwrap_or_default`
  hai — allowed, unwrap nahi.)
- Scope: sirf `crates/gui/**` + `checks/gui-parity-w1/**` change.
  Dusre paths sirf read kiye (capture/docs study notes).

## Bache hue gaps (agli wave)
1. Dual-pane folder me expander beech-column me — fixed cell wahan nahi.
2. Small-button hit (~20-24px) vs 16px cell residual ~4-8px.
3. 720px window par 3-col Home overflow — 2-col fallback nahi.
4. BC gutter ~40px andaza UNVERIFIED — hamara 16px barkarar.
5. Canvas 960x600 vs BC 720x869 aspect gap — canvas change nahi kiya.

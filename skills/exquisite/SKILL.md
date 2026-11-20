---
name: exquisite
description: Exquisite UI loop for autocompare-gui (geometry first, typography second, colors frozen). Use when closing Beyond Compare visual parity gaps.
---

# Exquisite (geo first, typo second)

Harness: `tools/exquisite/run.sh checks/<round>/`

`RESULT: PASS` means tests green + PNG valid (magic, >=8192B, non-blank).
Report also lists geometry signals (SPACE tokens, margin/gap consts, Grid count,
row-height uses, width calls, gutter uses) and typography signals
(TypoRole uses, monospace/strong counts, .size values, weights, line-heights,
letter-spacing) for px-measured review. Colors frozen: do not touch
Color32 palette, Visuals, heatmap colors — report counts them info-only.

## Loop (fix-and-repeat, max 6 rounds)

1. Run harness into fresh `checks/<round>/` dir.
2. Green: move on. Red: read failing `.out`, fix smallest root-cause diff in owning files.
3. Geometry FIRST: width, height, margins, padding, alignment, positioning. No color edits.
   Reuse `theme::SPACE_*`, `OUTER_MARGIN`/`SECTION_GAP` family, `TypoRole` sizes.
4. Typography SECOND: family, weight, line-height, letter-spacing. No color edits.
   Reuse existing `TYPO_*` tokens, `TypoRole::Mono/Body/Small`, no new sizes.
5. Repeat same round dir on re-run, advance N only for new attempt series.
6. Add one edge fixture per wave under `checks/<round>/comparison/` (e.g. narrow
   <720px wrap, empty-vs-nonempty, tail-insertion) with a headless test proving it.
7. Escalate if 2 rounds change no files or 10 checks fail: write `checks/inbox/<issue>.md`.

## Rules

- Hinglish Roman only in chat, commit messages English-only conventional commits.
- Clean-room outside observation: describe BC behavior in own words, never copy strings/assets.
- MS rust guidelines: forbid unsafe, no unwrap/expect/panic/todo in non-test code, headless-first.
- Disjoint OWNS per agent, conventional commit NO push (orchestrator verifies then pushes).
- `uv` for python helpers when python needed; stdlib first, no new deps.
- `$unlazy + $ponytail`: minimal diff, reuse consts, no placeholders.

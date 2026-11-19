---
name: gui-polish
description: Exquisite UI loop for autocompare-gui (geometry first, typography second, colors out of scope). Use when polishing layout, spacing, alignment, fonts, line-height.
---

# GUI Polish (geo first, typo second)

Loop harness: `tools/gui-polish/run-geo-typo.sh checks/<round>/`

`RESULT: PASS` means tests green + PNG valid (magic, >=8192B, non-blank).
Report also lists geometry signals (add_space values, Grid count, text_style_height uses)
and typography signals (monospace/strong/small counts, .size values) for px-measured review.

## Loop (fix-and-repeat, max 6 rounds)

1. Run harness into fresh `checks/<round>/` dir.
2. Green: move on. Red: read failing `.out`, fix smallest root-cause diff in owning files.
3. Geometry first: widths, heights, margins, padding, alignment, positioning. No color edits.
4. Typography second: family, weight, line-height, letter-spacing. No color edits.
5. Repeat same round dir on re-run, advance N only for new attempt series.
6. Escalate if 2 rounds change no files or 10 checks fail: write `checks/inbox/<issue>.md`.

## Rules

- Hinglish Roman only in chat, commit messages English-only conventional commits.
- Clean-room outside observation: describe BC behavior in own words, never copy strings/assets.
- MS rust guidelines: forbid unsafe, no unwrap/expect/panic/todo in non-test code, headless-first.
- Disjoint OWNS per agent, conventional commit NO push (orchestrator verifies then pushes).
- Colors explicitly out of scope: do not touch Color32 palette, Visuals, heatmap colors.

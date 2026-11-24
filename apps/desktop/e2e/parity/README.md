# Living parity specs (one per view, geometry only)

Rule: functional specs in `e2e/specs/` test BEHAVIOR only (filters, copy, nav,
wiring). They must NOT pin exact pixels (`offsetHeight === 22`, `=== 28`,
`gridTemplateColumns === ...`). All rendered-geometry + vision locks live here,
one file per view:

- `text.parity.spec.ts` — gutter grid, arrows, bracket, hatch, splitter, density, chrome budget
- `folder.parity.spec.ts` — density 17, orphan side, single toolbar, chrome budget
- `table.parity.spec.ts`, `hex.parity.spec.ts`, `sync.parity.spec.ts`,
  `media.parity.spec.ts`, `merge.parity.spec.ts` — scaffolded next, same pattern

Wave-numbered specs (`textgutter-66q`, `text-wrap`, `density-88`, ...) stay
green but are advisory for pixels; on conflict the living spec wins. UI loop
(`parity:snap` + Vision-Critic) and functional-gap loop (GATES session
round-trip, PathBar submit, copy persist) run separately.

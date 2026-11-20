# VTABLE7 REPORT — exq-vtable7 clippy plus align

Round: `checks/gui-exq-vtable7/round-1` — RESULT: PASS (1 round, max 6 me se).

## Kya kiya

1. **Clippy `useless_vec` fix (3110 + same-pattern 3076):** single-element
   `vec![vec![...]]` ko `core::iter::once(...).collect()` se banaya.
   Behavior same hai, sirf lint clean — `-D warnings` ab 0 hai.
2. **Column alignment geometry:** `paint_table_cell_grid` me header row aur
   data rows ab ek hi `Grid` (`TABLE_CELL_GRID_ID`) me paint hote hain.
   Pehle header apne alag Grid me tha, isliye header widths data columns se
   alag compute hote the — ab ek hi Grid widths share karta hai, columns
   align rehte hain (BC table parity).
3. **Typography/colors:** koi naya size nahi (Mono + strong reuse), koi
   Color32/Visuals touch nahi — colors frozen rakha.

## Evidence

- `round-1/after.png` (10401 bytes, PNG magic ok, non-blank)
- `round-1/exquisite-report.md` — RESULT: PASS
- `round-1/tests.out` — 841 passed, 0 failed
- `GATES.md` — gate proof

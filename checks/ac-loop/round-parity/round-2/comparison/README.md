# Round 2 Comparison Edge Fixtures: Grammar Whitespace Ignore

This directory contains edge fixtures validating grammar and whitespace ignore options against Beyond Compare parity:
1. `grammar_left.txt` vs `grammar_right.txt`:
   - Whitespace run differences inside function signature (`x: i32,  y: i32` vs `x: i32, y: i32`).
   - Strict comparison reports difference (exit code 1).
   - Space-change ignore (`-b`) collapses whitespace runs and treats them as identical (exit code 0).
2. `verify.sh`:
   - Automated script executing both strict and `-b` text comparison, verifying exit codes 1 and 0 respectively.

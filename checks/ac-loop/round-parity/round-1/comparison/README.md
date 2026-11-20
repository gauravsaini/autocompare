# Round 1 Comparison Edge Fixtures: 3-Way Text Merge

This directory contains edge fixtures validating 3-way text merge capabilities against Beyond Compare parity:
1. `clean_base.txt`, `clean_left.txt`, `clean_right.txt` -> `clean_expected.txt`:
   - Non-overlapping edits from left and right applied cleanly with context preservation.
   - Clean merge succeeds with exit code 0.
2. `merge3_base.txt`, `merge3_left.txt`, `merge3_right.txt` -> `expected_conflict.txt`:
   - Overlapping base range modifications produce standard conflict blocks with markers `<<<<<<< left`, `=======`, and `>>>>>>> right`.
   - Preserves non-conflicting edits and common context lines outside the conflict hunk.
   - Accurately reports conflicts with exit code 1.
3. `verify.sh`:
   - Automated verification script checking both clean and conflicting merges against expected outputs.

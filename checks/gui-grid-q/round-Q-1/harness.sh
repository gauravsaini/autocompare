#!/bin/bash
# Wave Q round-Q-1 harness: grid paint gates for views.rs only.
# Usage: bash checks/gui-grid-q/round-Q-1/harness.sh
# Writes tests.out, clippy.out, fmt.out, grid-report.md into its own dir.
set -u
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
ROUND="$ROOT/checks/gui-grid-q/round-Q-1"
VIEWS="$ROOT/crates/gui/src/views.rs"
cd "$ROOT" || exit 2

cargo test -p autocompare-gui > "$ROUND/tests.out" 2>&1
TESTS_OK=$?
cargo clippy -p autocompare-gui --all-targets -- -D warnings > "$ROUND/clippy.out" 2>&1
CLIPPY_OK=$?
cargo fmt -p autocompare-gui -- --check > "$ROUND/fmt.out" 2>&1
FMT_OK=$?

PASS_COUNT=$(grep -oE "[0-9]+ passed" "$ROUND/tests.out" | tail -1)
{
  echo "# Grid report (round-Q-1)"
  echo ""
  echo "- tests: ${PASS_COUNT:-?} ($([ $TESTS_OK -eq 0 ] && echo ok || echo FAIL))"
  echo "- clippy -D warnings: $([ $CLIPPY_OK -eq 0 ] && echo ok || echo FAIL)"
  echo "- fmt --check: $([ $FMT_OK -eq 0 ] && echo ok || echo FAIL)"
  for item in "format_hex_grid_row" "hex_grid_rows_for_hunk" "hunk_covers_offset" \
      "hex_minimap_row_at_y" "hex_minimap_jump_to_row" "click_and_drag" \
      "split_table_cells" "table_header_cells" "format_table_bordered_row" \
      "format_table_header_rule" "table_column_widths" "bring_into_view"; do
    if grep -q "$item" "$VIEWS"; then echo "- views.rs has $item: ok"; else echo "- views.rs has $item: FAIL"; fi
  done
  HEX_HOVER=$(grep -c "Sense::hover" "$VIEWS")
  echo "- Sense::hover count (text minimap only, untouched): $HEX_HOVER"
  NORGB=$(grep -c "Color32::from_rgb(" "$VIEWS" || true)
  echo "- Color32::from_rgb( additions: $NORGB (want 0)"
  NCONST=$(grep -c "pub const" "$VIEWS")
  echo "- pub const count (want 8, no new geometry tokens): $NCONST"
  if [ $TESTS_OK -eq 0 ] && [ $CLIPPY_OK -eq 0 ] && [ $FMT_OK -eq 0 ] \
    && [ "$NORGB" -eq 0 ] && [ "$NCONST" -eq 8 ]; then
    echo "RESULT: PASS"
  else
    echo "RESULT: FAIL"
  fi
} > "$ROUND/grid-report.md"
grep -q 'RESULT: PASS' "$ROUND/grid-report.md"

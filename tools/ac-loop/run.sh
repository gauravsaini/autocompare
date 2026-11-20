#!/bin/zsh
# ac-loop one-round harness: CLI parity + GUI geo-typo + live BC observe (read-only) -> REPORT.md
# Usage: tools/ac-loop/run.sh <round-dir>
set -u
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ROUND="${1:?usage: run.sh <round-dir>}"
mkdir -p "$ROUND/comparison"
CLI_OUT=0; GUI_OUT=0
tools/parity-harness/run-round.sh "$ROUND/cli" > "$ROUND/cli.log" 2>&1 || CLI_OUT=$?
tools/gui-polish/run-geo-typo.sh "$ROUND/gui" > "$ROUND/gui.log" 2>&1 || GUI_OUT=$?
# Live Beyond Compare observe (read-only, no copy, no focus steal): session menu via osascript
osascript -e 'tell application "System Events" to get name of every process' 2>/dev/null | tr ',' '\n' | grep -i -E "beyond|compare|bcomp" > "$ROUND/comparison/bc-process.txt" 2>&1 || true
"$ROOT/target/debug/autocompare" --help > "$ROUND/comparison/own-help.txt" 2>&1 || true
# Comparison delta: own 9 sessions vs BC observed session types (from docs/study.md baseline)
cat > "$ROUND/REPORT.md" <<EOF2
# ac-loop round report ($(basename $ROUND))
- CLI parity: $([ $CLI_OUT = 0 ] && echo PASS || echo FAIL) (log cli.log, report cli/parity-report.md)
- GUI geo-typo: $([ $GUI_OUT = 0 ] && echo PASS || echo FAIL) (log gui.log, report gui/geo-typo-report.md)
- BC live observe: $(cat $ROUND/comparison/bc-process.txt 2>/dev/null | head -n 5)
- own sessions: text folder hex merge table sync picture folder-merge audio (own-help.txt)
- comparison dir: add new edge fixtures here each round, rerun same dir on fix, advance N only on PASS series
$([ $CLI_OUT = 0 ] && [ $GUI_OUT = 0 ] && echo "RESULT: PASS" || echo "RESULT: FAIL")
EOF2
cat "$ROUND/REPORT.md"
[ $CLI_OUT = 0 ] && [ $GUI_OUT = 0 ]

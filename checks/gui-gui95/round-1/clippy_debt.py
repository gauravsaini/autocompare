"""G7 oracle: clippy debt must not grow beyond the base snapshot.

Runs `cargo clippy -p autocompare-gui --lib --bins` (production code
only, default flags) and counts warning/error lines. Baseline measured
2026-11-21 at clean base 11aabc9 with zero leaf code changes:
14 warnings, 0 errors (pre-existing: doc-indent x7, too-many-args x3,
unwrap_or_default x2, is_multiple_of x1, empty if-branch x1).
Fails if errors appear or warnings exceed the baseline.
Prints GUI_CLIPPY_DEBT_OK with the measured counts on success.
"""
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BASE_WARNINGS = 14

out = subprocess.run(
    ["cargo", "clippy", "-p", "autocompare-gui", "--lib", "--bins"],
    cwd=ROOT,
    capture_output=True,
    text=True,
).stderr

warnings = len([l for l in re.findall(r"^warning: .*", out, re.M) if "generated" not in l])
errors = len(re.findall(r"^error", out, re.M))

if errors > 0 or warnings > BASE_WARNINGS:
    print(f"CLIPPY DEBT GREW: warnings={warnings} (base {BASE_WARNINGS}) errors={errors}")
    raise SystemExit(1)
print(f"GUI_CLIPPY_DEBT_OK warnings={warnings} errors={errors}")

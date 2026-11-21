"""Fail-closed banned-construct scan for crates/cli/src prod code."""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[3] / "crates" / "cli" / "src"
BANNED = re.compile(
    r"(?<![A-Za-z0-9_])unsafe(?![A-Za-z0-9_])"
    r"|\.unwrap\s*\("
    r"|\.expect\s*\("
    r"|\bpanic!\s*\("
    r"|\btodo!\s*\("
    r"|\bunimplemented!\s*\("
    r"|\bunreachable!\s*\("
)

hits = []
for path in sorted(ROOT.glob("*.rs")):
    skip = False
    for i, line in enumerate(path.read_text().splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("#[cfg(test)]"):
            skip = True
            continue
        if skip:
            continue
        if BANNED.search(line):
            hits.append(f"{path.name}:{i}:{stripped}")

if hits:
    print("\n".join(hits))
    sys.exit(1)
print("BANNED_ZERO")

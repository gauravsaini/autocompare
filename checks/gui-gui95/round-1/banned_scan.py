"""G5 oracle: zero banned constructs in non-test GUI code.

Scans crates/gui/src/*.rs for unwrap()/expect(/panic!/todo!/
unimplemented! outside #[cfg(test)] mods. Test-only hits are allowed
(clippy deny-lints cover non-test code; this is the independent check).
Prints GUI_BANNED_OK only when no non-test hit exists.
"""
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SRC = ROOT / "crates" / "gui" / "src"
BANNED = re.compile(r"\bunwrap\(\)|\bexpect\(|panic!|todo!|unimplemented!")

bad = []
for path in sorted(SRC.glob("*.rs")):
    lines = path.read_text().splitlines()
    # Mark test-mod regions: #[cfg(test)] then mod <name> { ... } with brace depth.
    test_depths = []  # stack of (brace_depth_at_open)
    depth = 0
    pending_test = False
    in_test = 0  # nesting count of test mods
    # Track depth per line to decide membership.
    for i, line in enumerate(lines, 1):
        if "#[cfg(test)]" in line:
            pending_test = True
        stripped = line.strip()
        if pending_test and re.match(r"(pub\s+)?mod\s+\w+", stripped):
            # test mod opens here; count braces on this line
            opens = line.count("{") - line.count("}")
            test_depths.append((depth + opens, opens > 0 or "{" in "".join(lines[i:i+2])))
            pending_test = False
            in_test += 1
            depth += line.count("{") - line.count("}")
            continue
        if pending_test and "{" not in line:
            # attribute separated from mod by comments? keep pending
            pass
        elif pending_test and "{" in line:
            pending_test = False
        m = BANNED.search(line)
        if m and in_test == 0:
            bad.append(f"{path.name}:{i}: {line.strip()[:120]}")
        new_depth = depth + line.count("{") - line.count("}")
        # close test mods whose region ended
        while test_depths and new_depth < test_depths[-1][0]:
            test_depths.pop()
            in_test = max(0, in_test - 1)
        depth = new_depth

if bad:
    print("BANNED HITS (non-test):")
    for hit in bad:
        print(hit)
    raise SystemExit(1)
print("GUI_BANNED_OK")

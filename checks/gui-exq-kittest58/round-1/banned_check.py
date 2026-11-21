#!/usr/bin/env python3
"""Banned-token + OWNS-disjoint check (exq-kittest58 round-1, G4).

Follows the repo GATES.md G6 convention (rg-style real-`unsafe` scan that
ignores the `#![forbid(unsafe_code)]` attribute and `//` comments):

B1. crates/gui/src: zero real `unsafe` blocks/fns outside the forbid attr.
B2. Owned test file crates/gui/tests/real_egui_shot.rs: zero
    unsafe/unwrap(/expect(/panic!/todo! (Result<String> style throughout;
    stricter than the "expect only inside tests" allowance).
B3. `git status --short` shows ONLY owned paths
    (crates/gui/tests/real_egui_shot.rs + checks/gui-exq-kittest58/**),
    which also proves zero new banned tokens in non-test code: no src
    file is touched at all (pre-existing #[cfg(test)] hits elsewhere are
    out of scope and unverified by design, never modified here).

Prints OWNS-CLEAN iff all three hold.
"""

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent.parent
OWNED = ("crates/gui/tests/real_egui_shot.rs", "checks/gui-exq-kittest58/")
UNSAFE_RE = re.compile(r"unsafe\s*\{|fn\s+unsafe|unsafe\s+fn|unsafe\s*\(")
TEST_BANNED = ("unsafe", "unwrap(", "expect(", "panic!", "todo!")


def main():
    problems = []

    for path in sorted((ROOT / "crates/gui/src").rglob("*.rs")):
        for no, line in enumerate(path.read_text().splitlines(), 1):
            code = line.split("//", 1)[0]
            if "forbid(unsafe" in code:
                continue
            if UNSAFE_RE.search(code):
                problems.append(f"{path.relative_to(ROOT)}:{no}: {code.strip()}")

    test_src = (ROOT / "crates/gui/tests/real_egui_shot.rs").read_text()
    test_lines = [
        f"{no}: {line.strip()}"
        for no, line in enumerate(test_src.splitlines(), 1)
        if "forbid(unsafe_code)" not in line
        and any(tok in line.split("//", 1)[0] for tok in TEST_BANNED)
    ]
    if test_lines:
        problems.append("test-file banned: " + "; ".join(test_lines))

    out = subprocess.run(
        ["git", "status", "--short"], capture_output=True, text=True, cwd=ROOT
    ).stdout.splitlines()
    strays = [line for line in out if not any(line[3:].startswith(o) for o in OWNED)]
    if strays:
        problems.append("strays: " + "; ".join(s.strip() for s in strays))

    if problems:
        print("DIRTY " + " | ".join(problems))
        print("OWNS-DIRTY")
        return 1
    print("OWNS-CLEAN")
    return 0


if __name__ == "__main__":
    sys.exit(main())

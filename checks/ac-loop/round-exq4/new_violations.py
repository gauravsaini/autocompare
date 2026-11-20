#!/usr/bin/env python3
"""G5: zero NEW unwrap/expect/panic/todo/unimplemented/unreachable/indexing
in non-test gui code vs HEAD baseline. Added lines only (diff-scoped);
attribute/doc-comment lines excluded; baseline legacy defs[i] out of scope."""
import re
import subprocess
import sys

diff = subprocess.run(
    ["git", "diff", "HEAD", "--", "crates/gui/src"],
    capture_output=True, text=True, check=True,
).stdout

code_line = re.compile(r"^\+(?!\+)")
skip_line = re.compile(r"^\+\s*(#\[|///|//)")
bad = re.compile(
    r"unwrap\(\)|\.expect\(|panic!|todo!|unimplemented!|unreachable!|\w\[[^\[\]]+\]"
)
hits = [ln for ln in diff.splitlines()
        if code_line.match(ln) and not skip_line.match(ln) and bad.search(ln)]
for ln in hits:
    print("NEW VIOLATION:", ln)
print(f"NEW VIOLATIONS {len(hits)}")
sys.exit(0 if not hits else 1)

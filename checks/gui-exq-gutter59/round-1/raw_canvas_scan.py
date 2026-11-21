"""G2 oracle: raw canvas literals at non-test call sites in gutter.rs.

Strips top-level #[cfg(test)] modules, doc/comment lines, and `pub const`
single-source definitions (the tokens themselves). Any remaining
960.0/720.0/1200.0/700.0 literal is a call site that must adopt the token.
Prints RAW_CALLS=<n> plus offending lines.
"""

import re
import pathlib

lines = pathlib.Path("crates/gui/src/gutter.rs").read_text().split("\n")
out = []
skip = False
depth = 0
for ln in lines:
    if not skip and ln.strip() == "#[cfg(test)]":
        skip = True
        depth = 0
        continue
    if skip:
        depth += ln.count("{") - ln.count("}")
        if ln == "}" and depth <= 0:
            skip = False
        continue
    out.append(ln)

hits = []
for ln in out:
    s = ln.strip()
    if s.startswith("///") or s.startswith("//"):
        continue
    if s.startswith("pub const "):
        continue
    if re.search(r"\b(?:960\.0|720\.0|1200\.0|700\.0)\b", ln):
        hits.append(ln.strip()[:100])

print("RAW_CALLS=" + str(len(hits)))
for h in hits:
    print(h)

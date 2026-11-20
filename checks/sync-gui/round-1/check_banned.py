"""Banned-token gate for SYNCGUI OWNS non-test code.

Strips line/block comments, string literals and `mod *test* { ... }`
regions, then asserts no `unwrap(`/`expect(`/`panic!`/`todo!`/
`unimplemented!`/bare `unsafe` remains. `unsafe_code` (forbid guard)
and `"unsafe path"` literals are removed with strings/comments, so
they never count.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
FILES = [
    ROOT / "crates/compare/src/sync.rs",
    ROOT / "crates/compare/src/lib.rs",
    ROOT / "crates/gui/src/tree.rs",
]

TOKEN = re.compile(r"unwrap\s*\(|expect\s*\(|panic!|todo!|unimplemented!|\bunsafe\b")
MOD_TEST = re.compile(r"(?m)^(\s*)mod\s+(\w*test\w*)\s*\{")


def strip_tests(src: str) -> str:
    out = []
    pos = 0
    for m in MOD_TEST.finditer(src):
        # Only treat as test module when preceded by #[cfg(test)] nearby.
        head = src[max(0, m.start() - 200):m.start()]
        if "cfg(test)" not in head:
            continue
        out.append(src[pos:m.start()])
        depth = 1
        i = m.end()
        while i < len(src) and depth > 0:
            if src[i] == "{":
                depth += 1
            elif src[i] == "}":
                depth -= 1
            i += 1
        pos = i
    out.append(src[pos:])
    return "".join(out)


def strip_comments_strings(src: str) -> str:
    res = []
    i = 0
    n = len(src)
    while i < n:
        if src.startswith("//", i):
            j = src.find("\n", i)
            i = n if j < 0 else j
        elif src.startswith("/*", i):
            j = src.find("*/", i + 2)
            i = n if j < 0 else j + 2
        elif src[i] == '"':
            i += 1
            while i < n and src[i] != '"':
                if src[i] == "\\":
                    i += 1
                i += 1
            i += 1
        elif src[i] == "'":
            # lifetime or char: skip to closing quote on same line
            j = i + 1
            while j < n and src[j] not in ("'", "\n"):
                if src[j] == "\\":
                    j += 1
                j += 1
            i = j + 1
        else:
            res.append(src[i])
            i += 1
    return "".join(res)


bad = []
for path in FILES:
    src = path.read_text()
    clean = strip_comments_strings(strip_tests(src))
    for lineno, line in enumerate(clean.splitlines(), 1):
        if TOKEN.search(line):
            bad.append(f"{path.relative_to(ROOT)}:{lineno}:{line.strip()}")

if bad:
    print("BANNED_TOKENS_FOUND")
    for hit in bad:
        print(hit)
    sys.exit(1)
print("BANNED_CLEAN")

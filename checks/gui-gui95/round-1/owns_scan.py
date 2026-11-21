"""G6 oracle: disjoint OWNS honored.

Fails if `git status --porcelain` shows any touched path outside
crates/gui/** and checks/gui-gui95/**, or any change under
checks/gui-sessionpaint-by/**. Prints GUI_OWNS_OK otherwise.
"""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]

out = subprocess.run(
    ["git", "status", "--porcelain"],
    cwd=ROOT,
    capture_output=True,
    text=True,
    check=True,
).stdout.splitlines()

allowed = ("crates/gui/", "checks/gui-gui95/")
strays = []
for line in out:
    path = line[3:].strip().strip('"')
    # handle renames "old -> new"
    if " -> " in path:
        path = path.split(" -> ")[-1].strip()
    if not path.startswith(allowed):
        strays.append(line)

diff_out = subprocess.run(
    ["git", "status", "--porcelain", "--", "checks/gui-sessionpaint-by/"],
    cwd=ROOT,
    capture_output=True,
    text=True,
    check=True,
).stdout.strip()

if strays or diff_out:
    print("OWNS VIOLATION:")
    for line in strays:
        print(line)
    if diff_out:
        print(diff_out)
    raise SystemExit(1)
print("GUI_OWNS_OK")

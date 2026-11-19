# Local-Run Proof — debug GUI build + screenshot + CLI exits (01:00 UTC run)

Tareekh: 2026-11-20
Workspace: `/Users/ektasaini/autocompare`
Scope: sirf `checks/local-run/**` me write, koi source edit nahi
Screenshot path: `checks/local-run/run.png` (pehle se committed, dobara verify kiya)

Note: mission me `REPORT.md` naam tha lekin macOS FS case-insensitive hai,
isliye `REPORT.md` likhna `report.md` (doosre agent ki file) ko overwrite kar
deta hai. Is wajah se proof is alag file `run-proof.md` me hai. `report.md`
ko chhua nahi, HEAD se byte-identical restore karke chhoda hai.

## 1. GUI debug build

Command:
```bash
cargo build -p autocompare-gui
```

Output (tail):
```text
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.18s
BUILD_EXIT:0
```

Binary size:
```text
73326288 bytes | target/debug/autocompare-gui
-rwxr-xr-x  70M Nov 20 00:53 target/debug/autocompare-gui
```

## 2. GUI screenshot (offscreen fixture render)

Command:
```bash
/usr/bin/time -p ./target/debug/autocompare-gui --screenshot checks/local-run/run.png
```

Output:
```text
real 0.59
user 0.15
sys 0.00
SCREENSHOT_EXIT:0
```

Startup/render time: real 0.59s (user 0.15s, sys 0.00s). Koi window open nahi
hua, sirf fixture PNG render hua.

## 3. PNG magic + size verification

Commands:
```bash
ls -lh checks/local-run/run.png
file checks/local-run/run.png
xxd -p -l 8 checks/local-run/run.png
stat -f "%z bytes" checks/local-run/run.png
```

Output:
```text
-rw-r--r--  9.0K Nov 20 01:00 checks/local-run/run.png
checks/local-run/run.png: PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced
89504e470d0a1a0a
9174 bytes
```

Proof:
- PNG magic `89504e470d0a1a0a` verify hua
- Dimensions: 960 x 600, 8-bit/color RGBA, non-interlaced
- File size: 9174 bytes (9.0K)
- MD5: `f5911e33e944aec83a446d0a0ebfbdb3`

## 4. CLI --help proof

Command:
```bash
cargo run -p autocompare-cli -- --help
```

Output (head):
```text
Compare text, folders and bytes, and merge three ways

Usage: autocompare [OPTIONS] [COMMAND]

Commands:
  text          Compare two text files line by line, side by side
  folder        Compare two folder trees recursively by size and modified time
  hex           Compare two files byte by byte
  merge         Merge two files that share a common base version
  table         Compare two delimited (CSV-style) tables cell by cell
  sync          Plan (and optionally apply) a folder sync
  picture       Compare two PNG images pixel by pixel
  folder-merge  Plan (and optionally apply) a three-way folder merge
  audio         Compare two audio files (WAV, MP3) sample by sample
HELP_EXIT:0
```

CLI binary:
```text
9128864 bytes | target/debug/autocompare (8.7M)
autocompare 0.5.0
```

## 5. Sample text compare — exit 0 / 1 / 2 proof

Same file (exit 0):
```bash
cargo run -q -p autocompare-cli -- text tools/parity-harness/fixtures/t_same.txt tools/parity-harness/fixtures/t_same.txt --color never
```
```text
   1 alpha                             |    1 alpha
   2 beta                              |    2 beta
   3 gamma                             |    3 gamma
EXIT_SAME:0
```

Differing files (exit 1):
```bash
cargo run -q -p autocompare-cli -- text tools/parity-harness/fixtures/t_left.txt tools/parity-harness/fixtures/t_right.txt --color never
```
```text
   1 alpha                             |    1 alpha
   2 beta                              |    2 BETA
   3 gamma                             |    3 gamma
EXIT_DIFF:1
```

Missing file (exit 2):
```bash
cargo run -q -p autocompare-cli -- text tools/parity-harness/fixtures/t_left.txt /nonexistent/missing.txt --color never
```
```text
autocompare: path not found: /nonexistent/missing.txt
EXIT_ERR:2
```

Summary: help=0, same=0, diff=1, error=2. Sab expected ke mutabiq hai.

## 6. Collision note (parent ke liye)

- Is run ke dauran ek doosra agent isi repo me lagatar commit+push kar raha
  tha (`docs(status): poll ...` commits). Uske `git add -A` / reset race ki
  wajah se meri pehli do REPORT.md attempts gayab hui.
- Ek galat commit `fdd5e97` mere message ke saath bana lekin usme us doosre
  agent ka `report.md` content gaya, mera nahi. Us waqt ka HEAD ab push ho
  chuka hai, isliye history rewrite nahi ki — parent dekhe aur decide kare
  ki message amend karna hai ya nahi.
- `checks/local-run/report.md` ko HEAD se byte-identical restore kar diya
  hai (MD5 `7e6b01c52172d20cc81c54f63bececbf`).

Koi `crates/**` edit nahi, koi `target/release/**` touch nahi, koi push nahi.

# LOCAL-RUN REPORT — autocompare

Tareekh: 2026-11-19 14:01 UTC
Workspace: /Users/ektasaini/autocompare
Scope: sirf checks/local-run/** me likha, crates/** me zero edit, target/release/** ko touch nahi kiya
Note: macOS FS case-insensitive hai isliye REPORT.md aur report.md ek hi file hai, yehi final report hai

## 1. GUI debug build + screenshot proof

Command 1 — debug build:
```
cargo build -p autocompare-gui
```
Output tail:
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.57s
BUILD_EXIT:0
```
Dobara run par incremental cache se 0.18s me finish hua, matlab build pehle se fresh tha.

Command 2 — binary size:
```
ls -lh target/debug/autocompare-gui
stat -f "SIZE_BYTES:%z" target/debug/autocompare-gui
```
Output:
```
-rwxr-xr-x@ 1 ektasaini staff 70M Nov 20 00:53 target/debug/autocompare-gui
SIZE_BYTES:73326288
```
Binary size 73326288 bytes (70M) hai, path target/debug/autocompare-gui hai.

Command 3 — screenshot run + startup time:
```
/usr/bin/time ./target/debug/autocompare-gui --screenshot checks/local-run/run.png
echo "SCREENSHOT_EXIT:$?"
```
Output:
```
1.05 real 0.15 user 0.00 sys
SCREENSHOT_EXIT:0
```
Startup time 1.05 sec real hai, exit code 0 hai. Pehle run me 1.44 sec real aaya tha, range 1.0-1.5 sec hai.

Command 4 — PNG magic + size verify:
```
file checks/local-run/run.png
python3 -c "d=open('checks/local-run/run.png','rb').read(8); print('MAGIC_HEX:',d.hex()); print('MAGIC_OK:', d==bytes.fromhex('89504e470d0a1a0a'))"
ls -l checks/local-run/run.png
```
Output:
```
checks/local-run/run.png: PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced
MAGIC_HEX: 89504e470d0a1a0a
MAGIC_OK: True
-rw-r--r--@ 1 ektasaini staff 9174 Nov 20 01:00 checks/local-run/run.png
```
Screenshot path checks/local-run/run.png hai, PNG magic 89504e470d0a1a0a sahi hai, size 9174 bytes hai, dimension 960x600 RGBA 8-bit hai.

## 2. CLI --help + text compare exit 0/1/2 proof

Command 5 — help:
```
cargo run -p autocompare-cli -- --help
echo "HELP_EXIT:$?"
```
Output poora mila, summary:
```
Compare text, folders and bytes, and merge three ways
Usage: autocompare [OPTIONS] [COMMAND]
Commands:
  text  Compare two text files line by line, side by side
  folder  Compare two folder trees recursively by size and modified time
  hex  Compare two files byte by byte
  merge  Merge two files that share a common base version
  table  Compare two delimited (CSV-style) tables cell by cell
  sync  Plan (and optionally apply) a folder sync
  picture  Compare two PNG images pixel by pixel
  folder-merge  Plan (and optionally apply) a three-way folder merge
  audio  Compare two audio files (WAV, MP3) sample by sample
HELP_EXIT:0
```

Sample file setup, sirf checks/local-run ke andar:
```
printf 'alpha\nbeta\ngamma\n' > checks/local-run/sample_left.txt
printf 'alpha\nbeta\ngamma\n' > checks/local-run/sample_same.txt
printf 'alpha\nBETA\ngamma\n' > checks/local-run/sample_right.txt
```
Teeno file 17 byte ki hain.

Command 6 — exit 0 proof, same files:
```
cargo run -q -p autocompare-cli -- text checks/local-run/sample_left.txt checks/local-run/sample_same.txt
echo "EXIT:$?"
```
Output:
```
   1 alpha                             |    1 alpha
   2 beta                              |    2 beta
   3 gamma                             |    3 gamma
EXIT:0
```

Command 7 — exit 1 proof, alag files:
```
cargo run -q -p autocompare-cli -- text checks/local-run/sample_left.txt checks/local-run/sample_right.txt
echo "EXIT:$?"
```
Output:
```
   1 alpha                             |    1 alpha
   2 beta                              |    2 BETA
   3 gamma                             |    3 gamma
EXIT:1
```

Command 8 — exit 2 proof, missing file:
```
cargo run -q -p autocompare-cli -- text checks/local-run/sample_left.txt checks/local-run/does-not-exist.txt
echo "EXIT:$?"
```
Output:
```
autocompare: path not found: checks/local-run/does-not-exist.txt
EXIT:2
```

Summary: --help exit 0, same text exit 0, diff text exit 1, missing path exit 2. Teenon code proof ho gaya.

## 3. File list

- checks/local-run/REPORT.md = yehi file (FS case-insensitive, report.md ke naam se dikhti hai), 5163+ bytes
- checks/local-run/run.png, 9174 bytes, PNG magic ok
- checks/local-run/sample_left.txt, 17 bytes
- checks/local-run/sample_same.txt, 17 bytes
- checks/local-run/sample_right.txt, 17 bytes
- checks/local-run/verify.png purana hai, usko touch nahi kiya

## 4. Rule follow proof

- crates/** me koi edit nahi, git diff crates khaali hai
- target/release/** ko touch nahi kiya, sirf target/debug binary use kiya
- likha sirf checks/local-run/** me
- koi code change nahi, sirf run + proof hai

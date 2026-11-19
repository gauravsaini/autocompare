# autocompare

Compare text, folders and bytes, and merge three ways — from the terminal.

A clean-room rebuild of the classic file-comparison tool: side-by-side
text comparison with word-level inline highlights, recursive folder
comparison, byte-level (hex) comparison and three-way merge with
git-style conflict markers. Pure, safe Rust: no `unsafe`, no panics on
hostile input, and honest limits instead of silent ones.

Built from a user-facing study of the original — never from its insides.
See [`docs/study.md`](docs/study.md) for what was observed, with
confidence levels; no code, assets or wording were taken from it.

## Build

```sh
cargo build --release        # binary at target/release/autocompare
cargo test --workspace       # 72 tests
```

## Usage

### Text compare, side by side

```sh
autocompare text old.txt new.txt
```

For `old.txt` / `new.txt` containing:

```text
the quick brown fox        the quick red fox
jumps over                 jumps over
the lazy dog               the lazy dog
                           and keeps going
```

the tool prints (colors on a terminal: changed yellow with the changed
words accented, inserted green, deleted red):

```text
   1 the quick brown fox     |    1 the quick red fox
   2 jumps over              |    2 jumps over
   3 the lazy dog            |    3 the lazy dog
                             |    4 and keeps going
```

Options: `--unified` (patch view), `--ignore-case`,
`--ignore-space-change`, `--width N` (default 80).

```text
@@ -1,3 +1,4 @@
-the quick brown fox
+the quick red fox
 jumps over
 the lazy dog
+and keeps going
```

### Folder compare

```sh
autocompare folder left/ right/
```

Walks both trees recursively and prints every node with a status
glyph, sizes and dates:

```text
<          0          0  2026-11-18 10:40 2026-11-18 10:40  docs/
<         18         18  2026-11-18 10:40 2026-11-18 10:40  docs/guide.md
L         17          -  2026-11-18 10:40                -  left-only.txt
L          0          -  2026-11-18 10:40                -  nested/deep/hidden.txt
R          -         18                 - 2026-11-18 10:40  right-only.txt
9 entries: 0 same, 0 different, 0 newer left, 4 newer right, 4 left-only, 1 right-only
```

Glyphs: `=` same, `!` different (size or content), `>` newer left,
`<` newer right, `L` left-only, `R` right-only.

Options: `--content` (byte-compare files instead of size + time),
`--ignore-case` (name matching), `--include`/`--exclude` globs,
`--depth N` (default 32, capped walks are flagged in the output).

### Hex compare

```sh
autocompare hex bin_a bin_b
```

```text
left:  13 bytes
right: 13 bytes
first difference at offset 0x0000000c
0x0000000c   41                      A        |  42                      B
```

Options: `--max-hunks N` (default 50; more differences are flagged).

### Three-way merge

```sh
autocompare merge base.txt mine.txt theirs.txt
```

Non-overlapping edits apply cleanly; overlapping edits become one
conflict block:

```text
alpha
<<<<<<< left
BRAVO-left
=======
BRAVO-right
>>>>>>> right
charlie
delta
```

Options: `--output FILE` writes the result instead of standard output.

### Table compare

```sh
autocompare table old.csv new.csv
```

Compares delimited (CSV-style) files cell by cell. Changed cells show
as `[old => new]`; rows are aligned by position, inserted rows `R`,
removed rows `L`, equal rows `=` (dim). Quoted fields are parsed
RFC-4180-lite, so delimiters and line breaks inside quotes are data.

```text
!  hdr | name | [size => extent] | kind
=    1 | alpha | 10 | core
!    2 | bravo | [20 => 25] | core
=    3 | charlie,x | 30 | extra
R    4 | echo | 50 | core
=    5 | delta | 40 | core
```

Options: `--delimiter <char>` (default `,`), `--no-header`.

### Folder sync

```sh
autocompare sync left/ right/            # dry run: prints the plan
autocompare sync left/ right/ --apply    # executes it
```

Plans a one-directional (`--direction l2r|r2l`) or `--newer` sync.
One-directional sync never overwrites a newer destination (skipped
with a reason); entries whose content differs never lose to either
side; `--prune` deletes one-sided entries on the losing side.
Symlinks are never followed, copied or written through.

```text
→ left-only.txt
→ nested/deep-file.txt
~ mismatch.txt (content differs; no side wins)
~ newer.txt (destination is newer)
2 step(s) planned, 5 skipped
dry run: nothing changed (pass --apply to run)
```

`--apply` copies files (creating parent directories as needed) and
performs prune deletes; every step's failure is recorded and the run
continues. Exit codes: dry run with steps → 1; apply without failures
→ 0; any failed step → 1.

### Folder merge

```sh
autocompare folder-merge base/ left/ right/                 # dry run
autocompare folder-merge base/ left/ right/ --apply --output merged/
```

Three-way merge of two trees over their common base. Identical sides
take the left copy; one-sided edits take the edited side; files
changed on both sides merge as text (clean merges become `M`,
overlapping edits become `C` with git-style markers in the output);
both-changed binaries keep the left copy and are flagged as
conflicts. Deletions against an unchanged other side omit the file.

```text
L a.txt
R b.txt
C f.txt
M e.txt
~ c.txt (deleted on the left; right unchanged from base)
! f.txt (conflict)
13 step(s) planned, 1 omitted, 2 conflict(s)
```

`--apply` requires `--output DIR`, which must be missing or empty;
nothing is merged in place. Symlinks are never followed, read or
copied. Files above 32 MiB are treated as binary.

### Picture compare

```sh
autocompare picture left.png right.png [--heatmap diff.png]
```

Compares two PNG images pixel by pixel over their overlap:

```text
left:  8x8
right: 8x8
overlap: 8x8 (64 pixels)
differing pixels: 14 (21.88%)
difference bounds: (2,2)..(6,6)
```

`--heatmap FILE` writes a PNG with identical pixels dimmed to 30%
grayscale and differing pixels in red. Hostile inputs (forged
dimension headers) are refused, not crashed.

### Audio compare

```sh
autocompare audio a.wav b.wav
```

Compares WAV and MP3 files sample by sample over the shorter stream's
duration:

```text
left:  1.000s, 8000 Hz, 1 channel(s)
right: 1.000s, 8000 Hz, 1 channel(s)
first difference at 0.500s
differing 10ms windows: 1/100 (1.00%)
```

WAV decoding is a hand-written RIFF parser (no dependency: PCM
8/16/24/32-bit and IEEE float, 1–64 channels); MP3 decoding uses the
pure-Rust symphonia engine. Files that do not decode are refused with
a plain error; differing sample rates or channel counts are reported
honestly as not comparable instead of guessed.

### GUI client

```sh
cargo run -p autocompare-gui
```

A thin desktop client (eframe/egui) with nine tabs over the same
engine libraries the CLI uses — Text, Folder, Hex, Merge, Table,
Sync, Picture, FolderMerge and Audio, matching the nine session
types of the original: side-by-side rows with accent highlights,
folder listings with status colors, hex/plan/stat summaries, the
three-way merge output with conflict highlighting, `[old => new]`
table cells, and the difference heatmap rendered in-app. Merge and
FolderMerge take base/left/right paths; Sync and FolderMerge show
the dry-run plan (applying stays a CLI operation). Errors appear as
panels; the window never crashes on bad input (all engine errors
are values).

On Linux, the windowing system development packages are needed first
(the CI workflow installs exactly this set):

```sh
sudo apt-get install -y libxkbcommon-dev libxkbcommon-x11-0 \
  libwayland-dev libx11-dev libxrandr-dev libxi-dev libgl1-mesa-dev
```

### Reports, VCS flags and headless screenshot

```sh
autocompare text left.txt right.txt --report html:report.html
autocompare text base.txt left.txt --ro
cargo run -p autocompare-gui -- --screenshot gui-smoke.png
```

Every subcommand accepts `--report <fmt:path>` (`html`, `text`,
`xml`, `patch`) alongside the normal exit code. The `-ro`,
`-automerge`, `-mergeoutput <path>` and `-vcs1..-vcs4` flags mirror
the original's VCS invocation shape: `-ro` forces read-only mode,
`-automerge` writes the merged output only when conflict-free. The
GUI stays a thin client but supports a clean-room check contract:
`--screenshot <path>` renders the Text fixture session offscreen,
writes a valid PNG and exits 0 without opening a window.

## Exit codes

`0` no differences · `1` differences found (or merge conflicts) ·
`2` error — the diff(1) convention, scriptable.

## Design

- **Clean room**: rebuilt from user-visible behavior of the original,
  never from its code or assets.
- **Layers**: pure algorithms (`diff`) → filesystem comparison
  (`compare`) → thin CLI renderer (`cli`).
- **Never crash**: `unsafe` forbidden; `unwrap`/`expect`/`panic!` and
  indexing panics denied by clippy in non-test code; hostile paths,
  huge files, deep trees and invalid UTF-8 all return errors or
  degrade honestly.
- **Honest limits**: the line diff falls back to a coarse view past an
  edit budget (`exact=false`, noted in the output); folder walks cap
  depth; hex caps hunks. Limits are stated, never hidden.
- **Safe walks**: symlinks are never followed, so symlink cycles
  cannot hang a folder compare.

## Tests

406 workspace tests: replay validity for every diff case, brute-force
minimality on random pairs, budget fallback, symlink-safe folder
fixtures, streamed binary compare, RFC-4180-lite parser edge cases,
sync plan and apply semantics, three-way tree merge rules, PNG and
audio decode totality sweeps (every prefix truncation of a valid WAV,
random buffers, hostile headers), plus 57 headless GUI view-model
tests covering all nine tabs. Five exhaustive sweeps (~430k pairs, each checked for replay,
canonical form and minimal edit count) run separately in release
mode:

```sh
cargo test -p autocompare-diff --release -- --ignored
```

## Roadmap

- More audio codecs (FLAC, OGG)
- Sync / folder-merge apply actions in the GUI (plan views exist; `--apply` stays CLI)
- In-place editing (the original's panes are editors)

## License

MIT — see [LICENSE](LICENSE).

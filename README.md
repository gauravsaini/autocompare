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

126 workspace tests: replay validity for every diff case, brute-force
minimality on random pairs, budget fallback, symlink-safe folder
fixtures, streamed binary compare, RFC-4180-lite parser edge cases
(NULs, megabyte cells, multibyte, unterminated quotes), sync plan and
apply semantics on real trees. Five exhaustive sweeps (~430k pairs,
each checked for replay, canonical form and minimal edit count) run
separately in release mode:

```sh
cargo test -p autocompare-diff --release -- --ignored
```

## Roadmap

- Media and picture compares
- Folder merge
- GUI client over the same engine

## License

MIT — see [LICENSE](LICENSE).

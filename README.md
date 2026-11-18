# autocompare

A clean-room comparison tool: side-by-side text compare, recursive folder
compare, byte-level (hex) compare and three-way merge, built as a
headless-first Rust workspace (thin CLI over pure libraries).

Built from a user-facing study of a popular file-comparison application.
No code, assets or wording were taken from it; see `docs/study.md` for
what was observed and with what confidence.

Status: under construction.

```text
crates/
  diff      L0  pure algorithms: Myers line diff, inline spans, 3-way merge
  compare   L1  text/folder/binary comparison over the filesystem
  cli       L2  the autocompare binary: rendering, colors, exit codes
```

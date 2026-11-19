# Parity Sess Report — Headless Session Persistence + Reports + VFS

Date: 2026-11-19
Scope: `crates/sessions`, `crates/reports`, `crates/vfs`, `crates/evidence`
Read-only (reused via API, no edits): `crates/gui`, `crates/cli`, `crates/compare`, `crates/diff`
Harness: `cargo test -p autocompare-sessions -p autocompare-reports -p autocompare-vfs`

## Result

GREEN. 46 tests passed, 0 failed.

| Crate | Test target | Passed |
|---|---|---|
| autocompare-sessions | lib unit (session/workspace/settings) | 11 |
| autocompare-sessions | tests/round_trip.rs | 3 |
| autocompare-sessions | tests/migration.rs | 8 |
| autocompare-sessions | tests/corrupt_input.rs | 7 |
| autocompare-reports | tests/snapshot_tests.rs | 9 |
| autocompare-vfs | tests/local_test.rs | 3 |
| autocompare-vfs | tests/snapshot_test.rs | 2 |
| autocompare-vfs | tests/tar_test.rs | 2 |
| autocompare-vfs | tests/zip_test.rs | 1 |

Clippy: `cargo clippy -p autocompare-sessions -p autocompare-reports -p autocompare-vfs --all-targets -- -D warnings` clean.
Lints enforced at workspace level: `unsafe_code = "forbid"`, `unwrap_used/expect_used/panic/todo/unimplemented/unreachable/indexing_slicing = "deny"`.
Source check: no `unwrap()/expect()/panic!/todo!` in non-test `src/` of owned crates; no `unsafe` blocks (only `forbid(unsafe_code)` attributes and doc comments).

## Coverage

### 1. Versioned session persistence (`crates/sessions`)

- `SessionDocument { schema_version, tabs, active_tab_index, name }`, `CURRENT_SESSION_SCHEMA_VERSION = 1`.
  - `to_json` / `to_json_pretty` / `from_json` (auto-migrate) / `save_to_file` / `load_from_file` / `migrate`.
  - `migrate()`: v0 unversioned object, explicit `schema_version: 0`, legacy field aliases (`sessions`/`items`, `uuid`, `name`/`title`, `type`/`kind`, `left`/`path_left`, `right`/`path_right`), bare tab arrays, bare string-path arrays. Future versions return `SessionError::UnsupportedVersion`. Non-object/non-array roots, empty input, bad types return `CorruptInput` or `Json` errors. Never panics on hostile input.
- `WorkspaceDocument { schema_version, name, session_paths, active_session, window_layout, recent_paths, metadata }`, `CURRENT_WORKSPACE_SCHEMA_VERSION = 1`.
  - Same JSON/file API surface. v0 migration maps `title` -> `name`, `sessions` -> `session_paths`, fills defaults for missing window layout/recents/metadata. Future version rejected.
- `AppSettings { schema_version, theme, keymap, grammar_preset_name (+aliases grammar_preset/grammar), font_family, font_size, tab_size, auto_save, show_line_numbers, extra }`, `CURRENT_SETTINGS_SCHEMA_VERSION = 1`.
  - v0 migration maps `keys` -> `keymap`, `grammar`/`grammar_preset` -> `grammar_preset_name`. Future version rejected.
- `SessionError { Io, Json, UnsupportedVersion { found, max_supported }, CorruptInput }` with `Display` + `std::error::Error` + `From` impls.
- Tests: compact + pretty + file round-trips for all three documents, v0/legacy-field migrations, future-version rejection, corrupt-syntax matrix, hostile/fuzzed inputs via `catch_unwind` proving no panic.

### 2. Report generators (`crates/reports`)

Decoupled input model: `ReportInput { title, hunks: Vec<Hunk>, stats: DiffStats }`, `Hunk { old_start, old_lines, new_start, new_lines, lines }`, `HunkLine { change, content, old_line_no, new_line_no }`, `DiffStats { additions, deletions, modifications, unchanged }`, `ChildReportAnchor { title, href }`.

- HTML (`src/html.rs`): `HtmlReportBuilder` + `HtmlColorBuilder` + `HtmlMonoBuilder`, themes `Color`/`Mono`, child-report `<nav>` anchors, subtitle, stats bar, per-hunk tables with old/new line numbers and change markers. Helpers: `render_html`, `render_html_color`, `render_html_mono`, `render_html_with_children`. `escape_html` covers `& < > " '`.
- Plain text (`src/text.rs`): `TextReportBuilder` with optional line numbers and subtitle, `SUMMARY` block, `LINKED CHILD REPORTS` block, hunk headers. Helpers: `render_text`, `render_text_with_children`.
- XML (`src/xml.rs`): `XmlReportBuilder`, `<?xml?>` + `<diff-report xmlns="urn:autocompare:report:v1">`, `<title>`, `<subtitle>`, `<stats .../>`, `<child-reports>`, `<hunks>/<hunk>/<line change=...>`. Helpers: `render_xml`, `render_xml_with_children`. `escape_xml` covers `& < > " '` (apostrophe as `&apos;`).
- Patch (`src/patch.rs`): `PatchReportBuilder` with `PatchFormat::{Unified, Context}`, `with_paths/with_old_path/with_new_path`, `unified()/context()/format()`. Unified emits `--- a/...`, `+++ b/...`, `@@ -a,b +c,d @@` with ` /-/+/!` prefixes. Context emits `***`/`---` blocks with old/new sections. Helpers: `render_patch` (= unified), `render_unified_patch`, `render_context_patch`.
- Tests: color/mono snapshots with child anchors and subtitle, text snapshot + line-number mode, XML snapshot, unified + context snapshots, hostile XSS input escaping (HTML/XML escaped, text/patch pass through without panic), empty-input behavior.

### 3. VFS local read path (`crates/vfs`)

- `VfsPath`: normalized relative forward-slash path, rejects `..` and null bytes, `join/parent/file_name/extension/depth/components/to_physical_path`.
- `Vfs` trait: `list_dir`, `stat`, `read_stream(path, max_bytes)`, `read_all` (double-bounded, detects over-read), `walk` (BFS, `WalkOptions { follow_symlinks, max_depth, refuse_cycles }`), `exists`, `is_read_only`, `write_atomic`, `delete`.
- `LocalFs` (`src/local.rs`): rooted physical FS, `new` validates dir, `resolve` contains paths inside root, `list_dir`/`stat` via `symlink_metadata` (symlinks reported as symlink-only, never implicitly followed), `read_stream` with `FileTooLarge` pre-check + `BoundedReader` streaming cap, `write_atomic` via temp-file + `fsync` + rename with unique counter, `delete` refuses to follow symlinks into dirs, `walk_symlink_safe` with canonical-dir set and `SymlinkCycle` refusal, read-only mode, deterministic sorted listings, skips non-UTF8 names.
- Archive/snapshot (supporting, unchanged): `ZipFs`, `TarGzFs`, `SnapshotFs` (read-only metadata view), `create_snapshot` with CRC32 streaming (64 MB cap), `serialize/deserialize_bcss` (plain JSON + gzip auto-detect, strict `BCSS_SCHEMA_VERSION = 1` check).
- Tests: basic ops (stat/list/read/cap/atomic-write/delete), symlink cycle refusal in both follow and no-follow modes, read-only mode, snapshot round-trip + browse, tar plain + gzip, zip traversal + read.

### 4. Evidence (`crates/evidence`, read supporting role)

`CURRENT_SCHEMA_VERSION = 1`, `Confidence`, `LimitationCode` (8 codes), `CoverageReport`, `EvidenceRecord`, `HypothesisRecord`, `KnowledgeGraphRecord`, `ToolError`. Pure data, zero I/O, WASM-compatible. Covered by its own lib unit tests.

## Limitations

1. Snapshot FS is metadata-only: `SnapshotFs::read_stream` always returns `UnsupportedFormat` because `.bcss` stores metadata without file payloads. Folder-level parity comparisons against snapshots must use metadata/CRC, not byte reads.
2. Session migration covers v0 -> v1 only. There is no v2 schema and no downgrade path; any `schema_version > 1` is a hard `UnsupportedVersion` error.
3. `create_snapshot` CRC streaming caps files at 64 MB and silently records `crc32: None` on read errors instead of failing; large-file snapshots are therefore hash-incomplete by design.
4. Local FS skips non-UTF8 filenames silently during listing; such files are invisible to compare rather than reported.
5. Reports are headless string builders only. No GUI preview integration and no direct `compare`/`diff` engine adapter live in this scope; callers must construct `ReportInput` from engine output themselves.
6. No WASM exercise in this harness; `evidence` claims WASM compatibility but no wasm target was built or tested here.

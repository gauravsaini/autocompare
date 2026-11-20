# round-sessrep comparison fixtures (sessions, reports, and VFS clean-room depth)

Edge fixtures covering sessions persistence/migrations, multi-format report builders with hostile escaping/Unicode, and VFS archive/snapshot/symlink safety.

## 1. Sessions Edge Fixtures
- `edge-sess-01-v0-bare-array.json`: Legacy v0 session with bare array of paths `["/path/a.txt", "/path/b.txt"]` and unversioned fields. Demonstrates automatic migration to v1 schema with defaults preserved.
- `edge-sess-02-future-corrupt.json`: Future schema version (`schema_version: 999`) and corrupted inputs. Verifies safe error return (`FutureSchemaVersion` / `Syntax`) without panicking.
- `edge-sess-03-workspace-presets.json`: Complete `WorkspaceDocument` containing custom grammar presets, folder sync options (`Mirror`, depth_limit 2), and locked workspace guard.

## 2. Reports Edge Fixtures
- `edge-rep-01-hostile-xss-left.txt` & `edge-rep-01-hostile-xss-right.txt`: Hostile HTML/XSS injection payloads (`<script>`, `<svg/onload>`, attribute breakout, ampersands, quotes). Verifies escaping in HTML, XML, text, and patch renderers.
- `edge-rep-02-unicode-left.txt` & `edge-rep-02-unicode-right.txt`: Multi-byte Unicode, CJK characters, Arabic RTL phrases, emojis, and CRLF / mixed line endings.
- `edge-rep-expected.json`: Expected validation criteria across HTML (color & monochrome), Plain Text, XML, and Unified Patch builders.

## 3. VFS Edge Fixtures
- `edge-vfs-01-zipslip-spec.json`: ZipSlip / directory traversal attack payloads (`../../etc/shadow`, `..\windows\win.ini`, null bytes) verifying refusal by `VfsPath` and archive extractors.
- `edge-vfs-02-snapshot.bcss`: Valid `.bcss` Snapshot Document verifying roundtrip serialization, CRC32 checksums, shallow/deep traversal, and diff summaries.
- `edge-vfs-03-symlink-cycle.sh`: Executable bash test verifying LocalFS cycle refusal, link containment, atomic write safety, and read-only refusal.

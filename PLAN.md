Here is the full, engineering-grade PLAN.md for building a clean-room replica of Beyond Compare 5 (macOS) following storytold/craft-anything. It incorporates the baseline audit of the existing autocompare v0.5.0 workspace, the full Beyond Compare 5 macOS feature & UX specification, the craft-anything project layout and verification loop, shared Rust contracts, a parallel subagent Depth Tree with disjoint OWNS: paths, and runnable CHECK: / EXPECT: acceptance gates.

Once you confirm whether you want this written to /Users/ektasaini/autocompare/PLAN.md (extending autocompare v0.5.0 → v1.0.0) or scaffolded in a new subfolder from scratch, I'll save PLAN.md to disk and kick off Wave 0.

[invalid URL] — Clean-Room Beyond Compare 5 Replica (craft-anything Methodology)

1. Methodology, Clean-Room Boundary & Project Layout

This build follows the storytold/craft-anything four-stage loop (Study → Build → Check → Ready) and engineering standards ([invalid URL], docs/STATUS.md, docs/ROADMAP.md).

1.1 Clean-Room Contract (Default Mode)

⚬ Observe from outside only: Record user-visible behavior, session menus, layout geometry, comparison rules, and CLI/VCS contracts in capture/ and docs/study.md with explicit confidence (HIGH = directly observed, MEDIUM = product-class documented behavior, UNVERIFIED = pending live visual check) ([invalid URL], [invalid URL]).
⚬ Prohibited: No decompilation, disassembly, binary string extraction, resource/icon/asset copying, or copying of proprietary wording or names ([invalid URL]).
⚬ Honest Evidence & Failure Contract: Every inspection or comparison result carries explicit coverage and structured limitations (exact: bool, truncated: bool, limitations: Vec<LimitationCode>). Permission errors (e.g., macOS Accessibility or Screen Recording denied) return structured errors rather than fabricated empty success (docs/STATUS.md, docs/ROADMAP.md).

1.2 craft-anything Workspace Layout

Mirroring the canonical craft-anything project structure:

<project-root>/
├── craft.json                 # Versioned project manifest (target, mode provenance, rounds, not_working_checks)
├── PLAN.md                    # This living architecture, Depth Tree, and acceptance gate contract
├── capture/                   # Clean-room study inputs (screenshots, ui-tree.json, capture.json, study notes)
├── checks/                    # Per-round check outputs (round-N/capture.json, parity reports, screenshots)
├── investigation.cany/        # Structured evidence records, hypotheses, unknowns, and coverage logs
├── inbox/                     # Open user decisions, approval cards, build-progress.json
└── app/ (or workspace root)   # Pure-Rust workspace (edition 2024, unsafe_code = "forbid")


1.3 Non-Negotiable Rust Engineering Rules ([invalid URL])

⚬ Safe Rust & Never Crash: #![forbid(unsafe_code)] workspace-wide; clippy denies unwrap_used, expect_used, panic, todo, unimplemented, unreachable, and indexing_slicing in all non-test code. Checked arithmetic, bounded recursion, capped allocations.
⚬ Headless First ("Everything Is a Tool"): All comparison, merge, sync, session persistence, reporting, and VFS operations live in headless library crates and expose typed ToolSpec / command APIs. The egui UI is strictly a thin client with zero business logic or direct filesystem orchestration.
⚬ Strict Layering (xtask layers): Lower crates never depend on higher crates; core diff/format/evidence layers (L0–L2) stay WASM-clean (xtask wasm).
⚬ Process & Lock Discipline: External helpers (optional VCS/archive tools) run through a dedicated process wrapper with closed/piped stdin, wall-clock + idle deadlines, and capped output draining—never raw std::process::Command on the UI thread. Background jobs stream typed progress/results to the UI.
⚬ Zero Secrets & Versioned Persistence: All persisted configs (sessions.json, workspaces.json, settings.json, .bcss-style snapshots) include a schema_version field and migration path.

2. Baseline Audit (autocompare v0.5.0) vs. Beyond Compare 5 Gap Matrix

The existing autocompare v0.5.0 workspace (crates/diff, crates/compare, crates/cli, crates/gui; 264 tests) already covers the foundational read-only comparison algorithms for all 9 macOS session types, but has major gaps relative to a true Beyond Compare 5 desktop application:

Capability Area	Existing autocompare v0.5.0	Beyond Compare 5 (macOS) Target	Parity Gap to Close
Home View & Sessions	None (single window with 9 static tabs; typed path inputs)	Home View, session tree search (⌘F), saved/auto-saved/locked sessions, workspaces, multi-tab/multi-window, drag-and-drop	Build sessions crate + Home View, tabbed session workspace, persistence, native file/folder dialogs (rfd), drag-and-drop
Text Compare	Read-only side-by-side & unified diff; --ignore-case, --ignore-space-change	Live side-by-side / over-under editors, Word Wrap, thumbnail overview gutter, inline copy left/right arrows, next/prev diff navigation, display filters (All/Diffs/Same/Context), grammars & unimportant text, syntax highlighting, bottom detail pane	Build piece-table/rope live editor state, continuous re-diff, thumbnail gutter, copy-section/copy-line actions, grammar/unimportant-text engine, display filters, bookmarks
Text Merge (3-Way)	CLI/read-only git-style conflict markers	3-pane Text Merge (Left / Center Ancestor / Right) + editable Output pane (detachable), Take Left/Center/Right actions, Myers/Patience/Skew alignment, conflict navigation	Build interactive 3-way merge model with per-hunk Take actions, editable output buffer, conflict stepper, and save-to-output
Folder Compare	Flat recursive listing with 6 status glyphs, --content, globs, depth cap	Collapsible dual-pane tree, quick vs. CRC/binary/rules tests, rich display filters (No Orphans, Left/Right Newer, Flatten), double-click to open child file sessions, Copy/Move/Delete/Touch, background scan	Build interactive tree model with expand/collapse, child-session spawning, reconciliation actions with undo/confirm, full filter presets, background worker
Folder Sync & Merge	CLI plan + --apply; GUI shows dry-run plan only	Interactive Folder Sync (Update Left/Right/Both, Mirror, conflict nav, log panel) & Folder Merge with GUI execution	Add GUI execution with explicit consent/preview, Update Both / Mirror modes, conflict navigation, and live operation log
Hex Compare	8-byte CLI/GUI summary hunks (--max-hunks)	Scrollable Hex Compare & Editor with Complete/Fast/None alignment, synchronized hex+ASCII panes, 64-bit Go To, byte editing	Build virtualized hex/ASCII viewer + byte editor, alignment modes, 64-bit offset jump
Table Compare	RFC-4180 CSV row-by-position diff	Table Compare with key columns, column mapping, sort-before-align, numeric tolerance, multi-sheet Excel (.xlsx) & TSV/HTML tables	Add key-column alignment, column remapping, sort-before-align, numeric tolerance, and calamine .xlsx multi-sheet support
Picture Compare	PNG pixel diff + static red/gray heatmap	Picture Compare with Tolerance, Mismatch Range, and Blend modes, autoscale/rotate/flip, pixel-offset nudge, zoom/pan, JPEG/PNG/GIF/TIFF/BMP	Expand image decoders (image crate), implement Tolerance/Range/Blend renderers, zoom/pan/offset-nudge controls
Media Compare	WAV + MP3 PCM 10ms window diff	Media Compare comparing metadata tags across FLAC, MP3, MP4/AAC (plus optional PCM diff)	Add pure-Rust metadata tag reader (lofty) for FLAC/MP3/MP4/AAC alongside existing PCM audio diff
Reports, VCS & VFS	None	HTML/Text/XML/Patch reports, bcomp CLI & Git/VCS switches/exit codes, Zip/Tar/Gzip archives & .bcss snapshots, SFTP/WebDAV VFS	Build reports module, bcomp-compatible CLI wrapper & exit codes, virtual filesystem (Vfs trait) for local + Zip/Tar/Snapshots
macOS UX & Polish	Basic egui tabs, no themes/shortcuts	Dark/Light themes, customizable shortcuts (⌘F, ⌘I, ⌘0), --screenshot check contract	Build typed design-token theme system (Dark/Light), keymap table, and --screenshot headless verification flag

3. Target Architecture & Shared Contracts (Locked Before Fan-Out)

3.1 Layered Crate Map (cargo xtask layers)

L0 (Pure, zero I/O, WASM-safe)
├── crates/evidence     # EvidenceRecord, Confidence, LimitationCode, ToolError
├── crates/diff         # Text (Myers/Patience/inline), 3-way Merge, Grammar/Unimportant rules,
│                       # Hex alignment, Table (key cols, sort, tolerance), Image (Tolerance/Range/Blend),
│                       # Media (tags + PCM)
L1 (Domain & VFS Abstractions)
├── crates/vfs          # Vfs trait: LocalFs, ArchiveFs (zip/tar.gz), SnapshotFs (.bcss JSON/zstd)
├── crates/sessions     # Versioned SessionConfig, WorkspaceConfig, FileFormat/Grammar rules, Settings store
├── crates/reports      # HTML (color/mono/linked), Plain Text, XML, Unified/Context Patch generators
L2 (Orchestration & Tool Registry)
├── crates/compare      # Folder tree compare, Sync planner/applier, 3-way Folder merge, File/Media runners,
│                       # Background JobRunner (cancellation + bounded progress channels), ToolSpec registry
L3 (Thin Delivery Surfaces)
├── crates/cli          # `autocompare` + `bcomp`-compatible VCS/script flags, diff(1) & BC exit-code modes
├── crates/gui          # `eframe`/`egui` desktop app (ONLY crate allowed to depend on egui/eframe/rfd)
└── xtask/              # `cargo xtask layers | check-parity | secrets | wasm | ci`


3.2 Shared Core Interfaces & Schemas (Locked in Wave 0)

// crates/evidence/src/lib.rs
pub enum Confidence { High, Medium, Unverified }
pub enum LimitationCode {
    EditBudgetFallback, DepthCapped, HunkCapped, BinaryTreatedAsOpaque,
    PermissionDenied, SymlinkSkipped, TruncatedOutput, UnsupportedCodec,
}
pub struct CoverageReport {
    pub exact: bool,
    pub truncated: bool,
    pub limitations: Vec<LimitationCode>,
}

// crates/vfs/src/lib.rs
pub trait Vfs: Send + Sync {
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<VfsEntry>, VfsError>;
    fn stat(&self, path: &VfsPath) -> Result<VfsMeta, VfsError>;
    fn read_stream(&self, path: &VfsPath, max_bytes: u64) -> Result<Box<dyn std::io::Read + Send>, VfsError>;
    fn write_atomic(&self, path: &VfsPath, bytes: &[u8]) -> Result<(), VfsError>; // returns ReadOnly on archives/snapshots
    fn delete(&self, path: &VfsPath) -> Result<(), VfsError>;
    fn is_read_only(&self) -> bool;
}

// crates/sessions/src/[invalid URL]
pub enum SessionKind {
    FolderCompare, FolderMerge, FolderSync,
    TextCompare, TextMerge, HexCompare,
    TableCompare, PictureCompare, MediaCompare,
}
pub struct SessionDocument {
    pub schema_version: u32,            // starts at 1
    pub id: String,
    pub name: String,
    pub kind: SessionKind,
    pub locked: bool,
    pub paths: SessionPaths,            // TwoWay { left, right } | ThreeWay { base, left, right, output }
    pub rules: SessionRules,            // Filters, importance, alignment, display mode
}

// crates/compare/src/jobs.rs
pub struct JobHandle<T> { /* bounded channel + AtomicBool cancel token; UI polls without blocking */ }


4. Contract Inventory (Revision 1)

Contract ID	Required Outcome / Constraint	Owner Leaf	Observing Gate
C1	Workspace compiles cleanly with unsafe_code = "forbid", strict clippy lints, and xtask layers enforcement	leaf-0.1	leaf-0.1:G1, leaf-0.1:G2
C2	Clean-room craft.json, docs/study.md, and investigation.cany/ evidence schema initialized	leaf-0.1	leaf-0.1:G3
C3	Vfs abstraction supports LocalFS (symlink-safe), Zip/Tar archives, and .bcss snapshots	leaf-1.1	leaf-1.1:G1, leaf-1.1:G2
C4	Text engine supports Myers + Patience diff, grammar/unimportant-text rules, live edit buffer, and 3-way Take merge	leaf-1.2	leaf-1.2:G1, leaf-1.2:G2
C5	Table, Hex, Picture, and Media engines cover key-column/multi-sheet .xlsx, alignment modes, Tolerance/Range/Blend, and FLAC/MP3/MP4 tags	leaf-1.3	leaf-1.3:G1, leaf-1.3:G2
C6	Session & Workspace persistence (sessions.json, workspaces.json, settings.json, grammar presets) with schema versioning	leaf-2.1	leaf-2.1:G1
C7	Report generator outputs HTML (color/mono + linked child reports), Plain Text, XML, and Unified/Context Patch	leaf-2.2	leaf-2.2:G1
C8	Folder Compare, Sync (Update Left/Right/Both, Mirror), and Folder Merge run over Vfs with cancellable background jobs	leaf-2.3	leaf-2.3:G1, leaf-2.3:G2
C9	CLI supports all 9 subcommands, report generation, and bcomp-compatible VCS switches (-ro, -automerge, -mergeoutput, -vcs1..4)	leaf-3.1	leaf-3.1:G1, leaf-3.1:G2
C10	egui desktop app provides Home View, multi-tab workspace, interactive Text/Merge/Folder/Hex/Table/Picture/Media panes, Dark/Light themes, rfd dialogs, and --screenshot check mode	leaf-3.2	leaf-3.2:G1, leaf-3.2:G2
C11	End-to-end craft-anything Check loop passes: console parity, GUI --screenshot capture, headless view-model suite, and release build	root-4	root-4:G1, root-4:G2, root-4:G3

5. Depth Tree & Parallel Subagent Dispatch Table

Each leaf owns strictly disjoint file paths (OWNS:) and builds with an isolated CARGO_TARGET_DIR=target/agent-<leaf-id> to eliminate lock contention ([invalid URL]).

Wave	Leaf ID	Subagent Role	Deliverable	OWNS: (Disjoint Paths)	Needs	State
Wave 0	leaf-0.1	Workspace & Schema Architect	Workspace Cargo.toml, crates/evidence/**, xtask/**, craft.json, shared trait stubs	Cargo.toml, craft.json, crates/evidence/**, xtask/**, docs/study.md	none	READY
Wave 1	leaf-1.1	VFS & Archive Engineer	Vfs trait + LocalFS (symlink-safe), Zip/Tar.gz reader, .bcss snapshot serializer	crates/vfs/**	leaf-0.1	WAITING
Wave 1	leaf-1.2	Text & Merge Engine Engineer	Myers/Patience diff, grammar/unimportant-text rules, editable text buffer, 3-way Take merge	crates/diff/src/text*, crates/diff/src/merge*, crates/diff/src/grammar*	leaf-0.1	WAITING
Wave 1	leaf-1.3	Binary, Table & Media Engineer	Hex alignment (Complete/Fast/None), Table key-cols/sort/.xlsx, Picture Tolerance/Range/Blend, Media tags (lofty) + PCM	crates/diff/src/hex*, crates/diff/src/table*, crates/diff/src/image*, crates/diff/src/audio*, crates/diff/src/media_tags*	leaf-0.1	WAITING
Wave 2	leaf-2.1	Sessions & Config Engineer	Versioned SessionDocument, WorkspaceDocument, AppSettings, theme tokens, keymaps	crates/sessions/**	leaf-0.1	WAITING
Wave 2	leaf-2.2	Reporting Engine Engineer	HTML (color/mono + linked child reports), Plain Text, XML, and Patch report builders	crates/reports/**	Wave 1	WAITING
Wave 2	leaf-2.3	Folder & Job Orchestrator	Vfs-backed Folder Compare tree, Sync (Update Both/Mirror), Folder Merge, background JobRunner	crates/compare/**	Wave 1	WAITING
Wave 3	leaf-3.1	CLI & VCS Engineer	9 subcommands, report CLI, bcomp-compatible VCS mode (git diff/mergetool), exit-code suites	crates/cli/**	Wave 2	WAITING
Wave 3	leaf-3.2	Desktop GUI Engineer	egui Home View, multi-tab manager, thumbnail gutter, live Text/Merge editors, Folder tree drill-down, Hex/Table/Picture/Media views, Dark/Light themes, --screenshot hook	crates/gui/**	Wave 2	WAITING
Wave 4	root-4	Verification & Release Lead	Cross-crate integration, craft-anything Check rounds, release build, [invalid URL], [invalid URL]	checks/**, [invalid URL], [invalid URL]	Wave 3	WAITING

6. Per-Leaf Runnable Acceptance Gates (CHECK: / EXPECT:)

Every leaf must execute all 4 passes (Complete Implementation → Expert Reread → Defect Hunt → Polish & Gate Proof), and the parent orchestrator re-verifies every gate before releasing the leaf lease.

Wave 0 — leaf-0.1

⚬ [ ] leaf-0.1:G1: Workspace lints and layer rules pass with zero warnings.
  ⚬ CHECK: cargo clippy --workspace --all-targets -- -D warnings && cargo xtask layers
  ⚬ EXPECT: layers: ok (exit 0)
⚬ [ ] leaf-0.1:G2: L0 evidence crate compiles to wasm32-unknown-unknown.
  ⚬ CHECK: cargo xtask wasm
  ⚬ EXPECT: wasm: ok (exit 0)
⚬ [ ] leaf-0.1:G3: craft.json validates against versioned schema (schema_version: 1, mode: "clean-room").
  ⚬ CHECK: python3 -c "import json; d=json.load(open('craft.json')); assert d['schema_version']==1 and d['mode']=='clean-room'; print('CRAFT_JSON_OK')"
  ⚬ EXPECT: CRAFT_JSON_OK

Wave 1 — leaf-1.1, leaf-1.2, leaf-1.3

⚬ [ ] leaf-1.1:G1: LocalFS refuses symlink cycles; Zip/Tar and .bcss snapshot round-trip tests pass.
  ⚬ CHECK: cargo test -p autocompare-vfs -- --nocapture
  ⚬ EXPECT: test result: ok. (exit 0)
⚬ [ ] leaf-1.2:G1: Myers + Patience diff replay, minimality, unimportant-text grammar suppression, and 3-way Take merge tests pass.
  ⚬ CHECK: cargo test -p autocompare-diff text merge grammar
  ⚬ EXPECT: test result: ok. (exit 0)
⚬ [ ] leaf-1.3:G1: Hex alignment, Table key-column/multi-sheet .xlsx, Picture Tolerance/Range/Blend, and Media tag/PCM totality tests pass (including hostile/truncated inputs).
  ⚬ CHECK: cargo test -p autocompare-diff hex table image audio media_tags
  ⚬ EXPECT: test result: ok. (exit 0)

Wave 2 — leaf-2.1, leaf-2.2, leaf-2.3

⚬ [ ] leaf-2.1:G1: Session, Workspace, and Settings JSON round-trip + schema migration tests pass.
  ⚬ CHECK: cargo test -p autocompare-sessions
  ⚬ EXPECT: test result: ok. (exit 0)
⚬ [ ] leaf-2.2:G1: HTML (with linked child reports), Text, XML, and Unified Patch report snapshots pass.
  ⚬ CHECK: cargo test -p autocompare-reports
  ⚬ EXPECT: test result: ok. (exit 0)
⚬ [ ] leaf-2.3:G1: Folder compare (over LocalFS + Zip + .bcss), Sync (Update Both/Mirror/dry-run/apply), Folder Merge, and cancellable JobRunner tests pass.
  ⚬ CHECK: cargo test -p autocompare-compare
  ⚬ EXPECT: test result: ok. (exit 0)

Wave 3 — leaf-3.1, leaf-3.2

⚬ [ ] leaf-3.1:G1: All 9 CLI subcommands + --report + bcomp VCS flags (-ro, -automerge, -mergeoutput) pass fixture integration tests and exit codes (0/1/2 and BC mode).
  ⚬ CHECK: cargo test -p autocompare-cli
  ⚬ EXPECT: test result: ok. (exit 0)
⚬ [ ] leaf-3.2:G1: Pure headless GUI view-model tests cover Home View, tabs, thumbnail gutter geometry, live edit/copy-right mutations, 3-way Take resolution, folder tree expand/collapse, and Dark/Light theme contrast.
  ⚬ CHECK: cargo test -p autocompare-gui
  ⚬ EXPECT: test result: ok. (exit 0)
⚬ [ ] leaf-3.2:G2: GUI supports --screenshot <path> clean-room verification contract (opens unfocused, renders fixture session, writes valid PNG, exits 0).
  ⚬ CHECK: cargo run -p autocompare-gui -- --screenshot checks/round-1/gui-smoke.png && python3 -c "from pathlib import Path; b=Path('checks/round-1/gui-smoke.png').read_bytes(); assert b[:8]==b'\x89PNG\r\n\x1a\n' and len(b)>1024; print('SCREENSHOT_OK')"
  ⚬ EXPECT: SCREENSHOT_OK

Wave 4 — root-4 (Check & Ready)

⚬ [ ] root-4:G1: Full workspace format, strict clippy, layer check, secret scan, and test suite pass.
  ⚬ CHECK: cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo xtask layers && cargo xtask secrets
  ⚬ EXPECT: secrets: ok (exit 0)
⚬ [ ] root-4:G2: Exhaustive release-mode diff property sweeps pass.
  ⚬ CHECK: cargo test -p autocompare-diff --release -- --ignored
  ⚬ EXPECT: test result: ok. (exit 0)
⚬ [ ] root-4:G3: Release binaries build cleanly for Apple Silicon (autocompare, autocompare-gui).
  ⚬ CHECK: cargo build --release --workspace
  ⚬ EXPECT: Finished \release` profile(exit0`)

7. craft-anything Check & Refinement Loop

Following docs/STATUS.md and docs/ROADMAP.md:

1. Console Parity Checks: Run deterministic fixture pairs across all 9 session types, comparing exit codes, hunk offsets, counts, and generated reports across two independent runs.
2. Visual & Layout Checks: Launch autocompare-gui --screenshot for Home View, Text Compare (Dark & Light), Text Merge, Folder Compare, Table Compare, and Picture Compare; inspect rendered PNGs in checks/round-N/ against clean-room layout criteria (pane alignment, gutter visibility, contrast, zero clipped text).
3. Bounded Refinement & Anti-Stall Guards: Cap refinement at 6 rounds (CLI default 2); halt and escalate to inbox/ if 2 consecutive rounds produce no file changes or 10 consecutive checks fail to run cleanly (docs/ROADMAP.md).

8. Risks & Mitigations

Risk	Impact	Mitigation
No sandbox access to /Applications/Beyond [invalid URL]	Cannot automatically run live macOS Accessibility or Screen Recording captures against the installed app	Stay strictly in clean-room mode using Scooter Software's official v5 documentation + existing docs/study.md (HIGH session-menu evidence); optionally ingest user-supplied screenshots into capture/
Subagent local-filesystem isolation	Subagents in this environment cannot directly edit /Users/ektasaini/autocompare	Use subagents for parallel design, specification, and code-generation drafts per disjoint leaf, while the main agent writes, compiles, and verifies all files locally with cargo
egui large-file text & hex rendering lag	Multi-megabyte files can stall immediate-mode UI frames	Virtualize all scrolling views (egui::ScrollArea::show_rows), run diffs/walks on background JobRunner threads, and enforce explicit edit-budget / file-size caps

Ready to Proceed — Two Confirmations

1. Where to write & build: Should I save this [invalid URL] to /Users/ektasaini/autocompare/PLAN.md and extend the existing autocompare v0.5.0 codebase toward full Beyond Compare 5 parity (v1.0.0), or create a brand-new workspace from scratch in a separate subfolder (e.g., /Users/ektasaini/autocompare/bc-replica/)?
2. Scope priority: Would you like me to execute all waves (Waves 0–4) end-to-end, or prioritize a specific milestone first (for example, Home View + interactive Text/Merge editing + Folder tree drill-down before Archives/Reports/VCS)?
9. Execution Decision Record (2026-11-20)

Decision (user-confirmed): extend existing autocompare v0.5.0 codebase toward
v1.0.0 in place. No new subfolder. Full waves end-to-end, harness-driven.

Active dispatch (disjoint OWNS, isolated logs in /tmp):

| Agent | OWNS | Task |
|---|---|---|
| release | target/release/**, checks/release-smoke/** | Rebuild release after HEAD, harness PASS, publish gh release v0.5.0 with both binaries |
| wave-A | crates/diff/src/text*, merge*, grammar* | Harness loop max 3 rounds, MS rust rules |
| wave-B | crates/vfs/**, crates/compare/src/folder*, sync* | Harness loop max 3 rounds, symlink-safe |
| wave-C | crates/gui/**, checks/gui-live/** | Live BC app outside-observation, screenshot smoke |

Probe strategy: no fixed timer. Detached agyx --print per agent, file-log
polling via fresh short execs, rolling dispatch on green. Orchestrator owns
PLAN.md, GATES-release.md, scripts/verify-*.mjs only.

## Dispatch update (2026-11-20) — user: 2 agents dono ke liye
- Wave-A DONE (report in /tmp/wave-A.log, commit 60409bd pushed).
- Wave-C retired before producing output (no checks/gui-live/ notes existed);
  superseded by two builder agents per user order.
- GUI-T (text): OWNS views.rs, copy.rs, take.rs, gutter.rs, viewmodel.rs, checks/gui-text/**.
- GUI-F (folder): OWNS tree.rs, home.rs, checks/gui-folder/** (viewmodel.rs read-only).
- Wave-B + local-verify continue unchanged.

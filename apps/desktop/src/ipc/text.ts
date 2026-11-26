/**
 * Text-compare IPC surface (Wave 1a).
 *
 * Mirrors the `serde` DTOs in
 * `src-tauri/src/commands/text.rs` (`rename_all = "camelCase"` on the Rust
 * side, so the JSON field names below match exactly). All calls go through
 * the typed wrapper in `./client`, which serves fixtures under
 * `VITE_MOCK_IPC=1`.
 *
 * Paged-fetch rule (locked): the full comparison stays in Rust memory.
 * The UI only receives `(start, limit)` windows via `textGetRows`, so
 * whole files are never serialized over IPC.
 */

/** Row kind wire values from Rust `RowKind`. */
export type TextRowKind = "equal" | "changed" | "leftOnly" | "rightOnly";

/** Side wire values from Rust `Side`. */
export type TextSide = "left" | "right";

/** Search direction wire values from Rust `Direction`. */
export type TextDirection = "up" | "down";

/** Diff algorithm wire values for Wave-C2a (`DiffAlgorithm` in `crates/diff`). */
export type TextAlgorithm = "myers" | "patience";

import { invoke } from "./client";

/** Applied ignore flags echoed by the backend (`appliedOptions` on the wire). */
export interface AppliedTextOptions {
  readonly ignoreWhitespace: boolean;
  readonly ignoreCase: boolean;
  /** Wave-L1: minor / unimportant difference toggle. */
  readonly minorUnimportant?: boolean;
  /** Wave-C2a: blank-line folding (`grammar::is_blank_line` mirror). */
  readonly ignoreBlankLines?: boolean;
  /** Wave-C2a: diff algorithm echo (`"myers"` default | `"patience"`). */
  readonly algorithm?: TextAlgorithm | string;
  /** Wave-L1: encoding name passthrough. */
  readonly encoding?: string;
  /** Wave-L1: eol line ending format passthrough. */
  readonly eol?: string;
  /** Wave-L1: line indices suppressed / marked ignored (row flag, not deletion). */
  readonly suppressedLines?: readonly number[];
}

/**
 * Wave-81: per-side file stats echoed by `text_compare` when the backend
 * stats the inputs. Both fields optional: a backend may know one without
 * the other, and old backends / the mock omit the object entirely.
 * `mtimeUnix` is whole seconds since the Unix epoch.
 */
export interface TextFileMeta {
  readonly bytes?: number;
  readonly mtimeUnix?: number;
}

/** Rust mirror: `commands::text::TextCompareDto` (summary only, no content). */
export interface TextCompareDto {
  readonly session: string;
  readonly leftPath: string;
  readonly rightPath: string;
  readonly leftLines: number;
  readonly rightLines: number;
  /**
   * Wave-81: per-side file meta echoed by the backend when it stats the
   * inputs (`{ bytes, mtimeUnix }`, both optional). Absent on old
   * backends / mock paths that predate stats — the view hides the meta
   * line honestly instead of fabricating values. Activates with the
   * wave-82 backend; until then the summary carries lines only.
   */
  readonly leftMeta?: TextFileMeta;
  readonly rightMeta?: TextFileMeta;
  /** Aligned row count; the `total` used by paging. */
  readonly rows: number;
  /** Number of ops in the underlying diff script. */
  readonly opCount: number;
  /** False when the diff engine fell back to a coarse script. */
  readonly exact: boolean;
  /** True when the cached overview hit the chunk cap. */
  readonly truncated: boolean;
  /**
   * Wave-66T: echo of the flags the backend actually applied.
   * Absent on old backends / mock paths that predate ignores.
   */
  readonly appliedOptions?: AppliedTextOptions;
  readonly encoding?: string;
  readonly eol?: string;
  readonly minorUnimportant?: boolean;
  readonly ignoreWhitespace?: boolean;
  readonly ignoreCase?: boolean;
  readonly suppressedLines?: readonly number[];
}

/** Ignore flags for [`textCompare`] (both default `false` = exact diff). */
export interface TextIgnoreOptions {
  readonly ignoreWhitespace?: boolean;
  readonly ignoreCase?: boolean;
  readonly minorUnimportant?: boolean;
  readonly encoding?: string;
  readonly eol?: string;
  readonly suppressedLines?: readonly number[];
  /** Wave-C2a: Patience vs Myers diff engine. */
  readonly algorithm?: TextAlgorithm | string;
  /** Wave-C2a: fold whitespace-only lines (both spellings accepted). */
  readonly ignoreBlankLines?: boolean;
  // snake_case spellings:
  readonly ignore_whitespace?: boolean;
  readonly ignore_case?: boolean;
  readonly minor_unimportant?: boolean;
  readonly suppressed_lines?: readonly number[];
  readonly ignore_blank_lines?: boolean;
}

/** Rust mirror: `commands::text::InlineSpanDto` (half-open char offsets). */
export interface InlineSpanDto {
  readonly aStart: number;
  readonly aEnd: number;
  readonly bStart: number;
  readonly bEnd: number;
}

/** Rust mirror: `commands::text::TextRowDto`. */
export interface TextRowDto {
  readonly index: number;
  readonly kind: TextRowKind;
  readonly left: string | null;
  readonly right: string | null;
  /** Word/char diff spans; only for `changed` rows with both sides. */
  readonly inline: readonly InlineSpanDto[];
  /** Wave-L1: per-line suppressed flag (row flag, not deletion). */
  readonly suppressed?: boolean;
}

/** Rust mirror: `commands::text::RowsPage`. */
export interface RowsPage {
  readonly session: string;
  readonly start: number;
  readonly total: number;
  readonly rows: readonly TextRowDto[];
}

/** Rust mirror: `commands::text::OverviewChunk`. */
export interface OverviewChunk {
  readonly start: number;
  readonly len: number;
  readonly kind: TextRowKind;
}

/** Rust mirror: `commands::text::Overview`. */
export interface Overview {
  readonly session: string;
  readonly total: number;
  readonly chunks: readonly OverviewChunk[];
  readonly truncated: boolean;
}

/** Rust mirror: `commands::text::Hit`. */
export interface Hit {
  readonly index: number;
  readonly side: TextSide;
  /** Character offset of the match inside `line`. */
  readonly column: number;
  readonly line: string;
}

/** Rust mirror: `commands::text::CopiedSection`. */
export interface CopiedSection {
  readonly session: string;
  readonly index: number;
  readonly side: TextSide;
  readonly start: number;
  readonly end: number;
  readonly text: string;
  readonly lineCount: number;
}

/** Rust mirror: `commands::text::SavedSide`. */
export interface SavedSide {
  readonly session: string;
  readonly side: TextSide;
  readonly path: string;
  readonly lines: number;
  readonly bytes: number;
}

/** Compare two text files; caches the result under a new session id.
 *
 * Wave-66T: optional `opts` extend the wire backward-compatibly.
 * Omitted (or both `false`) sends the legacy `{left, right}` shape for a
 * byte-identical diff. Passing a flag sends both spellings the Rust
 * command accepts (camelCase for the TextView probe path, snake_case
 * for the typed wrapper path); the backend echoes `appliedOptions`.
 */
export function textCompare(
  left: string,
  right: string,
  opts?: TextIgnoreOptions,
): Promise<TextCompareDto> {
  const ws = opts?.ignoreWhitespace === true || opts?.ignore_whitespace === true;
  const cs = opts?.ignoreCase === true || opts?.ignore_case === true;
  const minor = opts?.minorUnimportant === true || opts?.minor_unimportant === true;
  const enc = opts?.encoding !== undefined && opts.encoding !== "" ? opts.encoding : undefined;
  const eol = opts?.eol !== undefined && opts.eol !== "" ? opts.eol : undefined;
  const supp = opts?.suppressedLines ?? opts?.suppressed_lines;
  // Wave-C2a engine-ready options: Patience algorithm + blank-line folding.
  const rawAlgo = typeof opts?.algorithm === "string" ? opts.algorithm.trim().toLowerCase() : "";
  const algo = rawAlgo === "patience" || rawAlgo === "myers" ? rawAlgo : undefined;
  const blank = opts?.ignoreBlankLines === true || opts?.ignore_blank_lines === true;

  const hasOptions =
    ws ||
    cs ||
    minor ||
    enc !== undefined ||
    eol !== undefined ||
    (supp !== undefined && supp.length > 0) ||
    algo !== undefined ||
    blank;

  if (typeof window !== "undefined") {
    (window as unknown as Record<string, unknown>).__TEXT_IGNORE_LAST__ = {
      ignoreWhitespace: ws,
      ignoreCase: cs,
      minorUnimportant: minor,
      encoding: enc,
      eol,
      suppressedLines: supp,
      algorithm: algo,
      ignoreBlankLines: blank,
    };
  }
  if (!hasOptions) return invoke<TextCompareDto>("text_compare", { left, right });
  return invoke<TextCompareDto>("text_compare", {
    left,
    right,
    ignoreWhitespace: ws,
    ignoreCase: cs,
    ignore_whitespace: ws,
    ignore_case: cs,
    minorUnimportant: minor,
    minor_unimportant: minor,
    encoding: enc,
    eol,
    suppressedLines: supp,
    suppressed_lines: supp,
    algorithm: algo,
    ignoreBlankLines: blank,
    ignore_blank_lines: blank,
  });
}

/** Fetch one `(start, limit)` window of aligned rows. */
export function textGetRows(
  session: string,
  start: number,
  limit: number,
): Promise<RowsPage> {
  return invoke<RowsPage>("text_get_rows", { session, start, limit });
}

/** Fetch the coarse whole-comparison map for scrollbars/minimaps. */
export function textGetOverview(session: string): Promise<Overview> {
  return invoke<Overview>("text_get_overview", { session });
}

/** Search cached rows; resolves `null` when there is no match. */
export function textFind(
  session: string,
  query: string,
  direction: TextDirection,
): Promise<Hit | null> {
  return invoke<Hit | null>("text_find", { session, query, direction });
}

/** Get the enclosing changed section's text on one side for the clipboard. */
export function textCopySection(
  session: string,
  index: number,
  dir: TextSide,
): Promise<CopiedSection> {
  return invoke<CopiedSection>("text_copy_section", { session, index, dir });
}

/** Write one cached side to `path`; resolves a receipt. */
export function textSaveSide(
  session: string,
  side: TextSide,
  path: string,
): Promise<SavedSide> {
  return invoke<SavedSide>("text_save_side", { session, side, path });
}

/** Rust mirror: `commands::text::AppliedEdit` (patch + re-compare receipt). */
export interface AppliedEdit {
  readonly session: string;
  readonly index: number;
  readonly side: TextSide;
  readonly kind: TextRowKind;
  readonly rows: number;
}

/** Rust mirror: `commands::text::AppliedHunk` (atomic hunk patch receipt). */
export interface AppliedHunk {
  readonly session: string;
  readonly start: number;
  readonly end: number;
  readonly dir: TextSide;
  readonly applied: number;
  readonly total: number;
  readonly rows: number;
}

/**
 * Patch one cached cell and re-run the comparison (wave-66V). The
 * backend regroups hunks, inline spans and the overview; the UI
 * re-fetches its window afterwards. Orphan/out-of-range edits reject.
 */
export function textApplyEdit(
  session: string,
  index: number,
  side: TextSide,
  text: string,
): Promise<AppliedEdit> {
  return invoke<AppliedEdit>("text_apply_edit", { session, index, side, text });
}

/**
 * Patch an entire hunk range `[start, end]` atomically (BC parity Swarm-1).
 * Take-side wins, orphan side filled, no row dropped; single compare_text_lines
 * and single store insert returning an AppliedHunk receipt.
 */
export function textApplyHunk(
  session: string,
  start: number,
  end: number,
  dir: TextSide,
): Promise<AppliedHunk> {
  return invoke<AppliedHunk>("text_apply_hunk", { session, start, end, dir });
}

/**
 * Restore a snapshotted row list into the backend text session (Undo/Redo sync
 * so subsequent `textSaveSide` persists the exact undone/redone content).
 */
export function textRestoreRows(
  session: string,
  rows: readonly TextRowDto[],
): Promise<number> {
  return invoke<number>("text_restore_rows", { session, rows });
}

/** Lightweight stat for external file-change detection (reload prompt). */
export function statPaths(
  paths: readonly string[],
): Promise<readonly (TextFileMeta | null)[]> {
  return invoke<readonly (TextFileMeta | null)[]>("stat_paths", { paths });
}

// ---------------------------------------------------------------------------
// Wave 5B: bundled demo pair (input files ONLY, never mock rows).
//
// When the view mounts with empty paths it auto-loads one of these
// committed fixture pairs through the SAME real `text_compare` invoke the
// production path uses, so first paint shows real engine rows. Candidates
// cover the dev layouts (repo-root vs `apps/desktop` vs `src-tauri` CWDs,
// since the Rust backend resolves relative paths against its own CWD);
// the first pair the backend accepts wins, otherwise the view degrades to
// its empty state -- rows are never fabricated here.
// ---------------------------------------------------------------------------

/** Demo input pairs tried in order by [`textCompareDemo`]. */
export const TEXT_DEMO_CANDIDATES: readonly (readonly [string, string])[] = [
  ["tools/parity-harness/fixtures/t_left.txt", "tools/parity-harness/fixtures/t_right.txt"],
  ["../../tools/parity-harness/fixtures/t_left.txt", "../../tools/parity-harness/fixtures/t_right.txt"],
  [
    "../../../tools/parity-harness/fixtures/t_left.txt",
    "../../../tools/parity-harness/fixtures/t_right.txt",
  ],
];

/** A demo pair the backend accepted, with its real compare summary. */
export interface DemoCompare {
  readonly left: string;
  readonly right: string;
  readonly summary: TextCompareDto;
}

/** True when `value` is a usable `text_compare` summary (session + total). */
function isUsableSummary(value: unknown): value is TextCompareDto {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as Record<string, unknown>)["session"] === "string" &&
    typeof (value as Record<string, unknown>)["rows"] === "number"
  );
}

/**
 * Compare the first usable demo pair via the real `text_compare` command.
 * Resolves null when the backend accepts none of the candidates (e.g. a
 * packaged build without the dev fixtures on disk); never throws, never
 * returns placeholder rows.
 */
export async function textCompareDemo(
  pairs: readonly (readonly [string, string])[] = TEXT_DEMO_CANDIDATES,
): Promise<DemoCompare | null> {
  for (const [left, right] of pairs) {
    try {
      const summary = await textCompare(left, right);
      if (isUsableSummary(summary)) return { left, right, summary };
    } catch {
      // Try the next candidate pair.
    }
  }
  return null;
}

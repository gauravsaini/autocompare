/**
 * Table (CSV) compare IPC surface.
 *
 * Wave 1b. Rust mirrors live in
 * `src-tauri/src/commands/table.rs` (all `serde` `camelCase`).
 * Wrappers follow the `src/ipc/client.ts` pattern: arg keys must match
 * the Rust parameter names exactly (`session`, `start_row`, `limit`,
 * `start_col`, `col_limit`, `query`, `by_key`).
 */

import { invoke } from "./client";

/**
 * Per-side file metadata (byte length + modification timestamp).
 * Optional-tolerant for fixtures and older payloads.
 */
export interface TableFileMeta {
  readonly bytes: number;
  readonly mtimeUnix: number | null;
}

/** Rust mirror: `commands::table::TableSummary`. */
export interface TableSummary {
  readonly sessionId: string;
  readonly leftRows: number;
  readonly rightRows: number;
  readonly leftCols: number;
  readonly rightCols: number;
  readonly equal: boolean;
  readonly byKey: boolean;
  /** Total difference rows over the FULL tables (wave-69). */
  readonly diffRows: number;
  /** Effective key columns; empty means positional comparison. */
  readonly keyColumns: readonly number[];
  readonly leftMeta?: TableFileMeta | null;
  readonly rightMeta?: TableFileMeta | null;
}

/** Rust mirror: `commands::table::TablePage`. */
export interface TablePage {
  readonly sessionId: string;
  readonly startRow: number;
  readonly limit: number;
  readonly startCol: number;
  readonly colLimit: number;
  readonly headerL: readonly string[];
  readonly headerR: readonly string[];
  readonly rowsL: readonly (readonly string[])[];
  readonly rowsR: readonly (readonly string[])[];
  readonly totalRowsL: number;
  readonly totalRowsR: number;
  readonly totalCols: number;
  readonly truncated: boolean;
}

/** Rust mirror: `commands::table::TableHit`. */
export interface TableHit {
  readonly side: "left" | "right";
  readonly row: number;
  readonly col: number;
  readonly value: string;
}

/**
 * Ignore flags for [`tableCompare`] (both default `false` = exact cell
 * match).
 *
 * Wave-164: optional `opts` extend the wire backward-compatibly (mirrors
 * the `textCompare` options pattern). Omitted (or both `false`) keeps
 * the legacy 4-field wire byte-identical. Passing a flag sends both
 * camelCase and snake_case spellings, matching the Rust command which
 * accepts either; the backend applies them to every cell comparison
 * (positional + key mode).
 *
 * Wave-C2c: optional `delimiter` (single-char string; omitted/empty =
 * `,`) plus numeric tolerance under three spellings
 * (`numericTolerance` / `numeric_tolerance` / `epsilon`, first `Some`
 * wins on the Rust side). A set tolerance lets near-equal numbers
 * compare equal; unset keeps the legacy wire byte-identical.
 */
export interface TableIgnoreOptions {
  readonly ignoreCase?: boolean;
  readonly ignoreSpace?: boolean;
  // snake_case spellings:
  readonly ignore_case?: boolean;
  readonly ignore_space?: boolean;
  // Wave-C2c engine-ready table options:
  readonly delimiter?: string;
  readonly numericTolerance?: number;
  readonly numeric_tolerance?: number;
  readonly epsilon?: number;
}

function tablePicOptsWire(opts: TableIgnoreOptions): {
  delimiter: string | null;
  numericTolerance: number | null;
  numeric_tolerance: number | null;
  epsilon: number | null;
} {
  const rawDelim = typeof opts.delimiter === "string" ? opts.delimiter : "";
  const pickNum = (v: unknown): number | null =>
    typeof v === "number" && Number.isFinite(v) ? v : null;
  // First Some wins on the Rust side; mirror that here so the last
  // request hook shows exactly what the backend will honor.
  const tol =
    pickNum(opts.numericTolerance) ?? pickNum(opts.numeric_tolerance) ?? pickNum(opts.epsilon);
  return {
    delimiter: rawDelim === "" ? null : rawDelim,
    numericTolerance: tol,
    numeric_tolerance: tol,
    epsilon: tol,
  };
}

export function tableCompare(
  left: string,
  right: string,
  byKey: boolean,
  keyColumns?: readonly number[],
  opts?: TableIgnoreOptions,
): Promise<TableSummary> {
  // Wave-164: optional `opts` carry ignore flags (case/space). Omitted
  // (or both `false`) keeps the legacy 4-field wire byte-identical.
  const cs = opts?.ignoreCase === true || opts?.ignore_case === true;
  const ws = opts?.ignoreSpace === true || opts?.ignore_space === true;
  // Wave-C2c: delimiter + numeric tolerance ride the same optional
  // `opts` (engine-ready, still omitted when unset).
  const pic = opts === undefined ? null : tablePicOptsWire(opts);
  const hasPic =
    pic !== null && (pic.delimiter !== null || pic.numericTolerance !== null);
  if (typeof window !== "undefined") {
    (window as unknown as Record<string, unknown>)["__TABLE_IGNORE_LAST__"] = {
      ignoreCase: cs,
      ignoreSpace: ws,
    };
    (window as unknown as Record<string, unknown>)["__TABLE_PICOPTS_LAST__"] = {
      delimiter: pic?.delimiter ?? null,
      numericTolerance: pic?.numericTolerance ?? null,
    };
  }
  if (!cs && !ws && !hasPic) {
    return invoke<TableSummary>("table_compare", {
      left,
      right,
      by_key: byKey,
      key_columns: keyColumns === undefined ? [] : [...keyColumns],
    });
  }
  return invoke<TableSummary>("table_compare", {
    left,
    right,
    by_key: byKey,
    key_columns: keyColumns === undefined ? [] : [...keyColumns],
    ignoreCase: cs,
    ignore_case: cs,
    ignoreSpace: ws,
    ignore_space: ws,
    ...(hasPic
      ? {
          delimiter: pic.delimiter,
          numericTolerance: pic.numericTolerance,
          numeric_tolerance: pic.numeric_tolerance,
          epsilon: pic.epsilon,
        }
      : {}),
  });
}

/** Serves one bounded cell window over both session tables. */
export function tableGetPage(
  session: string,
  startRow: number,
  limit: number,
  startCol: number,
  colLimit: number,
): Promise<TablePage> {
  return invoke<TablePage>("table_get_page", {
    session,
    start_row: startRow,
    limit,
    start_col: startCol,
    col_limit: colLimit,
  });
}

/** Substring-searches both session tables (≤100 hits, left first). */
export function tableFind(session: string, query: string): Promise<TableHit[]> {
  return invoke<TableHit[]>("table_find", { session, query });
}

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
 * Opens a table session for two CSV files. `byKey` requests
 * key-aligned comparison; an empty `keyColumns` in the result means
 * positional fallback (no headers or no unique column).
 *
 * Wave-66M: `keyColumns` carries user-designated key columns. Omitted
 * (or empty) preserves the legacy wire path: the backend auto-detects
 * under `byKey` and falls back to positional otherwise.
 */
export function tableCompare(
  left: string,
  right: string,
  byKey: boolean,
  keyColumns?: readonly number[],
): Promise<TableSummary> {
  return invoke<TableSummary>("table_compare", {
    left,
    right,
    by_key: byKey,
    key_columns: keyColumns === undefined ? [] : [...keyColumns],
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

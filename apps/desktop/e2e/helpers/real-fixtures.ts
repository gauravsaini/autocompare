/**
 * Wave-4A typed accessors over the committed real-engine `dumped.json`.
 *
 * `dumped.json` is the stdout of
 * `src-tauri/examples/dump_fixtures.rs`: real `text_compare` /
 * `folder_compare` / `table_compare` engine output (row kinds, paging
 * totals, virtualization-relevant counts), NOT hand-written mocks.
 * The mock IPC branch (`src/ipc/client.ts`) serves these same arrays,
 * so the counts below are the ground truth the view specs assert against.
 */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

export type RealTextKind = "equal" | "changed" | "leftOnly" | "rightOnly";

export interface RealTextRow {
  readonly index: number;
  readonly kind: RealTextKind;
  readonly left: string | null;
  readonly right: string | null;
  readonly inline: readonly unknown[];
}

export type RealFolderStatus =
  | "same"
  | "different"
  | "newerLeft"
  | "newerRight"
  | "leftOnly"
  | "rightOnly";

export interface RealFolderSide {
  readonly size: number;
  readonly modifiedMs: number | null;
}

export interface RealFolderNode {
  readonly relPath: string;
  readonly isDir: boolean;
  readonly left: RealFolderSide | null;
  readonly right: RealFolderSide | null;
  readonly status: RealFolderStatus;
}

export interface RealTableCells {
  readonly header: readonly string[];
  readonly headerR: readonly string[];
  readonly rowsL: ReadonlyArray<readonly string[]>;
  readonly rowsR: ReadonlyArray<readonly string[]>;
  readonly totalRowsL: number;
  readonly totalRowsR: number;
  readonly totalCols: number;
  readonly leftCols: number;
  readonly rightCols: number;
  readonly equal: boolean;
}

interface DumpedFixture {
  readonly textRows: readonly RealTextRow[];
  readonly folderNodes: readonly RealFolderNode[];
  readonly tableCells: RealTableCells;
}

const data: DumpedFixture = JSON.parse(
  readFileSync(join(dirname(fileURLToPath(import.meta.url)), "dumped.json"), "utf8"),
) as unknown as DumpedFixture;

/** All engine-produced text rows (index order, kinds mixed). */
export const TEXT_ROWS: readonly RealTextRow[] = data.textRows;

/** All engine-produced folder nodes (relPath sort order). */
export const FOLDER_NODES: readonly RealFolderNode[] = data.folderNodes;

/** Full engine-produced table grid (both sides). */
export const TABLE_CELLS: RealTableCells = data.tableCells;

/** Ground-truth counts every real-data assertion is derived from. */
export const COUNTS = {
  textRows: TEXT_ROWS.length,
  folderNodes: FOLDER_NODES.length,
  tableRowsL: TABLE_CELLS.totalRowsL,
  tableRowsR: TABLE_CELLS.totalRowsR,
  tableCols: TABLE_CELLS.totalCols,
  tableCells:
    TABLE_CELLS.totalRowsL * TABLE_CELLS.totalCols +
    TABLE_CELLS.totalRowsR * TABLE_CELLS.totalCols,
} as const;

/** Engine row-kind histogram (proves del/ins/chg runs exist). */
export const TEXT_KINDS: Readonly<Record<RealTextKind, number>> = TEXT_ROWS.reduce(
  (acc, row) => ({ ...acc, [row.kind]: acc[row.kind] + 1 }),
  { equal: 0, changed: 0, leftOnly: 0, rightOnly: 0 } as Record<RealTextKind, number>,
);

/** Engine folder-status histogram (proves one-sided + changed nodes). */
export const FOLDER_STATUSES: Readonly<Record<string, number>> = FOLDER_NODES.reduce(
  (acc: Record<string, number>, node) => ({
    ...acc,
    [node.status]: (acc[node.status] ?? 0) + 1,
  }),
  {},
);

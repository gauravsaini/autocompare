/**
 * Folder-compare IPC surface (Wave 1a).
 *
 * Mirrors the `serde` DTOs in
 * `src-tauri/src/commands/folder.rs` (`rename_all = "camelCase"` on the
 * Rust side, so the JSON field names below match exactly). All calls go
 * through the typed wrapper in `./client`, which serves fixtures under
 * `VITE_MOCK_IPC=1`.
 *
 * Paged-fetch rule (locked): the full folder comparison stays in Rust
 * memory. The UI only receives `(start, limit)` windows via
 * `folderGetTree`, so whole trees are never serialized over IPC.
 */

/** Verdict wire values from Rust `FolderStatusDto`. */
export type FolderStatusDto =
  | "same"
  | "different"
  | "newerLeft"
  | "newerRight"
  | "leftOnly"
  | "rightOnly";

/** Filter wire values accepted by `folderGetTree`. */
export type FolderFilter =
  | "all"
  | "changed"
  | FolderStatusDto;

import { invoke } from "./client";

/** Rust mirror: `commands::folder::FolderCompareDto` (summary, no entries). */
export interface FolderCompareDto {
  readonly session: string;
  readonly leftRoot: string;
  readonly rightRoot: string;
  readonly total: number;
  readonly truncated: boolean;
}

/** Rust mirror: `commands::folder::NodeInfoDto`. */
export interface NodeInfoDto {
  readonly size: number;
  /** Millis since the Unix epoch; `null` when unavailable. */
  readonly modifiedMs: number | null;
}

/** Rust mirror: `commands::folder::FolderEntryDto`. */
export interface FolderEntryDto {
  readonly relPath: string;
  readonly isDir: boolean;
  readonly left: NodeInfoDto | null;
  readonly right: NodeInfoDto | null;
  readonly status: FolderStatusDto;
}

/** Rust mirror: `commands::folder::TreePage`. */
export interface TreePage {
  readonly session: string;
  readonly start: number;
  readonly total: number;
  readonly filter: FolderFilter;
  readonly entries: readonly FolderEntryDto[];
}

/** Rust mirror: `commands::folder::FolderCancelDto`. */
export interface FolderCancelDto {
  readonly session: string;
  readonly cancelled: boolean;
}

/** Compare two folder trees; caches the result under a new snapshot id.
 *
 * `masks` is an optional BC-Rules style file-mask list (semicolon
 * separated, `*`/`?` wildcards, `-` prefix excludes). `null`/omitted
 * disables mask filtering; masks apply at compare time, before any
 * `filter` used by `folderGetTree`.
 */
export function folderCompare(
  left: string,
  right: string,
  masks?: string | null,
): Promise<FolderCompareDto> {
  return invoke<FolderCompareDto>("folder_compare", { left, right, masks: masks ?? null });
}

/** Fetch one `(start, limit)` window of snapshot entries. */
export function folderGetTree(
  session: string,
  start: number,
  limit: number,
  filter: FolderFilter,
): Promise<TreePage> {
  return invoke<TreePage>("folder_get_tree", { session, start, limit, filter });
}

/** Cancel an in-flight compare and/or evict its cached snapshot. */
export function folderCancel(session: string): Promise<FolderCancelDto> {
  return invoke<FolderCancelDto>("folder_cancel", { session });
}

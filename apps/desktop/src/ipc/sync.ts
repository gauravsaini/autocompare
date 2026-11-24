/**
 * Folder sync IPC (Wave 1c).
 *
 * Rust mirrors (`apps/desktop/src-tauri/src/commands/sync.rs`, all `serde`):
 * - `SyncApplyReport` (`rename_all = "camelCase"`).
 * - Engine pass-throughs keep engine serde names verbatim:
 *   `SyncOptions` (`direction | prune`), `SyncDirection`
 *   (`LeftToRight | RightToLeft | NewerWins`), `SyncPlan`
 *   (`steps | skipped | truncated`), `SyncStep`
 *   (`rel_path | is_dir | action`), `SyncAction`
 *   (`CopyToRight | CopyToLeft | DeleteRight | DeleteLeft`).
 *
 * Commands (registered by Wave-1R; names are the Rust function names):
 * `sync_plan`, `sync_apply`.
 */

import { invoke } from "./client";

/** Which side wins. Rust: `autocompare_compare::SyncDirection`. */
export type SyncDirection = "LeftToRight" | "RightToLeft" | "NewerWins";

/** Sync options; all fields optional here, engine defaults apply when omitted. */
export interface SyncOptions {
  readonly direction?: SyncDirection;
  readonly prune?: boolean;
}

/** One planned change. Rust: `autocompare_compare::SyncStep`. */
export type SyncAction = "CopyToRight" | "CopyToLeft" | "DeleteRight" | "DeleteLeft";

export interface SyncStep {
  readonly rel_path: string;
  readonly is_dir: boolean;
  readonly action: SyncAction;
}

/** Read-only sync plan. Rust: `autocompare_compare::SyncPlan`. */
export interface SyncPlan {
  readonly steps: readonly SyncStep[];
  readonly skipped: ReadonlyArray<readonly [string, string]>;
  readonly truncated: boolean;
}

/**
 * Outcome of `sync_apply`. Rust: `sync::SyncApplyReport`.
 * Dry runs leave `copied`/`deleted` at zero and fill the `planned_*`
 * counters; real runs do the opposite.
 */
export interface SyncApplyReport {
  readonly dryRun: boolean;
  readonly copied: number;
  readonly deleted: number;
  readonly plannedCopies: number;
  readonly plannedDeletes: number;
  readonly failed: ReadonlyArray<readonly [string, string]>;
}

/**
 * Build a sync plan (read-only). `opts` mirrors the engine
 * `SyncOptions` serde names; omitted fields fall back to the engine
 * default (left-to-right, no pruning).
 */
export function syncPlan(
  left: string,
  right: string,
  opts?: SyncOptions,
): Promise<SyncPlan> {
  return invoke<SyncPlan>("sync_plan", { left, right, opts: opts ?? null });
}

/**
 * Apply a sync plan. `dryRun = true` only inspects the plan and never
 * touches the filesystem.
 */
export function syncApply(
  left: string,
  right: string,
  plan: SyncPlan,
  dryRun: boolean,
): Promise<SyncApplyReport> {
  return invoke<SyncApplyReport>("sync_apply", {
    left,
    right,
    plan,
    dry_run: dryRun,
  });
}

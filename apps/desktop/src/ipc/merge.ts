/**
 * Three-way merge IPC (Wave 1c).
 *
 * Rust mirrors (`apps/desktop/src-tauri/src/commands/merge.rs`, all `serde`):
 * - `TextConflict`, `TextMergePlan`, `TextChoice`, `TextResolution`,
 *   `TextMergeApplyReport`, `FolderMergeApplyReport` (`rename_all = "camelCase"`).
 * - Engine pass-throughs keep engine serde names verbatim:
 *   `TreeMergePlan` (`steps | conflicts | skipped`), `TreeMergeStep`
 *   (`rel_path | action | merged`), `TreeMergeAction`
 *   (`TakeLeft | TakeRight | MergedText | Conflict | MakeDir`), and
 *   `TreeMergeReport` (`written | dirs_created | failed`).
 *
 * Commands (registered by Wave-1R; names are the Rust function names):
 * `merge_text_plan`, `merge_text_apply`, `merge_folder_plan`,
 * `merge_folder_apply`.
 */

import { invoke } from "./client";

/** One conflicting hunk. Rust: `merge::TextConflict`. */
export interface TextConflict {
  readonly id: number;
  readonly base: readonly string[];
  readonly left: readonly string[];
  readonly right: readonly string[];
}

/** Read-only text merge plan. Rust: `merge::TextMergePlan`. */
export interface TextMergePlan {
  readonly conflicts: readonly TextConflict[];
  readonly conflictCount: number;
  readonly cleanBlockCount: number;
  readonly hasConflicts: boolean;
}

/**
 * Per-conflict resolution. Rust: `merge::TextChoice`
 * (externally tagged: units are bare strings, custom is `{ custom: [...] }`).
 */
export type TextChoice =
  | "base"
  | "left"
  | "right"
  | "bothLeftFirst"
  | "bothRightFirst"
  | { readonly custom: readonly string[] };

/** Resolution for one conflict id. Rust: `merge::TextResolution`. */
export interface TextResolution {
  readonly conflictId: number;
  readonly choice: TextChoice;
}

/** Outcome of `merge_text_apply`. Rust: `merge::TextMergeApplyReport`. */
export interface TextMergeApplyReport {
  readonly outPath: string;
  readonly writtenLines: number;
  readonly conflictCount: number;
  readonly resolvedCount: number;
  readonly unresolvedCount: number;
  readonly hasConflictMarkers: boolean;
}

/** Engine step action. Rust: `autocompare_compare::TreeMergeAction`. */
export type TreeMergeAction =
  | "TakeLeft"
  | "TakeRight"
  | "MergedText"
  | "Conflict"
  | "MakeDir";

/** Engine merge step (engine serde names). */
export interface TreeMergeStep {
  readonly rel_path: string;
  readonly action: TreeMergeAction;
  readonly merged: readonly string[] | null;
}

/** Engine merge plan (engine serde names). */
export interface TreeMergePlan {
  readonly steps: readonly TreeMergeStep[];
  readonly conflicts: readonly string[];
  readonly skipped: ReadonlyArray<readonly [string, string]>;
}

/** Outcome of `merge_folder_apply`. Rust: `merge::FolderMergeApplyReport`. */
export interface FolderMergeApplyReport {
  readonly outPath: string;
  readonly written: number;
  readonly dirsCreated: number;
  readonly conflictCount: number;
  readonly conflictPaths: readonly string[];
  readonly failed: ReadonlyArray<readonly [string, string]>;
}

/** Read-only 3-way text merge plan (conflict/hunk list, no file bytes). */
export function mergeTextPlan(
  base: string,
  left: string,
  right: string,
): Promise<TextMergePlan> {
  return invoke<TextMergePlan>("merge_text_plan", { base, left, right });
}

/**
 * Apply a 3-way text merge. `outPath` is required server-side; when
 * omitted here the backend refuses rather than touching an input.
 */
export function mergeTextApply(
  base: string,
  left: string,
  right: string,
  resolutions: readonly TextResolution[],
  outPath?: string,
): Promise<TextMergeApplyReport> {
  return invoke<TextMergeApplyReport>("merge_text_apply", {
    base,
    left,
    right,
    resolutions,
    out_path: outPath ?? null,
  });
}

/** Read-only 3-way folder merge plan. */
export function mergeFolderPlan(
  base: string,
  left: string,
  right: string,
): Promise<TreeMergePlan> {
  return invoke<TreeMergePlan>("merge_folder_plan", { base, left, right });
}

/** Apply a folder merge plan into `outPath` (required server-side). */
export function mergeFolderApply(
  left: string,
  right: string,
  plan: TreeMergePlan,
  outPath?: string,
): Promise<FolderMergeApplyReport> {
  return invoke<FolderMergeApplyReport>("merge_folder_apply", {
    left,
    right,
    plan,
    out_path: outPath ?? null,
  });
}

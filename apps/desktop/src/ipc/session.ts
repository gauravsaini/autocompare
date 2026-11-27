/**
 * Session/settings/report IPC surface (Wave 1d).
 *
 * Mirrors the serde passthrough DTOs in
 * `src-tauri/src/commands/session.rs`, which in turn mirror
 * `autocompare_sessions::{SessionDocument, SessionTab, AppSettings}`
 * field-for-field. Unlike the other IPC modules these shapes keep the
 * engine's `snake_case` JSON keys, so the field names below match the
 * engine exactly and survive the Wave 1R engine swap untouched. All
 * calls go through the typed wrapper in `./client`, which serves
 * fixtures under `VITE_MOCK_IPC=1`.
 *
 * Storage rule (locked): sessions and settings persist under the system
 * temp directory (`<tmp>/autocompare-desktop/`) -- never HOME dotfiles.
 * Report exports are likewise confined to the temp directory.
 *
 * Wave 1R handoff: the command names below (`session_list`,
 * `session_load`, `session_save`, `settings_get`, `settings_set`,
 * `report_export`) must be registered in the `lib.rs`
 * `generate_handler![...]` table for production; under
 * `VITE_MOCK_IPC=1` the client fixture below answers instead.
 */

import { invoke } from "./client";

/** Rust mirror: `commands::session::SessionTab` (engine passthrough). */
export interface SessionTab {
  readonly id: string;
  readonly title: string;
  readonly session_type: string;
  readonly left_path: string | null;
  readonly right_path: string | null;
  readonly base_path: string | null;
  readonly output_path: string | null;
  readonly read_only: boolean;
  readonly locked: boolean;
  readonly options: Record<string, string>;
}

/** Rust mirror: `commands::session::SessionDocument` (engine passthrough). */
export interface SessionDocument {
  readonly schema_version: number;
  readonly tabs: readonly SessionTab[];
  readonly active_tab_index: number;
  readonly name: string | null;
  readonly locked: boolean;
}

/** Rust mirror: `commands::session::GrammarPreset` (engine passthrough). */
export interface GrammarPreset {
  readonly name: string;
  readonly extensions: readonly string[];
  readonly line_comment_prefix: string | null;
  readonly block_comment_start: string | null;
  readonly block_comment_end: string | null;
  readonly ignore_case: boolean;
}

/** Rust mirror: `commands::session::AppSettings` (engine passthrough). */
export interface AppSettings {
  readonly schema_version: number;
  readonly theme: string;
  readonly keymap: string;
  readonly grammar_preset_name: string;
  readonly grammar_presets: readonly GrammarPreset[];
  readonly font_family: string | null;
  readonly font_size: number | null;
  readonly tab_size: number | null;
  readonly auto_save: boolean;
  readonly show_line_numbers: boolean;
  readonly locked: boolean;
  readonly extra: Record<string, string>;
}

/** Rust mirror: `commands::session::SessionEntry`. */
export interface SessionEntry {
  readonly id: string;
  readonly name: string | null;
  readonly tabs: number;
  readonly locked: boolean;
}

/** Report formats accepted by `reportExport`. */
export type ReportFormat = "html" | "patch" | "md" | "csv" | "json";

/** Lists saved sessions (directory order, never an error). */
export function sessionList(): Promise<SessionEntry[]> {
  return invoke<SessionEntry[]>("session_list");
}

/** Loads a saved session by id. */
export function sessionLoad(id: string): Promise<SessionDocument> {
  return invoke<SessionDocument>("session_load", { id });
}

/**
 * Saves a session descriptor under a minted id; resolves the id.
 * Locked descriptors are refused.
 */
export function sessionSave(descriptor: SessionDocument): Promise<string> {
  return invoke<string>("session_save", { descriptor });
}

/**
 * Deletes a saved session by id; resolves `true` when a stored document
 * was removed, `false` when nothing was stored under the id (idempotent
 * delete). Rust mirror: `commands::session::session_delete`.
 */
export function deleteSession(id: string): Promise<boolean> {
  return invoke<boolean>("session_delete", { id });
}

/** Loads persisted settings, or defaults when nothing is stored yet. */
export function settingsGet(): Promise<AppSettings> {
  return invoke<AppSettings>("settings_get");
}

/**
 * Applies a JSON settings patch. Unknown keys and mistyped values are
 * rejected server-side; resolves the stored settings.
 */
export function settingsSet(patch: Record<string, unknown>): Promise<AppSettings> {
  return invoke<AppSettings>("settings_set", { patch });
}

/**
 * Renders a session descriptor (or pre-rendered diff report body) to `format`
 * and writes it to `outPath` (confined to the temp directory); resolves the
 * written path.
 */
export function reportExport(
  session: SessionDocument,
  format: ReportFormat,
  outPath: string,
  renderedBody?: string,
): Promise<string> {
  return invoke<string>("report_export", {
    session,
    format,
    out_path: outPath,
    rendered_body: renderedBody ?? null,
  });
}

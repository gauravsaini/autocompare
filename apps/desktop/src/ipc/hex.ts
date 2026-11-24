/**
 * Hex (binary) compare IPC surface.
 *
 * Wave 1b. Rust mirrors live in
 * `src-tauri/src/commands/hex.rs` (all `serde` `camelCase`).
 * Wrappers follow the `src/ipc/client.ts` pattern: arg keys must match
 * the Rust parameter names exactly (`session`, `offset`, `len`,
 * `needle_hex`, `from_offset`).
 */

import { invoke } from "./client";

/** Rust mirror: `commands::hex::HexSummary`. */
export interface HexSummary {
  readonly sessionId: string;
  readonly leftSize: number;
  readonly rightSize: number;
  readonly equal: boolean;
  /** Byte offset of the first difference, or null when identical. */
  readonly firstDiff: number | null;
}

/** Rust mirror: `commands::hex::HexPage`. */
export interface HexPage {
  readonly sessionId: string;
  readonly offset: number;
  /** Effective window length (server-clamped to 4096). */
  readonly len: number;
  /** Left-file bytes; empty past EOF. */
  readonly left: readonly number[];
  /** Right-file bytes; empty past EOF. */
  readonly right: readonly number[];
  /** max(leftSize, rightSize); page over [0, total). */
  readonly total: number;
  readonly truncated: boolean;
}

/** Opens a hex session for two files. Never returns file bytes. */
export function hexCompare(left: string, right: string): Promise<HexSummary> {
  return invoke<HexSummary>("hex_compare", { left, right });
}

/** Serves one bounded byte window over both session files. */
export function hexGetPage(
  session: string,
  offset: number,
  len: number,
): Promise<HexPage> {
  return invoke<HexPage>("hex_get_page", { session, offset, len });
}

/**
 * First offset of `needleHex` (whitespace-tolerant hex) in the
 * session's left file at or after `fromOffset`, or null when absent.
 */
export function hexFind(
  session: string,
  needleHex: string,
  fromOffset: number,
): Promise<number | null> {
  return invoke<number | null>("hex_find", {
    session,
    needle_hex: needleHex,
    from_offset: fromOffset,
  });
}

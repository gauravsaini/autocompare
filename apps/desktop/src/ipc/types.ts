/**
 * Locked IPC data-transfer objects (front-end side).
 *
 * Wave 0: empty placeholders only. Each interface below names the Rust
 * `serde` type it will mirror; Wave 1 locks the field lists and `rename_all`
 * conventions when `commands/*` lands. Nothing here is constructed yet --
 * `client.ts` returns fixtures for these shapes under `VITE_MOCK_IPC=1`.
 *
 * Rust mirrors (all `serde` Serialize/Deserialize):
 * - `SessionId`        -> autocompare-desktop `state::SessionId`
 * - `CachedSession`    -> autocompare-desktop `state::CachedSession`
 * - `PageRequest`      -> `{ session_id, offset, limit }` (Wave 1 command arg)
 * - `PageResponse<T>`  -> `{ items, offset, total, truncated }` (Wave 1 return)
 */

/** Opaque session id. Rust mirror: `state::SessionId` (newtype over string). */
export type SessionId = string;

export interface CachedSession {
  /** Rust: `state::CachedSession`. Fields locked in Wave 1. */
  readonly id: SessionId;
  readonly kind: string;
  readonly name: string;
  readonly locked: boolean;
  readonly pageCount: number;
}

export interface PageRequest {
  readonly sessionId: SessionId;
  readonly offset: number;
  readonly limit: number;
}

export interface PageResponse<T> {
  readonly items: readonly T[];
  readonly offset: number;
  readonly total: number;
  readonly truncated: boolean;
}

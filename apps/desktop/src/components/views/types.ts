/**
 * Shared view prop types + runtime guards (Wave 2B).
 *
 * All data views call IPC through `src/ipc/client.ts`, which resolves empty
 * fixtures (`[]`) under `VITE_MOCK_IPC=1`. Every view must therefore validate
 * payload shapes at runtime and degrade to loading/error/empty states --
 * never crash on a missing file or a mock fixture.
 */

export interface ViewPaths {
  readonly leftPath: string;
  readonly rightPath: string;
  /** Reserved for a caller-supplied session (Wave 2R may reuse it). */
  readonly sessionId?: string;
}

/** Render any thrown value as a plain string for inline display. */
export function errString(e: unknown): string {
  if (e instanceof Error) return e.message;
  return String(e);
}

/** Narrow an unknown IPC payload to a field-readable record. */
export function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null;
}

/** Extract an array field, or `[]` when the payload is a mock/empty fixture. */
export function arrayField<T>(v: Record<string, unknown>, key: string): readonly T[] {
  const raw = v[key];
  return Array.isArray(raw) ? (raw as readonly T[]) : [];
}

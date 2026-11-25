/**
 * Wave-C3a: namespaced per-view compare-option persistence (view-state).
 *
 * Har view ke compare options (text/table/folder) browser reload ke baad
 * survive karte hain: `localStorage` key `ac.viewstate.v1.<view>`, JSON,
 * try/catch. Corrupt ya missing storage = defaults (koi throw nahi).
 *
 * Load/save helpers ONLY — koi UI nahi, koi nayi dependency nahi. Views
 * apne option state ko lazy-init (`useState(() => loadViewState(...))`)
 * se restore karte hain aur change par `saveViewState` se persist karte
 * hain; layout/testid/IPC untouched.
 */

const KEY_PREFIX = "ac.viewstate.v1.";

function storageKey(view: string): string {
  return `${KEY_PREFIX}${view}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

/**
 * Stored view-state padho. Missing key / corrupt JSON / galat shape =
 * `defaults`. Partial object = known keys merge (galat type wali value
 * ke liye us key ka default rehta hai), taaki aadha-corrupt storage bhi
 * kabhi crash na kare.
 */
export function loadViewState<T extends Record<string, unknown>>(
  view: string,
  defaults: T,
): T {
  try {
    if (typeof window === "undefined" || window.localStorage == null) return defaults;
    const raw = window.localStorage.getItem(storageKey(view));
    if (raw === null) return defaults;
    const parsed: unknown = JSON.parse(raw);
    if (!isRecord(parsed)) return defaults;
    const next: Record<string, unknown> = { ...(defaults as Record<string, unknown>) };
    for (const key of Object.keys(defaults)) {
      const fallback = (defaults as Record<string, unknown>)[key];
      const candidate = parsed[key];
      if (typeof fallback === "boolean" && typeof candidate === "boolean") {
        next[key] = candidate;
      } else if (typeof fallback === "string" && typeof candidate === "string") {
        next[key] = candidate;
      }
    }
    return next as T;
  } catch {
    return defaults;
  }
}

/** View-state likho. Quota/private-mode failure = silent best-effort. */
export function saveViewState(view: string, value: Record<string, unknown>): void {
  try {
    if (typeof window === "undefined" || window.localStorage == null) return;
    window.localStorage.setItem(storageKey(view), JSON.stringify(value));
  } catch {
    // Best-effort only (quota / private mode).
  }
}

/** Ek view ka stored state mito (tests / reset path ke liye). */
export function clearViewState(view: string): void {
  try {
    if (typeof window === "undefined" || window.localStorage == null) return;
    window.localStorage.removeItem(storageKey(view));
  } catch {
    // Best-effort only.
  }
}

/**
 * Wave-161: ViewCommandBus — ONE routing vocabulary for the unified top
 * Toolbar (Beyond Compare parity, frontend slice only).
 *
 * The top `Toolbar` (App `toolbarActions`) and the in-view `TextToolbar`
 * prev/next buttons call the SAME bus: App `handleViewCommand` /
 * `handleViewFilter` / `handleMinorToggle` are the single dispatch
 * points, and views follow through these window events (views already
 * listen; no new IPC, no Rust change):
 * - `autocompare:view-command` `{ cmd, viewKind, dir? }` — prev/next
 *   follow-scroll (text), conflict nav (merge);
 * - `autocompare:view-filter` `{ filter, viewKind }` — All/Diffs/Same/
 *   Context for the active kind;
 * - `autocompare:view-minor` `{ on, viewKind }` — Minor toggle, applied
 *   by TextView through the EXISTING text IPC ignore options
 *   (`ignoreWhitespace` + `ignoreCase`), no engine change.
 *
 * Event names are spelled byte-identical to the L2 strings so every
 * existing listener keeps working unmodified.
 *
 * Wave-180: this bus is the SINGLE routing vocabulary for the 14-action
 * unified top Toolbar (full contract:
 * `checks/topbar-180/CONTRACT.md`). Rules/Format/Minor/filters keep
 * their own events below; Home/Sessions/Reload stay App-owned (tabs,
 * SessionManager, kind-matched compare — no per-view listener needed).
 * Every view subscribes to `VIEW_COMMAND_EVENT` kind-scoped and no-ops
 * honestly where the command does not apply (see contract table).
 */

/** Bus-routed nav/copy/swap commands (top bar + TextToolbar share them). */
export type ViewCommand = "prev" | "next" | "copy" | "copy-right" | "swap";

/** Bus-routed view filters (top bar All/Diffs/Same/Context). */
export type ViewFilterKind = "all" | "diffs" | "same" | "context";

/** Window event carrying a {@link ViewCommand}. */
export const VIEW_COMMAND_EVENT = "autocompare:view-command";

/** Window event carrying a {@link ViewFilterKind}. */
export const VIEW_FILTER_EVENT = "autocompare:view-filter";

/** Window event carrying the Minor toggle (`{ on: boolean }`). */
export const VIEW_MINOR_EVENT = "autocompare:view-minor";

/**
 * Wave-180: kind-scope guard shared by every per-view `VIEW_COMMAND_EVENT`
 * listener. Returns true when the event targets `ownKind` (exact match)
 * or is unkinded/blank (broadcast — the view applies it when it owns the
 * command, no-ops otherwise). Foreign kinds and malformed payloads
 * return false so text/merge nav never leaks across views.
 */
export function isBusCommandForKind(detail: unknown, ownKind: string): boolean {
  if (typeof detail !== "object" || detail === null) return true;
  const kind = (detail as { viewKind?: unknown }).viewKind;
  if (typeof kind !== "string" || kind === "") return true;
  return kind === ownKind;
}

/**
 * Dispatch a view command on the bus. `viewKind` scopes which view owns
 * it (listeners ignore foreign kinds); omitted = broadcast. `extra`
 * carries command-specific fields (e.g. merge copy `dir`).
 */
export function dispatchViewCommand(
  cmd: ViewCommand,
  viewKind?: string,
  extra?: Record<string, unknown>,
): void {
  window.dispatchEvent(
    new CustomEvent(VIEW_COMMAND_EVENT, { detail: { cmd, viewKind, ...extra } }),
  );
}

/**
 * Dispatch a view filter on the bus. Views listen already (or park
 * honestly); the dispatch + status note never throw.
 */
export function dispatchViewFilter(filter: ViewFilterKind, viewKind?: string | null): void {
  window.dispatchEvent(
    new CustomEvent(VIEW_FILTER_EVENT, { detail: { filter, viewKind: viewKind ?? null } }),
  );
}

/**
 * Dispatch the Minor toggle on the bus. TextView applies it through the
 * existing text IPC ignore options; every other kind keeps its honest
 * parked toolbar state (no listener, no crash).
 */
export function dispatchViewMinor(on: boolean, viewKind?: string): void {
  window.dispatchEvent(
    new CustomEvent(VIEW_MINOR_EVENT, { detail: { on, viewKind } }),
  );
}

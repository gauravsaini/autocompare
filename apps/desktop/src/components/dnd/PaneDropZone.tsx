import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  coercePickedPath,
  dedupTauriDropPaths,
  extractDroppedPath,
  extractDroppedPaths,
  firstTauriDropPath,
  hasTauriDropBridge,
  hoveredPaneHalf,
  isFolderViewActive,
  isTauriDropOnPathBar,
  markDropRecompare,
  resolveStripSideFromRect,
  resolveTauriDropCssSide,
} from "./dropPaths";
import { pickOpenDialog } from "../session/useSession";

/**
 * Wave-117: per-pane file-drop zone (Tauri BC parity, verify-first).
 *
 * Beyond Compare lets the user drag a file from Finder/Explorer onto the
 * left or right pane: the drop sets that side's path and immediately
 * re-compares. This component wraps the App view area and routes drops by
 * horizontal half — left half sets left, right half sets right — through
 * `onDropPath`, which owns `tabPaths` + the auto-compare (same IPC path
 * as the toolbar Compare, no new IPC here).
 *
 * Channels (priority order):
 * - Native Tauri window drag-drop event (`@tauri-apps/api/webview`
 *   `onDragDropEvent`, production shell only): real absolute paths.
 * - HTML5 `onDrop`/`onDragOver` on the wrapper (browser + mock):
 *   `extractDroppedPath` over `DataTransfer`.
 *
 * Chrome-budget contract: the zone adds ZERO layout — the wrapper keeps
 * the caller's className, and the affordance halves are
 * `pointer-events-none` absolute overlays (outline only, `outline` never
 * shifts layout). Halves carry `data-panedrop` so synthetic test drops on a
 * half resolve the side even with `clientX: 0`; Wave-225 promotes them to
 * named sections (`panedrop/section-left|right`, `data-overlay` so the
 * shared `assertNoOverlap` helper skips them) for BC two-pane parity.
 */

export type PaneSide = "left" | "right";

/** Wave-159: honest busy note for drops attempted while comparing. */
export const PANE_BUSY_NOTE = "busy — compare running, drop paused";

export interface PaneDropZoneProps {
  /** Fired with the resolved side + path for every accepted drop. */
  readonly onDropPath: (side: PaneSide, path: string) => void;
  /**
   * Wave-120: fired when 2+ paths land in ONE drop (BC dual-drop parity:
   * Item 0 -> Left, Item 1 -> Right, regardless of half). Omitted ->
   * single-path fallback (`onDropPath` with the first path, 117 intact).
   */
  readonly onDropPaths?: (paths: readonly string[], side: PaneSide) => void;
  /** `false` (e.g. no open session) disables drops + affordance. */
  readonly enabled: boolean;
  readonly className?: string;
  readonly children: React.ReactNode;
  /** Wave-159: busy guard during comparison run */
  readonly isComparing?: boolean;
  readonly comparing?: boolean;
  readonly onBusyDrop?: () => void;
}

function resolveSide(target: EventTarget | null, clientX: number, root: HTMLElement): PaneSide {
  if (target instanceof HTMLElement) {
    const sided = target.closest("[data-panedrop]");
    if (sided instanceof HTMLElement) {
      const side = sided.getAttribute("data-panedrop");
      if (side === "left" || side === "right") return side;
    }
    const pane = target.closest("[data-pane]");
    if (pane instanceof HTMLElement) {
      const p = pane.getAttribute("data-pane");
      if (p === "left" || p === "right") return p;
    }
  }
  // Wave-G2: same bar-rect helper the PathBar strip uses (one midpoint
  // rule, hovered-half fallback when the event carries no position) —
  // synthetic clientX:0 drops fall back to the armed half, else left.
  try {
    return resolveStripSideFromRect(clientX, root.getBoundingClientRect(), hoveredPaneHalf() ?? "left");
  } catch {
    return hoveredPaneHalf() ?? "left";
  }
}

const ACTIVE_OUTLINE = "2px dashed var(--ac-accent)";
const ACTIVE_FILL = "rgba(59, 130, 246, 0.08)";

export function PaneDropZone({
  onDropPath,
  onDropPaths,
  enabled,
  className = "",
  children,
  isComparing,
  comparing,
  onBusyDrop,
}: PaneDropZoneProps): React.JSX.Element {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [hoverSide, setHoverSide] = useState<PaneSide | null>(null);

  const isBusyNow = useCallback((): boolean => {
    return Boolean(
      isComparing ||
        comparing ||
        (typeof document !== "undefined" &&
          document.querySelector(
            '[data-comparing="true"], [aria-busy="true"], [data-testid="pathbar"][aria-busy="true"], [data-testid="toolbar/action-compare"][aria-busy="true"]',
          )),
    );
  }, [isComparing, comparing]);

  const handleDragOver = useCallback(
    (e: React.DragEvent<HTMLDivElement>) => {
      if (!enabled) return;
      if (isBusyNow()) {
        e.preventDefault();
        e.stopPropagation();
        setHoverSide(null);
        return;
      }
      e.preventDefault();
      e.stopPropagation();
      try {
        if (e.dataTransfer) e.dataTransfer.dropEffect = "copy";
      } catch {
        // Best-effort only.
      }
      const root = rootRef.current;
      if (root) setHoverSide(resolveSide(e.target, e.clientX, root));
    },
    [enabled, isBusyNow],
  );

  const handleDragLeave = useCallback((e: React.DragEvent<HTMLDivElement>) => {
    if (e.currentTarget.contains(e.relatedTarget as Node | null)) return;
    setHoverSide(null);
  }, []);

  const handleDrop = useCallback(
    (e: React.DragEvent<HTMLDivElement>) => {
      if (!enabled) return;
      if (isBusyNow()) {
        e.preventDefault();
        e.stopPropagation();
        setHoverSide(null);
        onBusyDrop?.();
        if (typeof window !== "undefined") {
          window.dispatchEvent(
            new CustomEvent("autocompare:drop-busy", { detail: { note: PANE_BUSY_NOTE } }),
          );
        }
        return;
      }
      e.preventDefault();
      e.stopPropagation();
      const root = rootRef.current;
      const side = root ? resolveSide(e.target, e.clientX, root) : "left";
      setHoverSide(null);

      // Wave-126: preserve active ignore flags on re-compare
      markDropRecompare();

      // Wave-120: 2+ paths in one drop route to `onDropPaths` (BC Item 0
      // -> Left, Item 1 -> Right); single path keeps the 117 `onDropPath`.
      if (onDropPaths) {
        const paths = extractDroppedPaths(e.dataTransfer);
        if (paths.length >= 2) {
          onDropPaths(paths, side);
          return;
        }
        if (paths.length === 1 && paths[0] !== undefined) {
          onDropPath(side, paths[0]);
          return;
        }
      }
      const path = extractDroppedPath(e.dataTransfer);
      if (path) onDropPath(side, path);
    },
    [enabled, isBusyNow, onBusyDrop, onDropPath, onDropPaths],
  );

  // Wave-173: click-to-browse per pane half (BC Browse parity). The
  // picked path rides the SAME `onDropPath` as a drop (so single-side
  // stages honestly + kind-switch apply, no new IPC). Buttons are
  // `hidden` until root hover (zero chrome at rest, zero layout shift —
  // absolute overlay, no stacked rows) and carry `data-panedrop` so a
  // mid-hover drop landing on them still resolves the half. Mock without
  // a stub parks them unrendered (no native dialog, FolderView twin).
  const canBrowse =
    typeof window !== "undefined" &&
    (typeof (window as unknown as Record<string, unknown>).__MOCK_DIALOG_OPEN__ === "function" ||
      (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ !== undefined);

  const handleBrowseSide = useCallback(
    (side: PaneSide): void => {
      if (!enabled) return;
      if (isBusyNow()) {
        onBusyDrop?.();
        if (typeof window !== "undefined") {
          window.dispatchEvent(
            new CustomEvent("autocompare:drop-busy", { detail: { note: PANE_BUSY_NOTE } }),
          );
        }
        return;
      }
      void (async () => {
        const picked = await pickOpenDialog({
          directory: isFolderViewActive(),
          multiple: false,
          title: `Select ${side} path`,
        });
        const path = coercePickedPath(picked as string | readonly string[] | null);
        if (!path) return;
        markDropRecompare();
        onDropPath(side, path);
      })();
    },
    [enabled, isBusyNow, onBusyDrop, onDropPath],
  );

  return (
    <div
      ref={rootRef}
      data-testid="panedrop/root"
      onDragOver={handleDragOver}
      onDragLeave={handleDragLeave}
      onDrop={handleDrop}
      className={`${className} group/panedrop`}
      style={{ position: "relative" }}
    >
      {children}
      {/* Wave-225: sections ARE testid-addressable (`panedrop/section-*`
          below); `data-overlay` keeps the shared `assertNoOverlap`
          helper honest for these pane-covering overlays. `[data-panedrop]`
          selectors keep working exactly as before. */}
      {enabled && canBrowse && (
        <>
          {(["left", "right"] as const).map((side) => (
            <button
              key={`browse-${side}`}
              type="button"
              data-testid={`panedrop/browse-${side}`}
              data-panedrop={side}
              aria-label={`Browse for ${side} path`}
              title={`Browse for ${side} path`}
              onClick={(e) => {
                e.stopPropagation();
                handleBrowseSide(side);
              }}
              className="hidden group-hover/panedrop:block"
              style={{
                position: "absolute",
                top: "3.25rem",
                left: side === "left" ? "25%" : "75%",
                transform: "translateX(-50%)",
                zIndex: 5,
                fontSize: "11px",
                fontWeight: 600,
                padding: "2px 10px",
                borderRadius: "9999px",
                border: "1px solid var(--ac-border)",
                backgroundColor: "var(--ac-bg-raised)",
                color: "var(--ac-fg)",
                whiteSpace: "nowrap",
                cursor: "pointer",
              }}
            >
              {`Browse ${side}`}
            </button>
          ))}
        </>
      )}
      {/* Wave-225: the halves ARE the two per-side drop sections
          (BC two-pane parity) — `panedrop/section-left|right` with the
          same `data-panedrop` side identity, so a drop dispatched on a
          section resolves via `closest` even with `clientX: 0`, while a
          real positioned drop resolves by half exactly as before.
          `data-overlay` keeps the shared `assertNoOverlap` helper
          honest (overlays cover panes by design); `pointer-events`
          stays none so clicks/drags pass through to the panes and the
          root keeps owning every drop/dragover (busy/bubble intact).
          `panedrop/root` stays as the compat wrapper. */}
      {enabled && (
        <>
          {(["left", "right"] as const).map((side) => {
            const active = hoverSide === side;
            return (
              <div
                key={side}
                data-testid={`panedrop/section-${side}`}
                data-panedrop={side}
                data-active={active ? "true" : "false"}
                data-overlay="true"
                aria-hidden="true"
                style={{
                  position: "absolute",
                  top: 0,
                  bottom: 0,
                  left: side === "left" ? 0 : "50%",
                  width: "50%",
                  pointerEvents: "none",
                  outline: active ? ACTIVE_OUTLINE : "none",
                  outlineOffset: "-2px",
                  backgroundColor: active ? ACTIVE_FILL : "transparent",
                }}
              >
                {active && (
                  <span
                    style={{
                      position: "absolute",
                      left: "50%",
                      top: "1rem",
                      transform: "translateX(-50%)",
                      fontSize: "11px",
                      fontWeight: 600,
                      padding: "2px 10px",
                      borderRadius: "9999px",
                      border: "1px solid var(--ac-border)",
                      backgroundColor: "var(--ac-bg-raised)",
                      color: "var(--ac-fg)",
                      whiteSpace: "nowrap",
                    }}
                  >
                    {`Drop to set ${side}`}
                  </span>
                )}
              </div>
            );
          })}
        </>
      )}
    </div>
  );
}

/**
 * Production-shell Tauri drag-drop subscription. Under `VITE_MOCK_IPC=1`
 * or outside the Tauri shell this is a no-op (the HTML5 handlers above
 * own drops there). Isolated here so `App` only wires one hook.
 *
 * Wave-120: optional `onDropPaths` receives 2+ shell paths in one drop
 * (BC dual-drop); single path keeps the 117 `onDropPath`.
 * Wave-126: deduplicates paths from payload/Tauri events so duplicate paths
 * are ignored while order is preserved.
 * Wave-143: shell probe fixed — `withGlobalTauri` defaults false, so the
 * real binary has no `window.__TAURI__`; subscribe when EITHER global
 * exists (`hasTauriDropBridge`, internals always injected in v2).
 * Side routing is DPR-aware (`resolveTauriDropCssSide`: physical px ->
 * CSS px before the half comparison, else Retina drops misroute).
 */
export function useTauriPaneDrop(
  enabled: boolean,
  onDropPath: (side: PaneSide, path: string) => void,
  onDropPaths?: (paths: readonly string[], side: PaneSide) => void,
): void {
  const handlerRef = useRef(onDropPath);
  handlerRef.current = onDropPath;
  const multiRef = useRef(onDropPaths);
  multiRef.current = onDropPaths;

  useEffect(() => {
    if (!enabled) return;
    if (import.meta.env.VITE_MOCK_IPC === "1") return;
    try {
      const w = window as unknown as Record<string, unknown>;
      // Wave-143: `__TAURI_INTERNALS__` is always injected in Tauri v2;
      // `__TAURI__` only with `withGlobalTauri:true` (we leave it false).
      if (typeof window === "undefined" || !hasTauriDropBridge(w)) return;
    } catch {
      return;
    }
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    void (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        if (cancelled) return;
        unlisten = await getCurrentWebview().onDragDropEvent((event) => {
          try {
            if (event.payload.type !== "drop") return;
            const payload = event.payload as unknown as {
              readonly paths?: readonly string[];
              readonly position?: { readonly x?: number };
            };
            // Wave-126: Tauri-path dedup (preserves order, deduplicates exact repeats)
            const paths = dedupTauriDropPaths(payload.paths);
            if (paths.length === 0) return;
            // Wave-132: PathBar-band drops belong to the PathBar hook
            // (one drop must not double-compare); unknown position keeps
            // the pane fallback below.
            // Wave-140: y-gated strip (input+gap+label) owns its band;
            // unknown position stays pane-owned — single compare guaranteed.
            if (isTauriDropOnPathBar(payload.position)) return;
            // Wave-159: reject drops attempted while comparing
            if (
              typeof document !== "undefined" &&
              document.querySelector(
                '[data-comparing="true"], [data-testid="pathbar"][aria-busy="true"], [data-testid="toolbar/action-compare"][aria-busy="true"]',
              )
            ) {
              window.dispatchEvent(
                new CustomEvent("autocompare:drop-busy", { detail: { note: PANE_BUSY_NOTE } }),
              );
              return;
            }
            markDropRecompare();
            // Wave-132: x vs half-width, hovered-half fallback when the
            // shell gives no position. Wave-143: physical-px -> CSS-px
            // first (Retina DPR 2 drops misrouted before this). Wave-G2:
            // unknown position stays pane-owned by design (single owner:
            // `isTauriDropOnPathBar` is false without y, so the PathBar
            // hook skips and exactly one hook compares).
            const side: PaneSide = resolveTauriDropCssSide(
              payload.position,
              window.innerWidth,
              hoveredPaneHalf() ?? "left",
            );
            // Wave-120: shell dual-drop (BC Item 0 -> Left, Item 1 ->
            // Right) before the 117 single-path fallback.
            if (paths.length >= 2 && multiRef.current) {
              multiRef.current(paths, side);
              return;
            }
            const path = paths[0] ?? firstTauriDropPath(paths);
            if (!path) return;
            handlerRef.current(side, path);
          } catch {
            // A malformed shell payload must never break the workspace.
          }
        });
      } catch {
        // Browser / mock: HTML5 handlers cover drops.
      }
    })();
    return () => {
      cancelled = true;
      try {
        unlisten?.();
      } catch {
        // Best-effort detach.
      }
    };
  }, [enabled]);
}

/**
 * Wave-132: production-shell Tauri drag-drop subscription for the
 * PathBar inputs. Same pattern as `useTauriPaneDrop`: no-op under
 * `VITE_MOCK_IPC=1` / outside the Tauri shell (HTML5 `onDrop` owns
 * drops there). Only handles drops landing on the PathBar strip
 * (`isTauriDropOnPathBar`); pane-band drops stay with the pane hook,
 * so one drop never double-compares.
 *
 * Position maps to the field side (x vs half-width, hovered-half
 * fallback): single-file stages that side via `onDropPath`, 2-file
 * dual stages Item0 -> left, Item1 -> right via `onDropPaths`.
 * Wave-G2: single-owner rule — unknown position is pane-owned
 * (`isTauriDropOnPathBar` false without y), so this hook skips it and
 * exactly one hook compares.
 */
export function useTauriPathBarDrop(
  enabled: boolean,
  onDropPath: (side: PaneSide, path: string) => void,
  onDropPaths?: (paths: readonly string[]) => void,
): void {
  const handlerRef = useRef(onDropPath);
  handlerRef.current = onDropPath;
  const multiRef = useRef(onDropPaths);
  multiRef.current = onDropPaths;

  useEffect(() => {
    if (!enabled) return;
    if (import.meta.env.VITE_MOCK_IPC === "1") return;
    try {
      const w = window as unknown as Record<string, unknown>;
      // Wave-143: same shell-probe fix as `useTauriPaneDrop` (internals
      // global, not the legacy `__TAURI__` which needs withGlobalTauri).
      if (typeof window === "undefined" || !hasTauriDropBridge(w)) return;
    } catch {
      return;
    }
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    void (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        if (cancelled) return;
        unlisten = await getCurrentWebview().onDragDropEvent((event) => {
          try {
            if (event.payload.type !== "drop") return;
            const payload = event.payload as unknown as {
              readonly paths?: readonly string[];
              readonly position?: { readonly x?: number };
            };
            if (!isTauriDropOnPathBar(payload.position)) return;
            const paths = dedupTauriDropPaths(payload.paths);
            if (paths.length === 0) return;
            // Wave-159: reject drops attempted while comparing
            if (
              typeof document !== "undefined" &&
              document.querySelector(
                '[data-comparing="true"], [data-testid="pathbar"][aria-busy="true"], [data-testid="toolbar/action-compare"][aria-busy="true"]',
              )
            ) {
              window.dispatchEvent(
                new CustomEvent("autocompare:drop-busy", { detail: { note: PANE_BUSY_NOTE } }),
              );
              return;
            }
            markDropRecompare();
            // Wave-143: DPR-aware side (physical -> CSS, Retina-safe).
            const side: PaneSide = resolveTauriDropCssSide(
              payload.position,
              window.innerWidth,
              hoveredPaneHalf() ?? "left",
            );
            if (paths.length >= 2 && multiRef.current) {
              multiRef.current(paths);
              return;
            }
            const path = paths[0] ?? firstTauriDropPath(paths);
            if (!path) return;
            handlerRef.current(side, path);
          } catch {
            // A malformed shell payload must never break the workspace.
          }
        });
      } catch {
        // Browser / mock: HTML5 handlers cover drops.
      }
    })();
    return () => {
      cancelled = true;
      try {
        unlisten?.();
      } catch {
        // Best-effort detach.
      }
    };
  }, [enabled]);
}

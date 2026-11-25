import { useEffect, useRef } from "react";
import {
  dedupTauriDropPaths,
  firstTauriDropPath,
  hasTauriDropBridge,
  hoveredPaneHalf,
  isTauriDropOnPathBar,
  markDropRecompare,
  resolveTauriDropCssSide,
} from "../dnd/dropPaths";
import { PANE_BUSY_NOTE, type PaneSide } from "../dnd/PaneDropZone";

/**
 * Wave-211: native Finder/Explorer file-drop subscription (BC parity).
 *
 * Real OS file drops land on the Tauri WINDOW (title bar, chrome, and the
 * webview all funnel through the window drag-drop event); the webview-level
 * `useTauriPaneDrop` hook never sees drops that miss the webview surface,
 * so Finder drops on the app window do nothing today. This hook subscribes
 * at the window level — `getCurrentWindow().onDragDropEvent` from
 * `@tauri-apps/api/window` (installed 2.12.5; v2 has no `onFileDropEvent`,
 * that name is Tauri v1-only) — and routes drops through the SAME App
 * handlers the web/HTML5 drops use (`onDropPath` / `onDropPaths`), so one
 * drop always yields exactly one auto-compare via `runComparePaths`.
 *
 * Routing mirrors the pathdrop-140 strip rule: CSS-pixel x vs the window
 * midpoint (`resolveTauriDropCssSide`: physical px -> CSS px, DPR-aware,
 * `hoveredPaneHalf() ?? "left"` fallback when the shell gives no
 * position — the same one-midpoint-rule family the PathBar strip and the
 * pane halves share). PathBar-band drops are skipped (`isTauriDropOnPathBar`)
 * so the PathBar hook stays the single owner there — never double-compare.
 * Hover/leave drive the EXISTING `PaneDropZone` `[data-panedrop]` overlay
 * halves (same outline/fill) so the affordance is reused, not duplicated.
 *
 * No-op under `VITE_MOCK_IPC=1` / plain browser (HTML5 handlers own drops
 * there), same contract as the webview hook it replaces in `App`.
 */

type NativeDropPosition = { readonly x?: unknown; readonly y?: unknown } | null | undefined;

// Same literals as `PaneDropZone` ACTIVE_OUTLINE / ACTIVE_FILL (kept there;
// duplicated, not restyled — the overlay DOM is reused as-is).
const NATIVE_ACTIVE_OUTLINE = "2px dashed var(--ac-accent)";
const NATIVE_ACTIVE_FILL = "rgba(59, 130, 245, 0.08)";

/** Arm/clear the existing pane-half affordance overlays (best-effort). */
function setNativeHoverAffordance(side: PaneSide | null): void {
  try {
    if (typeof document === "undefined") return;
    const halves = document.querySelectorAll("[data-panedrop]");
    halves.forEach((node) => {
      if (!(node instanceof HTMLElement)) return;
      const own = node.getAttribute("data-panedrop");
      const active = side !== null && own === side;
      node.setAttribute("data-active", active ? "true" : "false");
      node.style.outline = active ? NATIVE_ACTIVE_OUTLINE : "none";
      node.style.outlineOffset = "-2px";
      node.style.backgroundColor = active ? NATIVE_ACTIVE_FILL : "transparent";
    });
  } catch {
    // Affordance is cosmetic; a hostile DOM must never break drops.
  }
}

export function useNativeFileDrop(
  enabled: boolean,
  onDropPath: (side: PaneSide, path: string) => void,
  onDropPaths?: (paths: readonly string[], side: PaneSide) => void,
): void {
  const handlerRef = useRef(onDropPath);
  handlerRef.current = onDropPath;
  const multiRef = useRef(onDropPaths);
  multiRef.current = onDropPaths;
  // Belt-and-braces: ignore an identical OS re-emit within 300ms so one
  // physical drop can never double-compare even if the shell re-fires.
  const lastDropRef = useRef<{ readonly key: string; readonly at: number } | null>(null);

  useEffect(() => {
    if (!enabled) return;
    if (import.meta.env.VITE_MOCK_IPC === "1") return;
    try {
      const w = window as unknown as Record<string, unknown>;
      if (typeof window === "undefined" || !hasTauriDropBridge(w)) return;
    } catch {
      return;
    }
    let cancelled = false;
    let unlisten: (() => void) | null = null;
    void (async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        if (cancelled) return;
        unlisten = await getCurrentWindow().onDragDropEvent((event) => {
          try {
            const payload = event.payload as unknown as {
              readonly type?: unknown;
              readonly paths?: readonly string[];
              readonly position?: NativeDropPosition;
            };
            if (payload.type === "leave") {
              setNativeHoverAffordance(null);
              return;
            }
            if (payload.type === "enter" || payload.type === "over") {
              // PathBar band owns its affordance; pane halves arm here.
              if (isTauriDropOnPathBar(payload.position)) {
                setNativeHoverAffordance(null);
                return;
              }
              setNativeHoverAffordance(
                resolveTauriDropCssSide(
                  payload.position,
                  window.innerWidth,
                  hoveredPaneHalf() ?? "left",
                ),
              );
              return;
            }
            if (payload.type !== "drop") return;
            setNativeHoverAffordance(null);
            // Single-owner rule: PathBar-band drops belong to the PathBar
            // hook; unknown position stays pane-owned (exactly one compare).
            if (isTauriDropOnPathBar(payload.position)) return;
            const paths = dedupTauriDropPaths(payload.paths);
            if (paths.length === 0) return;
            const now = Date.now();
            const key = paths.join("\u0000");
            const last = lastDropRef.current;
            if (last !== null && last.key === key && now - last.at < 300) return;
            lastDropRef.current = { key, at: now };
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
            const side: PaneSide = resolveTauriDropCssSide(
              payload.position,
              window.innerWidth,
              hoveredPaneHalf() ?? "left",
            );
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
      setNativeHoverAffordance(null);
      try {
        unlisten?.();
      } catch {
        // Best-effort detach.
      }
    };
  }, [enabled]);
}

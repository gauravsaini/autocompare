import { useEffect, useState } from "react";
import type React from "react";
import type { SessionKind, V2Tab, V2ViewCommands } from "../types";
import { loadKeymapPreset, resolveKeymapAction } from "../utils/keymap";
import type { KeymapPreset } from "../utils/keymap";

function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  const tag = target.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT" || target.isContentEditable;
}

export interface UseV2ShortcutsOptions {
  readonly activeTabRef: React.MutableRefObject<V2Tab>;
  readonly commandsRef: React.MutableRefObject<V2ViewCommands>;
  readonly openSession: (kind: SessionKind, left?: string, right?: string, title?: string) => void;
  readonly closeTab: (id: string) => void;
  readonly onSaveSession: () => Promise<void>;
  readonly onDroppedPaths: (paths: readonly string[], side: "left" | "right") => void;
  readonly onTogglePalette: () => void;
  readonly onToggleShortcuts: () => void;
  readonly onOpenNewSession: () => void;
  readonly onOpenSettings: () => void;
  readonly onToggleTheme: () => void;
  readonly onToggleLayout: () => void;
}

export function useV2Shortcuts({
  activeTabRef,
  commandsRef,
  openSession,
  closeTab,
  onSaveSession,
  onDroppedPaths,
  onTogglePalette,
  onToggleShortcuts,
  onOpenNewSession,
  onOpenSettings,
  onToggleTheme,
  onToggleLayout,
}: UseV2ShortcutsOptions) {
  const [dropOverlay, setDropOverlay] = useState<"left" | "right" | "both" | null>(null);
  const [keymap, setKeymap] = useState<KeymapPreset>(() => loadKeymapPreset());

  useEffect(() => {
    const onKeymapChange = (e: Event): void => {
      setKeymap((e as CustomEvent<KeymapPreset>).detail ?? loadKeymapPreset());
    };
    window.addEventListener("autocompare:keymap-changed", onKeymapChange);
    return () => window.removeEventListener("autocompare:keymap-changed", onKeymapChange);
  }, []);

  // Global keyboard shortcuts (aligned 1:1 with src-tauri/src/menu.rs + BC ergonomics)
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const mod = e.metaKey || e.ctrlKey;
      const key = e.key.toLowerCase();
      if (mod && key === "k") {
        e.preventDefault();
        onTogglePalette();
        return;
      }
      if (mod && e.key === "/") {
        e.preventDefault();
        onToggleShortcuts();
        return;
      }
      if (mod && key === "s") {
        e.preventDefault();
        if (activeTabRef.current.kind !== "home") {
          void onSaveSession();
        }
        return;
      }
      if (mod && key === "o") {
        e.preventDefault();
        if (e.shiftKey) {
          openSession("folder");
        } else {
          onOpenNewSession();
        }
        return;
      }
      if (mod && key === "n") {
        e.preventDefault();
        onOpenNewSession();
        return;
      }
      if (mod && key === "f" && commandsRef.current.openFind) {
        e.preventDefault();
        commandsRef.current.openFind();
        return;
      }
      if (mod && key === ",") {
        e.preventDefault();
        onOpenSettings();
        return;
      }
      if (mod && key === "w" && activeTabRef.current.kind !== "home") {
        e.preventDefault();
        closeTab(activeTabRef.current.id);
        return;
      }
      if (mod && key === "d" && commandsRef.current.canCopy) {
        e.preventDefault();
        commandsRef.current.copySelected?.();
        return;
      }
      if (mod && key === "e" && commandsRef.current.canCopy) {
        e.preventDefault();
        commandsRef.current.copySelectedRight?.();
        return;
      }
      if (mod && !e.shiftKey && !e.altKey) {
        const kindByDigit: Record<string, SessionKind> = {
          "1": "text",
          "2": "folder",
          "3": "hex",
          "4": "table",
          "5": "media",
          "6": "sync",
          "7": "merge",
        };
        const targetKind = kindByDigit[key];
        if (targetKind) {
          e.preventDefault();
          openSession(targetKind);
          return;
        }
      }
      if (!isEditableTarget(e.target)) {
        if (!mod && e.key === "?") {
          e.preventDefault();
          onToggleShortcuts();
          return;
        }
        const mapped = resolveKeymapAction(e, keymap);
        if (mapped === "nextSection" && commandsRef.current.hasSections) {
          e.preventDefault();
          commandsRef.current.nextSection?.();
        } else if (mapped === "prevSection" && commandsRef.current.hasSections) {
          e.preventDefault();
          commandsRef.current.prevSection?.();
        } else if (mapped === "openFind" && commandsRef.current.openFind) {
          e.preventDefault();
          commandsRef.current.openFind();
        } else if (mapped === "copyLeft" && commandsRef.current.canCopy) {
          e.preventDefault();
          commandsRef.current.copySelected?.();
        } else if (mapped === "copyRight" && commandsRef.current.canCopy) {
          e.preventDefault();
          commandsRef.current.copySelectedRight?.();
        }
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [
    activeTabRef,
    commandsRef,
    closeTab,
    onSaveSession,
    openSession,
    onTogglePalette,
    onToggleShortcuts,
    onOpenNewSession,
    onOpenSettings,
    keymap,
  ]);

  // Native Tauri window-level file drag-and-drop listener
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let active = true;

    const attach = async (): Promise<void> => {
      const hasTauri =
        typeof window !== "undefined" &&
        ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
      if (!hasTauri) return;
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const fn = await getCurrentWindow().onDragDropEvent((event) => {
          if (!active) return;
          const payload = event.payload;
          if (payload.type === "enter" || payload.type === "over") {
            const x = payload.position?.x ?? 0;
            const dpr = window.devicePixelRatio || 1;
            const logicalX = x > window.innerWidth ? x / dpr : x;
            const count =
              "paths" in payload && Array.isArray(payload.paths) ? payload.paths.length : 1;
            setDropOverlay(
              count >= 2 ? "both" : logicalX >= window.innerWidth / 2 ? "right" : "left",
            );
          } else if (payload.type === "drop") {
            setDropOverlay(null);
            const x = payload.position?.x ?? 0;
            const dpr = window.devicePixelRatio || 1;
            const logicalX = x > window.innerWidth ? x / dpr : x;
            const side = logicalX >= window.innerWidth / 2 ? "right" : "left";
            onDroppedPaths(payload.paths ?? [], side);
          } else {
            setDropOverlay(null);
          }
        });
        if (active) unlisten = fn;
        else fn();
      } catch {
        // Non-Tauri browser preview
      }
    };

    void attach();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [onDroppedPaths]);

  // Native macOS menu-event listener (all 18 menu IDs in src-tauri/src/menu.rs)
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let active = true;

    const attach = async (): Promise<void> => {
      const hasTauri =
        typeof window !== "undefined" &&
        ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);
      if (!hasTauri) return;
      try {
        const { listen } = await import("@tauri-apps/api/event");
        const fn = await listen<{ id: string; command: string }>("menu-event", (event) => {
          if (!active) return;
          const id = event.payload?.id ?? "";
          switch (id) {
            case "file.open-session":
            case "file.open-workspace":
            case "session.new":
              onOpenNewSession();
              break;
            case "file.open-folder":
              openSession("folder");
              break;
            case "file.save-session":
              void onSaveSession();
              break;
            case "session.find":
              commandsRef.current.openFind?.();
              break;
            case "session.take-left":
              commandsRef.current.copySelected?.();
              break;
            case "session.take-right":
              commandsRef.current.copySelectedRight?.();
              break;
            case "view.kind-text":
              openSession("text");
              break;
            case "view.kind-folder":
              openSession("folder");
              break;
            case "view.kind-hex":
              openSession("hex");
              break;
            case "view.kind-table":
              openSession("table");
              break;
            case "view.kind-media":
              openSession("media");
              break;
            case "view.kind-sync":
              openSession("sync");
              break;
            case "view.kind-merge":
              openSession("merge");
              break;
            case "view.toggle-theme":
              onToggleTheme();
              break;
            case "view.toggle-layout":
              onToggleLayout();
              break;
            case "tools.settings":
              onOpenSettings();
              break;
            default:
              break;
          }
        });
        if (active) unlisten = fn;
        else fn();
      } catch {
        // Non-Tauri browser preview
      }
    };

    void attach();
    return () => {
      active = false;
      unlisten?.();
    };
  }, [
    commandsRef,
    openSession,
    onSaveSession,
    onOpenNewSession,
    onOpenSettings,
    onToggleTheme,
    onToggleLayout,
  ]);

  return { dropOverlay, setDropOverlay };
}

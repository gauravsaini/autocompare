import { useEffect } from "react";

/**
 * Wave-142: web fallback for native menu accelerators (BC parity).
 *
 * The Tauri shell (`src-tauri/src/menu.rs`) owns CmdOrCtrl+1..7 / N / O /
 * S / D / E / F as native menu accelerators, routed to `App` through the
 * `menu-event`. Under plain browsers / mock IPC (`VITE_MOCK_IPC=1`,
 * Playwright webkit) there is no native menu, so those chords did
 * nothing. This bridge re-emits the SAME `menu-event` CustomEvent the
 * native shell emits, so `App`'s existing (read-only, wave-141-owned)
 * menu wiring handles them with zero duplication.
 *
 * Deliberately SKIPPED: Cmd/Ctrl+F (and G). Every mounted view already
 * owns those chords with `preventDefault` + open-first-then-focus
 * (text/table/hex/merge/folder/sync); a global re-emit would double-open
 * and steal focus. F/G stay view-local.
 *
 * macOS note: Safari reserves Cmd+1..9 (tab switch), Cmd+N (new window),
 * Cmd+D (bookmark) at browser level; `preventDefault` on the page
 * suppresses them while the app has focus, but a real Tauri shell is
 * the authoritative host for those chords. See checks/shortcuts-142.
 */

/** Menu ids mirrored from `MENU_ITEM_IDS` in `src-tauri/src/menu.rs`. */
export const WEB_SHORTCUT_MENU_IDS = {
  "view.kind-text": "view.kind-text",
  "view.kind-folder": "view.kind-folder",
  "view.kind-hex": "view.kind-hex",
  "view.kind-table": "view.kind-table",
  "view.kind-media": "view.kind-media",
  "view.kind-sync": "view.kind-sync",
  "view.kind-merge": "view.kind-merge",
  "session.new": "session.new",
  "file.open-session": "file.open-session",
  "file.open-folder": "file.open-folder",
  "file.save-session": "file.save-session",
  "session.take-left": "session.take-left",
  "session.take-right": "session.take-right",
} as const;

export type WebShortcutMenuId = (typeof WEB_SHORTCUT_MENU_IDS)[keyof typeof WEB_SHORTCUT_MENU_IDS];

/** 1:1 IPC command intents mirrored from `route_menu_event` (menu.rs). */
const WEB_SHORTCUT_COMMANDS: Record<WebShortcutMenuId, string> = {
  "view.kind-text": "text_compare",
  "view.kind-folder": "folder_compare",
  "view.kind-hex": "hex_compare",
  "view.kind-table": "table_compare",
  "view.kind-media": "picture_compare",
  "view.kind-sync": "sync_plan",
  "view.kind-merge": "merge_compare",
  "session.new": "session_new",
  "file.open-session": "session_load",
  "file.open-folder": "folder_compare",
  "file.save-session": "session_save",
  "session.take-left": "text_copy_section",
  "session.take-right": "text_copy_section",
};

export interface WebShortcutChord {
  readonly key: string;
  readonly metaKey: boolean;
  readonly ctrlKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
}

/**
 * Pure chord matcher (unit-coverable, no DOM). Returns the menu id for a
 * CmdOrCtrl chord, else `null`. Requires the platform modifier
 * (Cmd on macOS / Ctrl elsewhere); Alt chords never match (macOS
 * Option-combos type special characters and must not be hijacked).
 */
export function matchWebShortcut(chord: WebShortcutChord): WebShortcutMenuId | null {
  if (!chord.metaKey && !chord.ctrlKey) return null;
  if (chord.altKey) return null;
  const key = chord.key;
  const lower = key.toLowerCase();
  if (!chord.shiftKey) {
    switch (key) {
      case "1":
        return "view.kind-text";
      case "2":
        return "view.kind-folder";
      case "3":
        return "view.kind-hex";
      case "4":
        return "view.kind-table";
      case "5":
        return "view.kind-media";
      case "6":
        return "view.kind-sync";
      case "7":
        return "view.kind-merge";
      default:
        break;
    }
  }
  // CmdOrCtrl+Shift+O opens a folder (native menu parity); plain O a session.
  if (lower === "o") {
    return chord.shiftKey ? "file.open-folder" : "file.open-session";
  }
  if (chord.shiftKey) return null;
  switch (lower) {
    case "n":
      return "session.new";
    case "s":
      return "file.save-session";
    case "d":
      return "session.take-left";
    case "e":
      return "session.take-right";
    default:
      return null;
  }
}

/** Emit the menu-event App already routes (native-shell shape). */
export function emitWebShortcut(id: WebShortcutMenuId): void {
  window.dispatchEvent(
    new CustomEvent("menu-event", { detail: { id, command: WEB_SHORTCUT_COMMANDS[id] } }),
  );
}

/**
 * Chrome-level listener (mounted once by `ChromeShell`). Matches each
 * keydown against `matchWebShortcut`; on a hit it `preventDefault`s
 * (browser Save/Bookmark/New-window suppression) and emits the
 * menu-event. Pass-through otherwise, so view-local chords (F/G, plain
 * l/r/n/p/j/k, arrows, Enter/Esc) are untouched.
 */
export function useWebShortcuts(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      const id = matchWebShortcut({
        key: e.key,
        metaKey: e.metaKey,
        ctrlKey: e.ctrlKey,
        shiftKey: e.shiftKey,
        altKey: e.altKey,
      });
      if (id === null) return;
      e.preventDefault();
      emitWebShortcut(id);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}

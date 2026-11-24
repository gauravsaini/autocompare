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
 * Deliberately SKIPPED from menu matching: Cmd/Ctrl+F (and G). Every mounted view already
 * owns those chords with `preventDefault` + open-first-then-focus
 * (text/table/hex/merge/folder/sync); a global menu re-emit would double-open
 * and steal focus. F/G stay view-local, backed by fallback triggers that
 * avoid colliding with view-local listeners.
 *
 * macOS note: Safari reserves Cmd+1..9 (tab switch), Cmd+N (new window),
 * Cmd+D (bookmark) at browser level; `preventDefault` on the page
 * suppresses them while the app has focus, but a real Tauri shell is
 * the authoritative host for those chords. See checks/shortcuts-142.
 *
 * Wave-G2: CmdOrCtrl+D/E (`session.take-left/right`) intentionally carry
 * no selection payload — `App.handleMenuTake` owns the fallback (nearest
 * hunk when an equal row is selected, honest parked note when nothing
 * is selected), so this bridge stays a pure 1:1 id forwarder.
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
  "session.take-left": "text_copy_left",
  "session.take-right": "text_copy_right",
};

export interface WebShortcutChord {
  readonly key: string;
  readonly metaKey: boolean;
  readonly ctrlKey: boolean;
  readonly shiftKey: boolean;
  readonly altKey: boolean;
}

export function isMacPlatform(): boolean {
  if (typeof navigator === "undefined") return false;
  return /Mac|iPhone|iPod|iPad/i.test(navigator.userAgent) || /Mac/i.test(navigator.platform);
}

/**
 * Pure chord matcher (unit-coverable, no DOM). Returns the menu id for a
 * CmdOrCtrl chord, else `null`. Requires the platform modifier
 * (Cmd on macOS / Ctrl elsewhere); Alt chords never match (macOS
 * Option-combos type special characters and must not be hijacked).
 * FIXSHELL GAP-12: On macOS, pure Ctrl chords are standard readline text
 * shortcuts (Ctrl+D forward delete, Ctrl+E end of line, Ctrl+N next line)
 * and must not match menu accelerators.
 */
export function matchWebShortcut(chord: WebShortcutChord, isMac: boolean = isMacPlatform()): WebShortcutMenuId | null {
  if (isMac) {
    // macOS requires metaKey (Cmd). Pure Ctrl chords must not match.
    if (!chord.metaKey) return null;
  } else {
    // Non-macOS requires Ctrl (or Meta).
    if (!chord.ctrlKey && !chord.metaKey) return null;
  }
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
 * Wave-FIXP (N6 parity): Trigger Find Next across mounted views.
 * If the active view's find overlay is mounted with a next button, clicks it;
 * if mounted with an input, focuses it; otherwise dispatches `autocompare:session-find`
 * to open the search bar.
 */
export function triggerFindNext(): boolean {
  if (typeof document === "undefined") return false;
  const nextBtn = document.querySelector<HTMLButtonElement>(
    '[data-testid="textview/find-next"], [data-testid="merge/find-next"], [data-testid="hex-find-next"], [data-testid="table-find-next"]',
  );
  if (nextBtn && !nextBtn.disabled) {
    nextBtn.click();
    return true;
  }
  const findInput = document.querySelector<HTMLElement>(
    '[data-testid="textview/find-input"], [data-testid="hex-find-input"], [data-testid="table-find-input"], [data-testid="merge/find-input"], [data-testid="folder-name-filter"], [data-testid="syncview/name-filter"]',
  );
  if (findInput) {
    findInput.focus();
    if (findInput instanceof HTMLInputElement) findInput.select();
    return true;
  }
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("autocompare:session-find"));
    return true;
  }
  return false;
}

/**
 * Wave-FIXP (N6 parity): Trigger Find Previous across mounted views.
 * If the active view's find overlay is mounted with a prev button, clicks it;
 * if mounted with an input, focuses it; otherwise dispatches `autocompare:session-find`
 * to open the search bar.
 */
export function triggerFindPrev(): boolean {
  if (typeof document === "undefined") return false;
  const prevBtn = document.querySelector<HTMLButtonElement>(
    '[data-testid="textview/find-prev"], [data-testid="merge/find-prev"]',
  );
  if (prevBtn && !prevBtn.disabled) {
    prevBtn.click();
    return true;
  }
  const findInput = document.querySelector<HTMLElement>(
    '[data-testid="textview/find-input"], [data-testid="hex-find-input"], [data-testid="table-find-input"], [data-testid="merge/find-input"], [data-testid="folder-name-filter"], [data-testid="syncview/name-filter"]',
  );
  if (findInput) {
    findInput.focus();
    if (findInput instanceof HTMLInputElement) findInput.select();
    return true;
  }
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent("autocompare:session-find"));
    return true;
  }
  return false;
}

/**
 * Wave-FIXP (N6 parity): Focus or toggle Go To overlay across views that support it
 * (Text, Hex, Folder). Views without Go To support (Merge, Table, Sync, Media)
 * return false without error.
 */
export function triggerGoTo(): boolean {
  if (typeof document === "undefined") return false;
  const selector =
    '#textview-goto-box, #hex-goto-box, #folder-goto-box, [data-testid="textview/goto-input"], [data-testid="hex-goto-input"], [data-testid="folder-goto-input"]';
  const gotoInput = document.querySelector<HTMLElement>(selector);
  if (gotoInput) {
    gotoInput.focus();
    if (gotoInput instanceof HTMLInputElement) gotoInput.select();
    return true;
  }

  // Views supporting GoTo: text, hex, folder.
  // Views without GoTo support (merge, table, sync, media) return false without error.
  const isText = Boolean(document.querySelector('[data-testid="view-text"]'));
  const isHex = Boolean(document.querySelector('[data-testid="view-hex"]'));
  const isFolder = Boolean(document.querySelector('[data-testid="view-folder"]'));

  if (!isText && !isHex && !isFolder) {
    return false;
  }

  if (isText) {
    if (typeof window !== "undefined") {
      window.dispatchEvent(new CustomEvent("autocompare:session-goto"));
    }
    const gotoToggle = document.querySelector<HTMLButtonElement>(
      '[data-testid="textview/toolbar-goto"]',
    );
    if (gotoToggle && !gotoToggle.disabled && gotoToggle.getAttribute("aria-expanded") !== "true") {
      gotoToggle.click();
    }
  } else if (isHex) {
    const gotoToggle = document.querySelector<HTMLButtonElement>(
      '[data-testid="hexview/toolbar-goto"]',
    );
    if (gotoToggle && !gotoToggle.disabled && gotoToggle.getAttribute("aria-expanded") !== "true") {
      gotoToggle.click();
    }
  } else if (isFolder) {
    // FolderView opens goto on Cmd/Ctrl+G keydown
    if (typeof window !== "undefined") {
      const evt = new KeyboardEvent("keydown", {
        key: "g",
        metaKey: isMacPlatform(),
        ctrlKey: !isMacPlatform(),
        bubbles: true,
        cancelable: true,
      });
      Object.defineProperty(evt, "_ac_synthetic", { value: true });
      window.dispatchEvent(evt);
    }
  }

  let attempts = 0;
  const tick = (): void => {
    attempts += 1;
    const target = document.querySelector<HTMLElement>(selector);
    if (target) {
      target.focus();
      if (target instanceof HTMLInputElement) target.select();
      return;
    }
    if (attempts < 25 && typeof window !== "undefined") window.setTimeout(tick, 30);
  };
  if (typeof window !== "undefined") window.setTimeout(tick, 0);
  return true;
}

/**
 * Wave-157: Trigger Next Difference across mounted views (Text, Merge).
 * Dispatches `autocompare:view-command` with cmd "next".
 */
export function triggerNextDiff(): boolean {
  if (typeof window === "undefined") return false;
  window.dispatchEvent(new CustomEvent("autocompare:view-command", { detail: { cmd: "next" } }));
  return true;
}

/**
 * Wave-157: Trigger Previous Difference across mounted views (Text, Merge).
 * Dispatches `autocompare:view-command` with cmd "prev".
 */
export function triggerPrevDiff(): boolean {
  if (typeof window === "undefined") return false;
  window.dispatchEvent(new CustomEvent("autocompare:view-command", { detail: { cmd: "prev" } }));
  return true;
}

/**
 * Chrome-level listener (mounted once by `ChromeShell`). Matches each
 * keydown against `matchWebShortcut`; on a hit it `preventDefault`s
 * (browser Save/Bookmark/New-window suppression) and emits the
 * menu-event. Pass-through otherwise, so view-local chords (plain
 * l/r/n/p/j/k, arrows, Enter) are untouched.
 *
 * Find & GoTo handling:
 * - Prevents default browser find (Cmd/Ctrl+F) and browser find-next (Cmd/Ctrl+G)
 *   so browser controls do not hijack the screen.
 * - Allows view-local listeners on mounted views to open and focus their own
 *   panels natively. Fallback triggers run smoothly only if the mounted view
 *   did not handle the chord.
 *
 * Esc handling:
 * - Closes active find/goto/chrome overlays (Session dialog, panel dismissals).
 *
 * Scope note (AUDIT2 N6):
 * - Find Next / Prev: handled via F3 / Shift+F3 (BC parity, zero collision).
 * - GoTo: handled view-locally in TextView/FolderView/HexView; triggerGoTo
 *   serves as a safe fallback.
 */
export function useWebShortcuts(): void {
  useEffect(() => {
    const onKey = (e: KeyboardEvent): void => {
      // Ignore synthetic events generated by our own triggers
      if ((e as unknown as { _ac_synthetic?: boolean })._ac_synthetic) {
        return;
      }

      const target = e.target as HTMLElement | null;
      const isInput =
        target &&
        (target.tagName === "INPUT" ||
          target.tagName === "TEXTAREA" ||
          target.isContentEditable ||
          target.getAttribute("contenteditable") === "true");

      // FIXSHELL GAP-12: In text inputs, pure Ctrl chords (e.g. macOS/readline
      // Ctrl+D forward delete, Ctrl+E end of line, Ctrl+N next line) must not
      // be hijacked by menu accelerators.
      if (isInput && e.ctrlKey && !e.metaKey) {
        return;
      }

      // Esc handler: closes any active find/goto/chrome overlays
      if (e.key === "Escape") {
        let handled = false;

        // 1. Session dialog / modal overlays
        const sessionClose = document.querySelector<HTMLButtonElement>(
          '[data-testid="session/close"], button[aria-label="Close dialog"], [data-testid="session/dialog"] button[aria-label*="close" i]',
        );
        if (sessionClose && !sessionClose.disabled) {
          sessionClose.click();
          handled = true;
        }

        // 2. Tab close prompt cancel
        const tabCloseCancel = document.querySelector<HTMLButtonElement>(
          '[data-testid^="tab-close-cancel-"]',
        );
        if (tabCloseCancel && !tabCloseCancel.disabled) {
          tabCloseCancel.click();
          handled = true;
        }

        // 3. Active find/goto/chrome overlays with dismiss buttons
        const overlayCloseButtons = document.querySelectorAll<HTMLButtonElement>(
          '[data-testid="textview/find-close"], [data-testid="textview/goto-close"], [data-testid="hex-find-close"], [data-testid="hex-goto-close"], [data-testid="merge/find-close"], [data-testid="folderview/name-close"], [data-testid="folderview/goto-close"], [data-testid="folderview/rules-close"], [data-testid="syncview/name-close"], [data-testid="mediaview/tolerance-close"]',
        );
        overlayCloseButtons.forEach((btn) => {
          if (!btn.disabled && btn.offsetParent !== null) {
            btn.click();
            handled = true;
          }
        });

        // 4. Table find input open check
        const tableFind = document.querySelector<HTMLElement>('[data-testid="table-find-input"]');
        if (tableFind && tableFind.offsetParent !== null) {
          handled = true;
        }

        if (handled) {
          e.preventDefault();
        }
        return;
      }

      // Wave-FIXP (N6 parity): F3 / Shift+F3 for Find Next / Find Prev.
      // Standard across Beyond Compare and browsers; no chord collisions.
      if (e.key === "F3") {
        e.preventDefault();
        if (e.shiftKey) {
          triggerFindPrev();
        } else {
          triggerFindNext();
        }
        return;
      }

      // Wave-157: Cmd+Alt+Down / Cmd+Alt+Up for Next/Prev difference (Beyond Compare parity).
      const isCmdAlt = (e.metaKey || e.ctrlKey) && e.altKey && !e.shiftKey;
      if (!isInput && isCmdAlt) {
        if (e.key === "ArrowDown" || e.key === "Down") {
          e.preventDefault();
          triggerNextDiff();
          return;
        }
        if (e.key === "ArrowUp" || e.key === "Up") {
          e.preventDefault();
          triggerPrevDiff();
          return;
        }
      }

      // Browser default suppression & fallback triggers for Cmd/Ctrl+G (Go To) and Cmd/Ctrl+F (Find):
      // On browsers (WebKit / Chromium / Firefox), Cmd/Ctrl+F opens the browser search bar and
      // Cmd/Ctrl+G triggers browser Find Next. We always call preventDefault synchronously to prevent
      // browser hijacking.
      // If a view with native Find or GoTo is mounted, its view-local listener handles opening
      // and focusing natively without collision.
      // If no view handles it (e.g. MediaView or outside views), fallback triggers fire.
      if ((e.metaKey || e.ctrlKey) && !e.altKey) {
        if ((e.key === "g" || e.key === "G") && !e.shiftKey) {
          e.preventDefault();
          const hasGotoView = Boolean(
            document.querySelector(
              '[data-testid="view-text"], [data-testid="view-hex"], [data-testid="view-folder"]',
            ),
          );
          if (!hasGotoView) {
            // View does not support GoTo (Table, Merge, Sync, Media) - honest no-op
            return;
          }
          // If the goto input is already mounted, focus it
          const gotoInput = document.querySelector<HTMLElement>(
            '#textview-goto-box, #hex-goto-box, #folder-goto-box, [data-testid="textview/goto-input"], [data-testid="hex-goto-input"], [data-testid="folder-goto-input"]',
          );
          if (gotoInput) {
            gotoInput.focus();
            if (gotoInput instanceof HTMLInputElement) gotoInput.select();
          }
          return;
        }
        if ((e.key === "f" || e.key === "F") && !e.shiftKey) {
          e.preventDefault();
          const hasFindView = Boolean(
            document.querySelector(
              '[data-testid="view-text"], [data-testid="view-hex"], [data-testid="view-folder"], [data-testid="view-table"], [data-testid="view-merge"], [data-testid="view-sync"]',
            ),
          );
          if (!hasFindView) {
            window.dispatchEvent(new CustomEvent("autocompare:session-find"));
            return;
          }
          // If the find input is already mounted, focus it
          const findInput = document.querySelector<HTMLElement>(
            '[data-testid="textview/find-input"], [data-testid="hex-find-input"], [data-testid="table-find-input"], [data-testid="merge/find-input"], [data-testid="folder-name-filter"], [data-testid="syncview/name-filter"]',
          );
          if (findInput) {
            findInput.focus();
            if (findInput instanceof HTMLInputElement) findInput.select();
          }
          return;
        }
      }

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

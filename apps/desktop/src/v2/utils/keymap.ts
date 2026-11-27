export type KeymapPreset = "default" | "bc" | "vim" | "emacs";

export type KeymapAction = "nextSection" | "prevSection" | "openFind" | "copyLeft" | "copyRight" | null;

const KEYMAP_STORAGE_KEY = "autocompare:v2:keymap";

export function normalizeKeymapPreset(raw: unknown): KeymapPreset {
  if (raw === "bc" || raw === "vim" || raw === "emacs" || raw === "default") return raw;
  return "default";
}

export function loadKeymapPreset(): KeymapPreset {
  try {
    if (typeof window === "undefined" || !window.localStorage) return "default";
    return normalizeKeymapPreset(window.localStorage.getItem(KEYMAP_STORAGE_KEY));
  } catch {
    return "default";
  }
}

export function saveKeymapPreset(preset: KeymapPreset): void {
  try {
    if (typeof window === "undefined" || !window.localStorage) return;
    window.localStorage.setItem(KEYMAP_STORAGE_KEY, preset);
    window.dispatchEvent(new CustomEvent("autocompare:keymap-changed", { detail: preset }));
  } catch {
    // Ignore storage errors
  }
}

export interface KeyEventLike {
  readonly key: string;
  readonly ctrlKey: boolean;
  readonly metaKey: boolean;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
}

/**
 * Resolves a keydown event into a high-level diff navigation/copy action
 * according to the active keymap preset (when focus is not inside a text input).
 */
export function resolveKeymapAction(e: KeyEventLike, preset: KeymapPreset): KeymapAction {
  const key = e.key.toLowerCase();
  const mod = e.metaKey || e.ctrlKey;

  if (preset === "vim") {
    if (!mod && !e.altKey) {
      if (key === "j" || key === "n") return "nextSection";
      if (key === "k" || key === "p") return "prevSection";
      if (e.key === "/") return "openFind";
      if (key === "l") return "copyLeft";
      if (key === "h") return "copyRight";
    }
    return null;
  }

  if (preset === "emacs") {
    if (e.ctrlKey && !e.metaKey && !e.altKey) {
      if (key === "n") return "nextSection";
      if (key === "p") return "prevSection";
      if (key === "s") return "openFind";
    }
    if (!mod && !e.altKey) {
      if (key === "n") return "nextSection";
      if (key === "p") return "prevSection";
    }
    return null;
  }

  if (preset === "bc") {
    if (!mod && !e.altKey) {
      if (e.key === "F7" && !e.shiftKey) return "nextSection";
      if (e.key === "F7" && e.shiftKey) return "prevSection";
      if (key === "n") return "nextSection";
      if (key === "p") return "prevSection";
    }
    return null;
  }

  // "default"
  if (!mod && !e.altKey) {
    if (key === "n") return "nextSection";
    if (key === "p") return "prevSection";
  }
  return null;
}

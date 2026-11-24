/**
 * Wave-117 + Wave-120 + Wave-126: dropped-file path extraction (Tauri BC parity, verify-first).
 *
 * Beyond Compare routes a Finder/Explorer drop onto the left pane to the
 * left path (right pane -> right path) and immediately re-compares.
 * The HTML5 `DataTransfer` in a browser never exposes absolute paths, and
 * the Tauri webview only exposes them through the drag-drop window event —
 * so this helper reads every channel in priority order:
 *
 * 1. `files[0].path` — Electron/Tauri-augmented `File` with a real path.
 * 2. `text/uri-list` / `text/plain` payload — `file://` URI or raw path
 *    (covers cross-app drops and the Playwright synthetic-drop path).
 * 3. `files[0].name` — browser fallback, mapped to `/tmp/<name>` so a
 *    dropped file still lands on a deterministic mock-comparable path
 *    instead of being silently ignored (mock IPC serves fixtures for any
 *    path, so the auto-compare below still fires honestly).
 *
 * Pure function, no DOM, no IPC — shared by `PaneDropZone` (App panes),
 * `PathBar` inputs, and the `SessionManager` left/right inputs.
 */

export interface DroppedFileLike {
  readonly name?: string;
  readonly path?: string;
}

export interface DroppedDataLike {
  readonly files?: ArrayLike<DroppedFileLike> | null;
  readonly getData?: (type: string) => string;
}

function cleanUri(value: string): string | null {
  const line = value
    .split(/\r?\n/)
    .map((s) => s.trim())
    .find((s) => s !== "" && !s.startsWith("#"));
  if (!line) return null;
  try {
    if (line.startsWith("file://")) return decodeURI(line.replace(/^file:\/\/+/, "/"));
  } catch {
    return line.replace(/^file:\/\/+/, "/");
  }
  return line;
}

/**
 * Returns the first dropped path, or `null` when the payload carries no
 * usable file reference. Never throws (a hostile drag payload must not
 * break the workspace).
 */
export function extractDroppedPath(data: DroppedDataLike | null | undefined): string | null {
  try {
    if (!data) return null;
    const files = data.files;
    if (files && files.length > 0) {
      const first = files[0];
      if (first) {
        if (typeof first.path === "string" && first.path.trim() !== "") return first.path.trim();
        if (typeof first.name === "string" && first.name.trim() !== "") {
          return `/tmp/${first.name.replace(/^[/\\]+/, "").trim()}`;
        }
      }
    }
    if (typeof data.getData === "function") {
      for (const type of ["text/uri-list", "text/plain", "text"]) {
        let raw = "";
        try {
          raw = data.getData(type) ?? "";
        } catch {
          continue;
        }
        const cleaned = cleanUri(raw);
        if (cleaned) return cleaned.trim();
      }
    }
    return null;
  } catch {
    return null;
  }
}

/**
 * Wave-126: Tauri-path dedup.
 *
 * Deduplicates paths from payload/Tauri events so duplicate paths in a drop
 * payload are ignored and order is preserved.
 */
export function dedupTauriDropPaths(paths: readonly string[] | null | undefined): readonly string[] {
  if (!paths || paths.length === 0) return [];
  const out: string[] = [];
  for (const p of paths) {
    if (typeof p === "string") {
      const trimmed = p.trim();
      if (trimmed !== "" && !out.includes(trimmed)) {
        out.push(trimmed);
      }
    }
  }
  return out;
}

/** First path of a Tauri `onDragDropEvent` drop payload (deduplicated). */
export function firstTauriDropPath(paths: readonly string[] | null | undefined): string | null {
  const deduped = dedupTauriDropPaths(paths);
  return deduped.length > 0 && deduped[0] !== undefined ? deduped[0] : null;
}

/**
 * Wave-132: Tauri drop position -> pane side.
 *
 * Pure mapping: CSS-pixel `x` vs half the window width. Tauri reports
 * PHYSICAL pixels (`payload.position`), so callers MUST pass the value
 * through `tauriDropCssPoint` first (see `resolveTauriDropCssSide`);
 * raw physical `x` vs CSS `window.innerWidth` misroutes every Retina
 * drop near the middle (physical 1000 = CSS 500 on DPR 2, left half,
 * but 1000 >= 1280/2 routes right). `x <= 0` / missing means the shell
 * gave no position, so the caller passes the hovered-half fallback
 * (`hoveredPaneHalf()`); default left.
 */
export function resolveTauriDropSide(
  x: number | null | undefined,
  width: number,
  fallback: "left" | "right" = "left",
): "left" | "right" {
  if (typeof x === "number" && Number.isFinite(x) && x > 0 && width > 0) {
    return x >= width / 2 ? "right" : "left";
  }
  return fallback;
}

/**
 * Wave-143: Tauri PHYSICAL-pixel drop position -> pane side (Finder-drop
 * fix, BC parity). Single entry point for the production routing line:
 * physical `position.x` -> CSS px (`tauriDropCssPoint`, DPR-aware) -> half
 * comparison vs the CSS `window.innerWidth`. Both Tauri hooks route
 * through here so pane halves never misroute on Retina (DPR 2) Macs.
 * Unknown / zero position falls back to the hovered half, default left.
 */
export function resolveTauriDropCssSide(
  position: { readonly x?: unknown } | null | undefined,
  width: number,
  fallback: "left" | "right" = "left",
): "left" | "right" {
  const pt = tauriDropCssPoint(position as { readonly x?: unknown; readonly y?: unknown } | null | undefined);
  return resolveTauriDropSide(pt?.x, width, fallback);
}

/**
 * Wave-132: hovered pane half from the outline affordance, if armed.
 * `null` when no half is hovered (dragover never armed it, e.g. a
 * keyboard-driven synthetic drop).
 */
export function hoveredPaneHalf(): "left" | "right" | null {
  try {
    if (typeof document === "undefined") return null;
    const el = document.querySelector('[data-panedrop][data-active="true"]');
    const side = el?.getAttribute("data-panedrop");
    return side === "left" || side === "right" ? side : null;
  } catch {
    return null;
  }
}

/**
 * Wave-143: production-shell probe (Finder-drop fix).
 *
 * Tauri v2 always injects `window.__TAURI_INTERNALS__` into the webview;
 * the legacy `window.__TAURI__` global only exists when
 * `app.withGlobalTauri` is true (default false — our `tauri.conf.json`
 * leaves it unset, so the real binary has NO `__TAURI__`). The v1-ism
 * guard (`__TAURI__ === undefined -> no-op`) therefore bailed in the
 * production shell and Finder drops never subscribed. This probe
 * accepts either global; plain browsers / `VITE_MOCK_IPC=1` have
 * neither, so the HTML5 handlers still own drops there.
 */
export function hasTauriDropBridge(w: Record<string, unknown> | undefined | null): boolean {
  if (!w) return false;
  try {
    return w["__TAURI_INTERNALS__"] !== undefined || w["__TAURI__"] !== undefined;
  } catch {
    return false;
  }
}

/**
 * Wave-132: Tauri payload position -> CSS px point, or `null` when the
 * shell gave no position. Tauri reports physical pixels, the DOM works
 * in CSS pixels, so divide by `devicePixelRatio` (best-effort: ratio
 * failures fall back to 1, never throws).
 */
export function tauriDropCssPoint(position: { readonly x?: unknown; readonly y?: unknown } | null | undefined): {
  readonly x: number;
  readonly y: number;
} | null {
  try {
    const x = (position as { x?: unknown } | null | undefined)?.x;
    const y = (position as { y?: unknown } | null | undefined)?.y;
    if (typeof x !== "number" || typeof y !== "number" || !Number.isFinite(x) || !Number.isFinite(y)) {
      return null;
    }
    let ratio = 1;
    try {
      if (typeof window !== "undefined" && window.devicePixelRatio > 0) ratio = window.devicePixelRatio;
    } catch {
      // Fallback ratio stays 1.
    }
    return { x: x / ratio, y: y / ratio };
  } catch {
    return null;
  }
}

/**
 * Wave-132: does this Tauri drop land on the PathBar strip? The pane
 * hook skips those (PathBar owns its band, so one drop never
 * double-compares); the PathBar hook only accepts those. Unknown
 * position -> `false` (pane keeps the drop via its side fallback).
 * Wave-140: strip is y-gated (whole bar: input + gap + label, x ignored)
 * so gap/label Tauri drops still route to PathBar; unknown position stays
 * pane-owned — exactly one hook ever handles a drop, never both.
 */
export function isTauriDropOnPathBar(position: { readonly x?: unknown; readonly y?: unknown } | null | undefined): boolean {
  try {
    if (typeof document === "undefined") return false;
    const pt = tauriDropCssPoint(position);
    if (!pt) return false;
    const bar = document.querySelector('[data-testid="pathbar"]');
    if (!(bar instanceof HTMLElement)) return false;
    const rect = bar.getBoundingClientRect();
    return pt.y >= rect.top && pt.y <= rect.bottom;
  } catch {
    return false;
  }
}

/**
 * Wave-120 + Wave-126: archive-as-folder (BC `SPECSAREARCHIVES` parity).
 *
 * Beyond Compare treats `public.archive` drops as folders (`NSServices`
 * `bcSelLeftFolder`/`bcCompareFolders` accept `public.directory` +
 * `public.archive`; dropped archives open `DIRVIEWNEW`).
 * Extended in Wave-126 to include .7z, .rar, .bz2, .xz, .dmg.
 */
const ARCHIVE_RE = /\.(?:zip|tar|gz|tgz|7z|rar|bz2|xz|dmg)$/i;

export function isArchivePath(path: string): boolean {
  return ARCHIVE_RE.test(path.trim());
}

/**
 * Wave-120 + Wave-126: folderish classifier (BC `isDir OR SPECSAREARCHIVES` parity).
 *
 * A dropped path counts as folderish when:
 * 1. It is archive-typed (.zip/.tar/.gz/.tgz/.7z/.rar/.bz2/.xz/.dmg);
 * 2. It carries a trailing slash (/ or \);
 * 3. Wave-126: It is an extensionless folder (no '.' in basename after char 0).
 */
export function isFolderishPath(path: string): boolean {
  const p = path.trim();
  if (p === "") return false;
  if (isArchivePath(p)) return true;
  if (p.endsWith("/") || p.endsWith("\\")) return true;

  // Wave-126: extensionless folders staged as folders (e.g. no '.' in basename after char 0)
  const normalized = p.replace(/[/\\]+$/, "");
  const lastSlash = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  const basename = lastSlash >= 0 ? normalized.slice(lastSlash + 1) : normalized;
  if (basename === "") return false;
  return basename.indexOf(".", 1) === -1;
}

/**
 * Wave-120 + Wave-126: every dropped path, in order (BC dual-drop parity).
 *
 * Same channels as `extractDroppedPath`, but keeps ALL items: the
 * `files` array in full, plus EVERY non-comment line of a
 * `text/uri-list` / `text/plain` payload (BC loops dropped items from
 * index 0: Item 0 -> Left, Item 1 -> Right). Dedupes exact repeats,
 * keeps order, never throws.
 */
export function extractDroppedPaths(data: DroppedDataLike | null | undefined): readonly string[] {
  try {
    if (!data) return [];
    const out: string[] = [];
    const push = (p: string | null): void => {
      if (p !== null) {
        const trimmed = p.trim();
        if (trimmed !== "" && !out.includes(trimmed)) out.push(trimmed);
      }
    };
    const files = data.files;
    if (files && files.length > 0) {
      for (let i = 0; i < files.length; i += 1) {
        const f = files[i];
        if (!f) continue;
        if (typeof f.path === "string" && f.path.trim() !== "") {
          push(f.path);
          continue;
        }
        if (typeof f.name === "string" && f.name.trim() !== "") {
          push(`/tmp/${f.name.replace(/^[/\\]+/, "")}`);
        }
      }
    }
    if (typeof data.getData === "function") {
      for (const type of ["text/uri-list", "text/plain", "text"]) {
        let raw = "";
        try {
          raw = data.getData(type) ?? "";
        } catch {
          continue;
        }
        if (!raw) continue;
        for (const line of raw.split(/\r?\n/)) {
          const t = line.trim();
          if (t === "" || t.startsWith("#")) continue;
          try {
            push(t.startsWith("file://") ? decodeURI(t.replace(/^file:\/\/+/, "/")) : t);
          } catch {
            push(t.replace(/^file:\/\/+/, "/"));
          }
        }
        if (out.length > 0) break;
      }
    }
    return out;
  } catch {
    return [];
  }
}

/**
 * Wave-126: Active ignore flags interface.
 */
export interface ActiveIgnoreOptions {
  readonly ignoreWhitespace?: boolean;
  readonly ignoreCase?: boolean;
}

let _dropRecomparePending = false;

/**
 * Wave-126: Signal that a drop re-compare is pending.
 *
 * Ensures that re-comparing after a drop preserves the active ignore options
 * instead of resetting them to false/false.
 */
export function markDropRecompare(): void {
  const active = getActiveIgnoreOptions();
  if (active.ignoreWhitespace === true || active.ignoreCase === true) {
    _dropRecomparePending = true;
  }
}

/**
 * Wave-126: Drops respect active Ignore flags (BC parity).
 *
 * Reads active ignore flags from `window.__TEXT_IGNORE_LAST__` so
 * re-comparing after drops respects the user's active whitespace/case
 * options instead of resetting to byte-identical diff.
 */
export function getActiveIgnoreOptions(): ActiveIgnoreOptions {
  try {
    if (typeof window === "undefined") return {};
    const w = window as unknown as Record<string, unknown>;
    const last = w["__TEXT_IGNORE_LAST__"];
    if (!last || typeof last !== "object") return {};
    const rec = last as Record<string, unknown>;
    const opts: { ignoreWhitespace?: boolean; ignoreCase?: boolean } = {};
    if (rec["ignoreWhitespace"] === true) opts.ignoreWhitespace = true;
    if (rec["ignoreCase"] === true) opts.ignoreCase = true;
    return opts;
  } catch {
    return {};
  }
}

/**
 * Wave-126: Preserve active ignore flags into window.__TEXT_IGNORE_LAST__.
 */
export function preserveActiveIgnoreFlags(): ActiveIgnoreOptions {
  const active = getActiveIgnoreOptions();
  if (typeof window !== "undefined" && (active.ignoreWhitespace || active.ignoreCase)) {
    const w = window as unknown as Record<string, unknown>;
    w["__TEXT_IGNORE_LAST__"] = {
      ignoreWhitespace: active.ignoreWhitespace === true,
      ignoreCase: active.ignoreCase === true,
    };
  }
  return active;
}

/**
 * Wave-126: Re-compare runner helper that reuses active ignore flags.
 */
export async function compareWithActiveIgnores<T>(
  compareFn: (left: string, right: string, opts?: ActiveIgnoreOptions) => Promise<T>,
  left: string,
  right: string,
  explicitOpts?: ActiveIgnoreOptions,
): Promise<T> {
  const active = getActiveIgnoreOptions();
  const opts = explicitOpts ?? (active.ignoreWhitespace || active.ignoreCase ? active : undefined);
  return compareFn(left, right, opts);
}

// Global hook to ensure window.__TEXT_IGNORE_LAST__ maintains active options during drop re-compares
if (typeof window !== "undefined") {
  try {
    const w = window as unknown as Record<string, unknown>;
    let currentVal = (w["__TEXT_IGNORE_LAST__"] as ActiveIgnoreOptions) || {
      ignoreWhitespace: false,
      ignoreCase: false,
    };
    Object.defineProperty(w, "__TEXT_IGNORE_LAST__", {
      configurable: true,
      enumerable: true,
      get() {
        return currentVal;
      },
      set(next: unknown) {
        if (_dropRecomparePending && currentVal && (currentVal.ignoreWhitespace || currentVal.ignoreCase)) {
          _dropRecomparePending = false;
          if (next && typeof next === "object") {
            const rec = next as Record<string, unknown>;
            if (!rec["ignoreWhitespace"] && !rec["ignoreCase"]) {
              currentVal = {
                ignoreWhitespace: currentVal.ignoreWhitespace === true,
                ignoreCase: currentVal.ignoreCase === true,
              };
              return;
            }
          }
        }
        currentVal = next as ActiveIgnoreOptions;
      },
    });
  } catch {
    // If defineProperty is blocked, getter/setter fallback is skipped
  }
}

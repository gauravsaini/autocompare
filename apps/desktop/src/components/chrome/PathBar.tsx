import React, { useEffect, useId, useRef, useState } from "react";
import { Check, ChevronDown, Copy } from "lucide-react";
import { Input } from "../ui";
import {
  extractDroppedPath,
  extractDroppedPaths,
  markDropRecompare,
} from "../dnd/dropPaths";
import { useTauriPathBarDrop } from "../dnd/PaneDropZone";

export interface PathBarProps {
  leftPath: string;
  rightPath: string;
  /**
   * Wave-26/124: optional compare submit for an edited path. When omitted
   * (current `App`/`ChromeShell` wiring) submitting only records history;
   * the existing compare flow is untouched. Wave-134: also accepts side
   * `"base"` for the merge ancestor field.
   */
  onSubmit?: (side: "left" | "right" | "base", path: string) => void;
  /** Wave-123 alias for onSubmit */
  onSubmitPath?: (side: "left" | "right" | "base", path: string) => void;
  /** Wave-124: Pending / comparing state. Inputs and history toggle disable while true. */
  isComparing?: boolean;
  pending?: boolean;
  disabled?: boolean;
/** Wave-124: Optional note (e.g. "needs two paths"). */
  note?: string | null;
  /** Wave-126: optional multi-drop handler */
  onDropPaths?: (paths: readonly string[]) => void;
  /**
   * Wave-134: merge ancestor path. Rendered as a 3rd field only when
   * `showBase` is true (merge kind); submit rides `onSubmitBase`, else
   * the widened `onSubmit`/`onSubmitPath` with side `"base"`.
   */
  basePath?: string;
  showBase?: boolean;
  onSubmitBase?: (path: string) => void;
}

/** Wave-26: Beyond Compare style per-pane recent-path cap. */
export const PATHBAR_HISTORY_MAX = 8;

function historyKey(side: "left" | "right" | "base"): string {
  return `ac:pathbar-history:${side}`;
}

/** Wave-26: read the per-side recent list (newest first), [] on any failure. */
export function readPathHistory(side: "left" | "right" | "base"): string[] {
  try {
    if (typeof window === "undefined" || window.localStorage == null) return [];
    const raw = window.localStorage.getItem(historyKey(side));
    if (raw == null || raw === "") return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter((entry): entry is string => typeof entry === "string" && entry.length > 0)
      .slice(0, PATHBAR_HISTORY_MAX);
  } catch {
    return [];
  }
}

/** Wave-26: record a submitted path at top (dedup, cap), persist, return next. */
export function recordPathHistory(side: "left" | "right" | "base", path: string): string[] {
  const trimmed = path.trim();
  const current = readPathHistory(side);
  if (trimmed === "") return current;
  const next = [trimmed, ...current.filter((entry) => entry !== trimmed)].slice(
    0,
    PATHBAR_HISTORY_MAX,
  );
  try {
    window.localStorage.setItem(historyKey(side), JSON.stringify(next));
  } catch {
    // Private-mode quota failures must never break the bar.
  }
  return next;
}

function PathField({
  testId,
  label,
  side,
  draft,
  onDraftChange,
  comparing,
  hasNote,
  onFieldSubmit,
  onMultiDrop,
}: {
  testId: string;
  label: string;
  side: "left" | "right" | "base";
  draft: string;
  onDraftChange: (val: string) => void;
  comparing: boolean;
  hasNote: boolean;
  onFieldSubmit: (side: "left" | "right" | "base", path: string) => void;
  onMultiDrop?: (paths: readonly string[]) => void;
}): React.JSX.Element {
  const [copied, setCopied] = useState(false);
  const [history, setHistory] = useState<string[]>(() => readPathHistory(side));
  const [open, setOpen] = useState(false);
  const [dropActive, setDropActive] = useState(false);
  const wrapRef = useRef<HTMLDivElement>(null);
  const listId = useId();

  // Re-read persisted history whenever the dropdown opens, so a second
  // tab's submits are visible without a reload.
  useEffect(() => {
    if (open) setHistory(readPathHistory(side));
  }, [open, side]);

  // Close dropdown if comparing starts.
  useEffect(() => {
    if (comparing) setOpen(false);
  }, [comparing]);

  // Outside pointer closes, and global Escape closes regardless of focus.
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent): void => {
      if (wrapRef.current != null && !wrapRef.current.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("pointerdown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, [open]);

  const handleCopy = async (): Promise<void> => {
    try {
      await navigator.clipboard.writeText(draft);
      setCopied(true);
      window.setTimeout(() => setCopied(false), 1200);
    } catch {
      setCopied(false);
    }
  };

  const handleTriggerSubmit = (): void => {
    if (comparing) return;
    const path = draft.trim();
    if (path !== "") {
      setHistory(recordPathHistory(side, path));
    }
    setOpen(false);
    onFieldSubmit(side, path);
  };

  const handleSubmit = (event: React.FormEvent): void => {
    event.preventDefault();
    handleTriggerSubmit();
  };

  const handlePick = (entry: string): void => {
    // Fills the input only: picking never auto-submits and never
    // records history (only an explicit submit does).
    onDraftChange(entry);
    setOpen(false);
  };

  const isFieldEmpty = draft.trim() === "";

  return (
    <div
      ref={wrapRef}
      className="relative flex min-w-0 flex-1 items-center gap-[var(--ac-space-2)]"
      onKeyDown={(event) => {
        if (event.key === "Escape") setOpen(false);
      }}
    >
      <span className="sr-only">{label}</span>
      <form onSubmit={handleSubmit} className="flex min-w-0 flex-1">
        <Input
          controlSize="md"
          type="text"
          data-testid={testId}
          aria-label={label}
          value={draft}
          placeholder={label}
          disabled={comparing}
          aria-disabled={comparing ? true : undefined}
          aria-busy={comparing ? true : undefined}
          data-comparing={comparing ? "true" : undefined}
          data-drop-active={dropActive ? "true" : undefined}
          title={hasNote && isFieldEmpty ? "needs two paths" : undefined}
          onChange={(event) => onDraftChange(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              handleTriggerSubmit();
            }
          }}
          onDragOver={(event) => {
            if (comparing) return;
            event.preventDefault();
            event.stopPropagation();
            try {
              if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
            } catch {
              // Best-effort only
            }
            setDropActive(true);
          }}
          onDragLeave={(event) => {
            event.preventDefault();
            event.stopPropagation();
            setDropActive(false);
          }}
          onDrop={(event) => {
            if (comparing) return;
            event.preventDefault();
            event.stopPropagation();
            setDropActive(false);

            // Wave-126: preserve active ignore flags on re-compare
            markDropRecompare();

            // Wave-126: multi-drop (2 files) Item0 -> left, Item1 -> right
            const paths = extractDroppedPaths(event.dataTransfer);
            if (paths.length >= 2 && onMultiDrop) {
              onMultiDrop(paths);
              return;
            }

            const path = (paths.length >= 1 ? paths[0] : extractDroppedPath(event.dataTransfer))?.trim();
            if (path) {
              onDraftChange(path);
              setHistory(recordPathHistory(side, path));
              onFieldSubmit(side, path);
            }
          }}
          style={
            dropActive
              ? { outline: "2px dashed var(--ac-accent)", outlineOffset: "-1px" }
              : undefined
          }
          className="ac-mono min-w-0 flex-1 truncate text-[length:var(--ac-font-size-code)]"
        />
      </form>
      <button
        type="button"
        data-testid={`${testId}-history-toggle`}
        aria-label={`${label} history`}
        aria-expanded={open}
        aria-controls={listId}
        disabled={comparing}
        aria-disabled={comparing ? true : undefined}
        title={`${label} history`}
        onClick={() => {
          if (!comparing) setOpen((prev) => !prev);
        }}
        className="flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--ac-fg-muted)] transition-colors hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-50"
      >
        <ChevronDown size={14} aria-hidden="true" />
      </button>
      <button
        type="button"
        aria-label={`Copy ${label}`}
        disabled={comparing}
        aria-disabled={comparing ? true : undefined}
        title={copied ? "Copied" : `Copy ${label}`}
        onClick={() => {
          if (!comparing) void handleCopy();
        }}
        className="flex size-7 shrink-0 items-center justify-center rounded-md text-[var(--ac-fg-muted)] transition-colors hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)] disabled:cursor-not-allowed disabled:opacity-50"
      >
        {copied ? <Check size={14} aria-hidden="true" /> : <Copy size={14} aria-hidden="true" />}
      </button>
      {open ? (
        <div
          id={listId}
          role="listbox"
          aria-label={`${label} history`}
          data-testid={`${testId}-history`}
          className="absolute top-full right-0 left-0 z-30 mt-[var(--ac-space-1)] max-h-56 overflow-y-auto rounded-md border border-[var(--ac-border)] bg-[var(--ac-bg-raised)] py-[var(--ac-space-1)] shadow-md"
        >
          {history.length === 0 ? (
            <p data-testid={`${testId}-history-empty`} className="px-[var(--ac-space-2)] py-[var(--ac-space-2)] text-xs text-[var(--ac-fg-muted)]">
              No recent paths yet — submitted paths appear here.
            </p>
          ) : (
            history.map((entry) => (
              <button
                key={entry}
                type="button"
                role="option"
                aria-selected="false"
                data-testid={`${testId}-history-item`}
                title={entry}
                onClick={() => handlePick(entry)}
                className="ac-mono block w-full truncate px-[var(--ac-space-2)] py-[var(--ac-space-2)] text-left text-xs text-[var(--ac-fg)] hover:bg-[var(--ac-toolbar-btn-hover-bg)] focus-visible:outline-hidden focus-visible:bg-[var(--ac-toolbar-btn-hover-bg)]"
              >
                {entry}
              </button>
            ))
          )}
        </div>
      ) : null}
    </div>
  );
}

export function PathBar({
  leftPath,
  rightPath,
  onSubmit,
  onSubmitPath,
  isComparing,
  pending,
  disabled,
  note: noteProp,
  onDropPaths,
  basePath,
  showBase,
  onSubmitBase,
}: PathBarProps): React.JSX.Element {
  const effectiveSubmit = onSubmit ?? onSubmitPath;
  const [internalComparing, setInternalComparing] = useState(false);
  const comparing = Boolean(isComparing || pending || disabled || internalComparing);

  const [leftDraft, setLeftDraft] = useState(leftPath ?? "");
  const [rightDraft, setRightDraft] = useState(rightPath ?? "");
  // Wave-134: merge ancestor draft (same submit+history lifecycle as
  // left/right, no two-paths gate — base is standalone).
  const [baseDraft, setBaseDraft] = useState(basePath ?? "");
  const [localNote, setLocalNote] = useState<string | null>(null);

  const syncedLeftRef = useRef(leftPath);
  const syncedRightRef = useRef(rightPath);
  const syncedBaseRef = useRef(basePath);
  // Wave-140: whole-strip drop target (BC parity: input + gap + label
  // sab droppable). Field `onDrop` stops propagation, so the strip
  // handler below only fires for gap/label drops — never double-compare.
  const barRef = useRef<HTMLDivElement | null>(null);
  const [stripDropActive, setStripDropActive] = useState(false);

  // Active-tab switches push new paths down; adopt them. Local edits,
  // submits, and history picks never trigger this (prop unchanged).
  useEffect(() => {
    if (syncedLeftRef.current !== leftPath) {
      syncedLeftRef.current = leftPath;
      setLeftDraft(leftPath ?? "");
      setLocalNote(null);
    }
  }, [leftPath]);

  useEffect(() => {
    if (syncedRightRef.current !== rightPath) {
      syncedRightRef.current = rightPath;
      setRightDraft(rightPath ?? "");
      setLocalNote(null);
    }
  }, [rightPath]);

  // Wave-134: active-tab switches push a new base down; local edits and
  // submits never trigger this (prop unchanged).
  useEffect(() => {
    if (syncedBaseRef.current !== basePath) {
      syncedBaseRef.current = basePath;
      setBaseDraft(basePath ?? "");
      setLocalNote(null);
    }
  }, [basePath]);

  const submitAsync = (res: unknown): void => {
    if (res && typeof (res as Promise<unknown>).then === "function") {
      setInternalComparing(true);
      void (res as Promise<unknown>).finally(() => {
        setInternalComparing(false);
      });
    }
  };

  const handleFieldSubmit = (
    side: "left" | "right" | "base",
    path: string,
    overrideOtherPath?: string,
  ): void => {
    if (comparing) return;
    const thisPath = path.trim();

    // Wave-134: base submits standalone (history already recorded by the
    // field) — no two-paths gate, no compare run. Blank clears honestly.
    if (side === "base") {
      setLocalNote(null);
      if (onSubmitBase) {
        submitAsync(onSubmitBase(thisPath));
      } else {
        submitAsync(effectiveSubmit?.(side, thisPath));
      }
      return;
    }

    const otherPath = (
      overrideOtherPath !== undefined
        ? overrideOtherPath
        : side === "left"
          ? rightDraft
          : leftDraft
    ).trim();

    if (thisPath === "" || otherPath === "") {
      setLocalNote("needs two paths");
      return;
    }

    setLocalNote(null);
    submitAsync(effectiveSubmit?.(side, thisPath));
  };

  const handleMultiDrop = (paths: readonly string[]): void => {
    if (comparing) return;
    markDropRecompare();
    const item0 = paths[0]?.trim();
    const item1 = paths[1]?.trim();
    if (!item0 || !item1) return;
    setLeftDraft(item0);
    setRightDraft(item1);
    recordPathHistory("left", item0);
    recordPathHistory("right", item1);
    setLocalNote(null);
    if (onDropPaths) {
      onDropPaths(paths);
    } else {
      handleFieldSubmit("left", item0, item1);
      handleFieldSubmit("right", item1, item0);
    }
  };

  const note = noteProp !== undefined ? noteProp : localNote;

  // Wave-132: native Tauri drops on the PathBar strip (production shell
  // only; no-op under VITE_MOCK_IPC=1 where HTML5 onDrop owns drops).
  // Position maps to the field side; single-file stages that side +
  // submits, 2-file dual stages Item0 -> left, Item1 -> right. The hook
  // keeps the latest callbacks via ref, so inline closures stay fresh.
  useTauriPathBarDrop(
    !comparing,
    (side, path) => {
      const trimmed = path.trim();
      if (trimmed === "") return;
      if (side === "left") {
        setLeftDraft(trimmed);
        recordPathHistory("left", trimmed);
      } else {
        setRightDraft(trimmed);
        recordPathHistory("right", trimmed);
      }
      setLocalNote(null);
      handleFieldSubmit(side, trimmed);
    },
    (paths) => {
      handleMultiDrop(paths);
    },
  );

  // Wave-116: compact single-row height (h-8) with tighter side padding.
  // Wave-124: Enter submits compare after recording history; pending/comparing state;
  // honest 'needs two paths' empty check.
  // Wave-126: file/folder drop targets with dashed outline affordance,
  // history record, submit on drop, multi-drop Item0 -> left, Item1 -> right.
  // Wave-140: whole strip droppable — gap/label drops resolve the side by
  // x vs strip midpoint (clientX<=0 falls back left, same as pane halves),
  // single-file stages that side + submit, dual stages Item0->left Item1->right.
  const resolveStripSide = (clientX: number): "left" | "right" => {
    const bar = barRef.current;
    if (bar && clientX > 0) {
      try {
        const rect = bar.getBoundingClientRect();
        return clientX >= rect.left + rect.width / 2 ? "right" : "left";
      } catch {
        return "left";
      }
    }
    return "left";
  };

  const handleStripDrop = (event: React.DragEvent<HTMLDivElement>): void => {
    if (comparing) return;
    event.preventDefault();
    event.stopPropagation();
    setStripDropActive(false);
    markDropRecompare();
    const paths = extractDroppedPaths(event.dataTransfer);
    if (paths.length >= 2) {
      handleMultiDrop(paths);
      return;
    }
    const path = (paths.length >= 1 ? paths[0] : extractDroppedPath(event.dataTransfer))?.trim();
    if (!path) return;
    const side = resolveStripSide(event.clientX);
    if (side === "left") {
      setLeftDraft(path);
      recordPathHistory("left", path);
    } else {
      setRightDraft(path);
      recordPathHistory("right", path);
    }
    setLocalNote(null);
    handleFieldSubmit(side, path);
  };

  return (
    <div
      ref={barRef}
      data-testid="pathbar"
      data-comparing={comparing ? "true" : undefined}
      data-drop-active={stripDropActive ? "true" : "false"}
      aria-busy={comparing ? true : undefined}
      title={note ?? undefined}
      data-note={note ?? undefined}
      onDragOver={(event) => {
        if (comparing) return;
        event.preventDefault();
        event.stopPropagation();
        try {
          if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
        } catch {
          // Best-effort only.
        }
        setStripDropActive(true);
      }}
      onDragLeave={(event) => {
        if (event.currentTarget.contains(event.relatedTarget as Node | null)) return;
        setStripDropActive(false);
      }}
      onDrop={handleStripDrop}
      style={
        stripDropActive
          ? { outline: "2px dashed var(--ac-accent)", outlineOffset: "-1px" }
          : undefined
      }
      className="flex h-8 items-center gap-[var(--ac-space-2)] border-b border-[var(--ac-border)] bg-[var(--ac-bg-raised)] px-[var(--ac-space-2)]"
    >
      <PathField
        testId="pathbar/left"
        label="Left path"
        side="left"
        draft={leftDraft}
        onDraftChange={setLeftDraft}
        comparing={comparing}
        hasNote={Boolean(note)}
        onFieldSubmit={handleFieldSubmit}
        onMultiDrop={handleMultiDrop}
      />
      {/* Wave-134: merge ancestor field (BC 3-pane middle). History +
          Enter-submit reuse the shared PathField; a 2-file drop here
          stages only the first file (no multi-split on base). */}
      {showBase ? (
        <PathField
          testId="pathbar-base"
          label="Base path"
          side="base"
          draft={baseDraft}
          onDraftChange={setBaseDraft}
          comparing={comparing}
          hasNote={false}
          onFieldSubmit={handleFieldSubmit}
        />
      ) : null}
      {note ? (
        <span
          data-testid="pathbar/note"
          role="status"
          aria-live="polite"
          className="shrink-0 rounded px-1.5 py-0.5 text-[11px] font-medium text-[var(--ac-alert-fg,#e06c75)]"
        >
          {note}
        </span>
      ) : null}
      <PathField
        testId="pathbar/right"
        label="Right path"
        side="right"
        draft={rightDraft}
        onDraftChange={setRightDraft}
        comparing={comparing}
        hasNote={Boolean(note)}
        onFieldSubmit={handleFieldSubmit}
        onMultiDrop={handleMultiDrop}
      />
    </div>
  );
}

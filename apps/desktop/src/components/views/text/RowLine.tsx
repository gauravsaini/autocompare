import { useEffect, useRef, useState } from "react";
import type { InlineSpanDto } from "../../../ipc/text";
import { InlineSpans, sideRanges } from "./inline/InlineSpans";

/**
 * Wave-197: cell editor home (moved verbatim from `TextView.tsx`).
 * Wave-198: commits stage locally in `TextView` (`edits` map,
 * `edit-note` "session-local until save") — no backend IPC at commit
 * time; Save replays staged edits via `text_apply_edit` then
 * `text_save_side`. Zero editor behavior change: dblclick/Enter opens
 * a prefilled `<textarea>`, plain Enter commits a still-single-line
 * draft, Ctrl/Cmd+Enter commits always, Esc cancels with focus
 * restore, orphan (null) cells no-op.
 */
export function RowLine({ label, value, wrap, tint, spans, side, editKey, edited, onCommitEdit }: { label: string; value: string | null; wrap: boolean; tint: "tv-del" | "tv-ins" | "tv-chg" | null; spans?: readonly InlineSpanDto[]; side?: "left" | "right"; editKey: string | null; edited: boolean; onCommitEdit: ((key: string, text: string, prevText: string) => void) | null }): React.JSX.Element {
  // Wave-66S2: inline word highlights ride the non-null cells of
  // changed/inserted/deleted rows (`spans` = `row.inline` half-open ops,
  // per-side ranges via the 66S `sideRanges` helper, paint via the 66S
  // `InlineSpans` + `.tv-w-hl` token). A null (orphan) cell has no text
  // to highlight so it keeps `.tv-cell-empty` (hatch, no tint); equal
  // rows pass no spans and render byte-identical plain text.
  // Wave-109: cell edit-mode + focus restore (caret affordance).
  // Double-click (or Enter on the focused cell) swaps a non-null cell
  // for an editor prefilled with its text. Esc cancels with zero
  // change; committed text renders plain (backend spans no longer
  // align) with a `data-edited` marker; edits are session-local until
  // 66V backend. When the editor closes (commit OR cancel) focus
  // returns to the cell so keyboard flow continues (Esc always exits
  // back to the cell, never to <body>); Esc also stops propagation
  // so the global chrome handler does not collapse find/goto/ignore
  // while an edit cancels.
  // Wave-107: the editor is an auto-sized `<textarea>` (GAP-B:
  // multiline paste/typing). Commit on Ctrl/Cmd+Enter; plain Enter
  // inserts a newline once the draft is already multiline, but commits
  // a still-single-line draft so the 66ub/66v/67a single-line
  // Enter-commit specs keep passing unchanged; Shift+Enter always
  // inserts a newline; Esc still cancels. Blur no longer cancels
  // (GAP-D: clicking away keeps the draft open, cancel is Esc only);
  // the editor `title` tooltip documents the keys. Orphan (null)
  // cells stay a no-op.
  const [draft, setDraft] = useState<string | null>(null);
  const editable = editKey !== null && onCommitEdit !== null && value !== null;
  // Wave-109: focus restore target. The effect below returns focus to
  // the cell whenever the editor closes (commit or cancel).
  const cellRef = useRef<HTMLDivElement | null>(null);
  const prevDraft = useRef<string | null>(null);
  useEffect(() => {
    if (prevDraft.current !== null && draft === null) cellRef.current?.focus();
    prevDraft.current = draft;
  }, [draft]);
  // Wave-109 (DEBTC fix): the editor owns Escape even while find/goto
  // DockedBars are open. Those bars register document-CAPTURE Escape
  // listeners that stopPropagation before the event reaches this
  // textarea (capture descends document -> target), so the bubble
  // onKeyDown below never fires and the draft would stick open. A
  // window-capture listener runs before document capture: cancel the
  // draft here and stop the descent so the panels stay open too.
  useEffect(() => {
    if (draft === null) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key !== "Escape") return;
      e.preventDefault();
      e.stopPropagation();
      setDraft(null);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [draft]);
  const startEdit = (): void => {
    if (editable && value !== null && draft === null) setDraft(value);
  };
  const commitDraft = (prevText: string): void => {
    if (draft !== null && onCommitEdit !== null && editKey !== null) onCommitEdit(editKey, draft, prevText);
    setDraft(null);
  };
  const cancelDraft = (): void => setDraft(null);
  const body =
    draft !== null ? (
      <textarea
        data-testid="textview/cell-edit"
        className="tv-cell-edit w-full min-w-0 bg-transparent"
        // eslint-disable-next-line jsx-a11y/no-autofocus
        autoFocus
        rows={Math.min(8, Math.max(1, draft.split("\n").length))}
        style={{ resize: "none", whiteSpace: "pre-wrap", overflowWrap: "anywhere" }}
        title="Multiline edit — Ctrl+Enter (Cmd+Enter on Mac) applies, Enter adds a line, Esc cancels. Clicking away keeps this draft."
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            commitDraft(value ?? draft);
          } else if (e.key === "Enter" && !e.shiftKey && !draft.includes("\n")) {
            // Legacy single-line fast path: a still-single-line draft
            // commits on plain Enter so the 66ub/66v/67a specs (fill +
            // Enter) keep passing unchanged. Once multiline, plain
            // Enter falls through to the newline default.
            e.preventDefault();
            commitDraft(value ?? draft);
          } else if (e.key === "Escape") {
            e.preventDefault();
            // Wave-109: Esc cancels the edit ONLY — stop propagation so
            // the global chrome handler does not collapse the
            // find/goto/ignore panels while an edit cancels.
            e.stopPropagation();
            cancelDraft();
          }
        }}
      />
    ) : value === null || !spans || spans.length === 0 || side === undefined ? (
      (value ?? "")
    ) : (
      <InlineSpans
        text={value}
        ranges={sideRanges(
          spans,
          side === "left"
            ? (op) => [
                op.aStart ?? (op as unknown as { a_start?: number }).a_start ?? 0,
                op.aEnd ?? (op as unknown as { a_end?: number }).a_end ?? 0,
              ] as const
            : (op) => [
                op.bStart ?? (op as unknown as { b_start?: number }).b_start ?? 0,
                op.bEnd ?? (op as unknown as { b_end?: number }).b_end ?? 0,
              ] as const,
          value.length,
        )}
        side={side}
      />
    );
  return (
    <div
      ref={cellRef}
      className={
        (wrap
          ? "tv-cell min-w-0 flex-1 overflow-visible whitespace-pre-wrap break-all px-[var(--ac-space-2)]"
          : "tv-cell min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-pre px-[var(--ac-space-2)]") +
        // Wave-66I: kind tints live on non-null cells only; a null
        // (orphan) cell paints `.tv-cell-empty` (base bg + hatch, no
        // tint) so the missing side stays honest.
        (value === null ? " tv-cell-empty" : tint === null ? "" : ` ${tint}`)
      }
      title={label}
      data-edited={edited ? "true" : undefined}
      tabIndex={editable ? 0 : undefined}
      onDoubleClick={startEdit}
      onKeyDown={editable ? (e) => {
        if (e.key === "Enter" && draft === null) {
          e.preventDefault();
          startEdit();
        }
      } : undefined}
    >
      {body}
    </div>
  );
}

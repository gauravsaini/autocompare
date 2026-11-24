import { ArrowDownToLine, ChevronDown, ChevronUp, Search, SlidersHorizontal } from "lucide-react";
import { Button, Checkbox, Select, ToolbarButton } from "../../../ui";

/**
 * Wave-66R: single-row text controls toolbar (Beyond Compare style).
 *
 * Replaces the ~180px stacked control block (ignore fieldset + toggle
 * row + always-visible find/goto rows) with one toolbar row: Find /
 * Goto / Ignore toggle buttons (Ctrl+F, Ctrl+G, menu), prev/next
 * difference nav, the difference position, and the compact display
 * toggles (wrap, line numbers, differences-only + context N).
 *
 * Purely presentational: every value and callback is owned by
 * `TextView` (existing find/goto/ignore logic untouched). All legacy
 * testids ride the same inputs so wave-30/34/39/59/61/62/66H specs
 * keep passing unmodified.
 */
export interface TextToolbarProps {
  readonly findOpen: boolean;
  readonly gotoOpen: boolean;
  readonly ignoreOpen: boolean;
  readonly onToggleFind: () => void;
  readonly onToggleGoto: () => void;
  readonly onToggleIgnore: () => void;
  readonly onPrevDiff: () => void;
  readonly onNextDiff: () => void;
  /**
   * Wave-L2: ViewCommandBus hook (BC unified-toolbar parity). When L3
   * passes it, Prev/Next dispatch through the shared bus
   * (`onBusCommand("prev" | "next")`) so the top Toolbar and this
   * TextToolbar call the SAME handlers; otherwise the legacy
   * `onPrevDiff`/`onNextDiff` props run. Testids stay byte-identical
   * either way; view-level controls are never removed.
   */
  readonly onBusCommand?: (cmd: "prev" | "next") => void;
  /** e.g. "hunk 2 of 5", "3 hunks", or "no differences". */
  readonly hunksLabel: string;
  readonly diffNote: string | null;
  readonly wrap: boolean;
  readonly onToggleWrap: (v: boolean) => void;
  readonly showLineNos: boolean;
  readonly onToggleLineNos: (v: boolean) => void;
  readonly diffsOnly: boolean;
  readonly onToggleDiffsOnly: (v: boolean) => void;
  readonly contextLines: number;
  readonly onContextLines: (n: number) => void;
  /** Wave-89: per-side save rides the toolbar overflow (no separate
   * save row above the list). Same testids/behavior, owned by
   * `TextView`; disabled while saving or with no session. */
  readonly canSave: boolean;
  readonly savingSide: "left" | "right" | null;
  readonly onSaveSide: (side: "left" | "right") => void;
  /** Wave-L3: honest per-side dirty indicator (staged edits or staged copies). */
  readonly leftDirty?: boolean;
  readonly rightDirty?: boolean;
  /** Wave-L3: Copy-Line controls (selection-or-line to other side). */
  readonly canCopyLine?: boolean;
  readonly copyLineDisabledReason?: string | null;
  readonly onCopyLine?: (dir: "left" | "right") => void;
}

export default function TextToolbar(props: TextToolbarProps): React.JSX.Element {
  const {
    findOpen,
    gotoOpen,
    ignoreOpen,
    onToggleFind,
    onToggleGoto,
    onToggleIgnore,
    onPrevDiff,
    onNextDiff,
    onBusCommand,
    hunksLabel,
    diffNote,
    wrap,
    onToggleWrap,
    showLineNos,
    onToggleLineNos,
    diffsOnly,
    onToggleDiffsOnly,
    contextLines,
    onContextLines,
    canSave,
    savingSide,
    onSaveSide,
    leftDirty = false,
    rightDirty = false,
    canCopyLine = false,
    copyLineDisabledReason = null,
    onCopyLine,
  } = props;
  return (
    <div
      role="toolbar"
      aria-label="Text controls"
      data-testid="textview/toolbar"
      className="tv-controls-bar flex-nowrap overflow-x-auto whitespace-nowrap"
    >
      <ToolbarButton
        data-testid="textview/toolbar-find"
        label={findOpen ? "Hide find (Ctrl+F)" : "Show find (Ctrl+F)"}
        title="Find in text (Ctrl+F)"
        expanded={findOpen}
        onClick={onToggleFind}
      >
        <Search size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Find</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="textview/toolbar-goto"
        label={gotoOpen ? "Hide goto (Ctrl+G)" : "Show goto (Ctrl+G)"}
        title="Goto row (Ctrl+G)"
        expanded={gotoOpen}
        onClick={onToggleGoto}
      >
        <ArrowDownToLine size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Goto</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="textview/toolbar-ignore"
        label={ignoreOpen ? "Hide ignore options" : "Show ignore options"}
        title="Ignore whitespace / case (backend may not support them)"
        expanded={ignoreOpen}
        onClick={onToggleIgnore}
      >
        <SlidersHorizontal size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Ignore</span>
      </ToolbarButton>
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="textview/toolbar-prev-diff"
        label="Previous difference"
        title="Previous difference"
        onClick={() => {
          if (onBusCommand) onBusCommand("prev");
          else onPrevDiff();
        }}
      >
        <ChevronUp size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Prev</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="textview/toolbar-next-diff"
        label="Next difference"
        title="Next difference"
        onClick={() => {
          if (onBusCommand) onBusCommand("next");
          else onNextDiff();
        }}
      >
        <ChevronDown size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Next</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="textview/toolbar-copy-line-left"
        label="Copy line left"
        title={copyLineDisabledReason ?? "Copy line to left (take right line)"}
        disabled={!canCopyLine}
        onClick={() => onCopyLine?.("right")}
      >
        <span>Copy Line L</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="textview/toolbar-copy-line-right"
        label="Copy line right"
        title={copyLineDisabledReason ?? "Copy line to right (take left line)"}
        disabled={!canCopyLine}
        onClick={() => onCopyLine?.("left")}
      >
        <span>Copy Line R</span>
      </ToolbarButton>
      <span data-testid="textview/toolbar-hunks" className="ac-meta text-[var(--ac-fg-muted)]">
        {hunksLabel}
      </span>
      {diffNote === null ? null : (
        <span data-testid="textview/diffnav-note" role="note" className="ac-meta text-[var(--ac-fg-muted)]">
          {diffNote}
        </span>
      )}
      <span aria-hidden="true" className="tv-controls-sep" />
      <Checkbox data-testid="textview/wrap-toggle" label="Word wrap" checked={wrap} onChange={(e) => onToggleWrap(e.target.checked)} />
      <Checkbox data-testid="textview/linenos-toggle" label="Line numbers" checked={showLineNos} onChange={(e) => onToggleLineNos(e.target.checked)} />
      <Checkbox data-testid="textview/diffsonly-toggle" label="Differences only" checked={diffsOnly} onChange={(e) => onToggleDiffsOnly(e.target.checked)} />
      <label className="inline-flex cursor-pointer items-center gap-2 text-[length:var(--ac-font-size-ui)] font-normal text-[var(--ac-fg)]">
        Context
        <Select
          data-testid="textview/context-lines"
          aria-label="Context lines around differences"
          title={diffsOnly ? "Unchanged rows shown on each side of every difference" : "Enable Differences only to choose context lines"}
          value={String(contextLines)}
          disabled={!diffsOnly}
          onChange={(e) => {
            const n = Number.parseInt(e.target.value, 10);
            if (n === 0 || n === 1 || n === 3 || n === 5) onContextLines(n);
          }}
        >
          <option value="0">0</option>
          <option value="1">1</option>
          <option value="3">3</option>
          <option value="5">5</option>
        </Select>
      </label>
      {/* Wave-89: save rides the toolbar overflow (right-aligned group,
          always mounted so legacy testids stay visible on load). */}
      <span aria-hidden="true" className="tv-controls-sep" />
      <span data-testid="textview/toolbar-overflow" className="inline-flex items-center gap-[var(--ac-space-2)]">
        <Button
          data-testid="textview/save-left"
          aria-label={leftDirty ? "Save left side to disk (unsaved changes)" : "Save left side to disk"}
          title={leftDirty ? "Save left side to disk (unsaved changes)" : "Save left side to disk"}
          disabled={!canSave || savingSide !== null}
          onClick={() => onSaveSide("left")}
        >
          {leftDirty && (
            <>
              <span
                data-testid="textview/dirty-left"
                className="mr-1 inline-block size-1.5 shrink-0 rounded-full bg-[var(--ac-tab-dirty)]"
                aria-label="unsaved changes"
              />
              <span data-testid="textview/dirty-dot" className="sr-only" aria-hidden="true" />
            </>
          )}
          {savingSide === "left" ? "Saving…" : "Save left"}
        </Button>
        <Button
          data-testid="textview/save-right"
          aria-label={rightDirty ? "Save right side to disk (unsaved changes)" : "Save right side to disk"}
          title={rightDirty ? "Save right side to disk (unsaved changes)" : "Save right side to disk"}
          disabled={!canSave || savingSide !== null}
          onClick={() => onSaveSide("right")}
        >
          {rightDirty && (
            <>
              <span
                data-testid="textview/dirty-right"
                className="mr-1 inline-block size-1.5 shrink-0 rounded-full bg-[var(--ac-tab-dirty)]"
                aria-label="unsaved changes"
              />
              <span data-testid="textview/dirty-dot" className="sr-only" aria-hidden="true" />
            </>
          )}
          {savingSide === "right" ? "Saving…" : "Save right"}
        </Button>
      </span>
    </div>
  );
}

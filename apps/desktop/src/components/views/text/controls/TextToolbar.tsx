import { ArrowDownToLine, ChevronDown, ChevronUp, Search, SlidersHorizontal, Type } from "lucide-react";
import { Button, Checkbox, Select, ToolbarButton } from "../../../ui";

/**
 * Wave-199c: single-row text controls toolbar (Beyond Compare style).
 *
 * One toolbar row: Find / Goto / Ignore / Format toggles (Ctrl+F,
 * Ctrl+G, menus), prev/next difference nav, display toggles (wrap,
 * line numbers, differences-only + context N), save overflow.
 * Find/Goto/Ignore/Format panels float in the `tv-chrome` overlay
 * stack (absolute popovers, never in-flow) so open panels never push
 * `textview/row-0` down. Difference counts live in the bottom status
 * bar (`textview/toolbar-hunks` + `textview/diffnav-note` render
 * there, BC "N difference sections" truth) — same testids/text,
 * new address.
 *
 * Purely presentational: every value and callback is owned by
 * `TextView`. All legacy testids ride the same inputs.
 */
export interface TextToolbarProps {
  readonly findOpen: boolean;
  readonly gotoOpen: boolean;
  readonly ignoreOpen: boolean;
  readonly onToggleFind: () => void;
  readonly onToggleGoto: () => void;
  readonly onToggleIgnore: () => void;
  /** Wave-164: Format (encoding/EOL) popover toggle. Same collapse
      pattern as Ignore; state + re-compare owned by `TextView`. */
  readonly formatOpen: boolean;
  readonly onToggleFormat: () => void;
  readonly onPrevDiff: () => void;
  readonly onNextDiff: () => void;
  /**
   * Wave-161: ViewCommandBus hook (BC unified-toolbar parity). App passes
   * `onBusCommand` through ViewRouter (`onTextBusCommand`), so Prev/Next
   * dispatch through the SAME App `handleViewCommand` bus as the top
   * Toolbar — no duplicate handlers; without the prop the legacy
   * `onPrevDiff`/`onNextDiff` props run. Testids stay byte-identical
   * either way; view-level controls are never removed.
   */
  readonly onBusCommand?: (cmd: "prev" | "next") => void;
  /** Wave-199c: moved to the bottom status bar (BC "N difference
      sections" truth). Kept in the props interface so `TextView`
      call sites compile unchanged; the toolbar no longer renders
      them — `TextView` renders the same testids in `textview/status`.
      @deprecated toolbar no longer renders; read from status bar. */
  readonly hunksLabel?: string;
  readonly diffNote?: string | null;
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
    formatOpen,
    onToggleFormat,
    onPrevDiff,
    onNextDiff,
    onBusCommand,
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
      <ToolbarButton
        data-testid="textview/toolbar-format"
        label={formatOpen ? "Hide format options" : "Show format options"}
        title="Encoding / line endings (backend honors them)"
        expanded={formatOpen}
        onClick={onToggleFormat}
      >
        <Type size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Format</span>
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
        <span>Copy line left</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="textview/toolbar-copy-line-right"
        label="Copy line right"
        title={copyLineDisabledReason ?? "Copy line to right (take left line)"}
        disabled={!canCopyLine}
        onClick={() => onCopyLine?.("left")}
      >
        <span>Copy line right</span>
      </ToolbarButton>
      {/* Wave-199c: counts moved to the bottom status bar (BC "N
          difference sections" truth). `textview/toolbar-hunks` +
          `textview/diffnav-note` render in `textview/status` with the
          same testids/text — the toolbar stays one compact row. */}
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

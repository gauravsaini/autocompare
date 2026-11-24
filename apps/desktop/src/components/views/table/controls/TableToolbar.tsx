/**
 * Wave-68: single-row table controls toolbar (Beyond Compare style).
 *
 * Collapses the ~6 stacked chrome rows above the grid (sort status,
 * counts, cols toggle/show-all/visible, export + receipt, cols menu
 * button, by-key toggle/status, find row) into one toolbar row: the
 * by-key toggle + status, the Columns toggle + Show all + visible
 * count, the Export button (+ inline receipt), and the Find toggle +
 * prev/next difference nav. Purely presentational: every value and
 * callback is owned by `TableView` (existing by-key / visibility /
 * export / find / sort logic untouched). All legacy testids ride the
 * same controls so wave-20/24/32/38/44 specs keep passing unmodified.
 *
 * The cols menu defaults CLOSED (wave-32 contract: the columns spec
 * asserts `aria-expanded="false"` + no menu on load); the find panel
 * defaults OPEN (wave-20/38 specs fill the find box on load). Both
 * panels are in-flow (never absolute) so the overlap gate holds.
 */

import { ChevronDown, ChevronUp, Search } from "lucide-react";
import { Button, Checkbox, ToolbarButton } from "../../../ui";

export interface TableToolbarProps {
  readonly byKey: boolean;
  readonly onToggleByKey: (v: boolean) => void;
  readonly colsOpen: boolean;
  readonly onToggleCols: () => void;
  readonly showAllDisabled: boolean;
  readonly onShowAll: () => void;
  readonly hiddenCount: number;
  readonly onExport: () => void;
  readonly findOpen: boolean;
  readonly onToggleFind: () => void;
  readonly diffCount: number;
  /** 0-based cursor into the loaded diff rows; -1 when there are none. */
  readonly diffPos: number;
  readonly onPrevDiff: () => void;
  readonly onNextDiff: () => void;
}

export default function TableToolbar(props: TableToolbarProps): React.JSX.Element {
  const {
    byKey,
    onToggleByKey,
    colsOpen,
    onToggleCols,
    showAllDisabled,
    onShowAll,
    hiddenCount,
    onExport,
    findOpen,
    onToggleFind,
    diffCount,
    diffPos,
    onPrevDiff,
    onNextDiff,
  } = props;
  return (
    <div
      role="toolbar"
      aria-label="Table controls"
      data-testid="tableview/toolbar"
      className="tv-controls-bar"
    >
      <Checkbox id="table-bykey-toggle" data-testid="table-bykey-toggle" label="Align by key" checked={byKey} onChange={(e) => onToggleByKey(e.target.checked)} />
      <span
        data-testid="table-bykey-status"
        role="status"
        className="ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]"
      >
        {byKey ? "by-key" : "positional"}
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button
        data-testid="table-cols-toggle"
        aria-expanded={colsOpen}
        aria-controls="table-cols-menu"
        onClick={onToggleCols}
      >
        {colsOpen ? "Hide columns panel" : "Columns"}
      </Button>
      <Button
        data-testid="table-cols-show-all"
        onClick={onShowAll}
        disabled={showAllDisabled}
      >
        Show all
      </Button>
      <span
        data-testid="table-cols-visible"
        role="status"
        className="ac-mono text-[length:var(--ac-font-size-code)] font-normal leading-[var(--ac-line-height-code)] tracking-[var(--ac-letter-spacing-code)] text-[var(--ac-fg-muted)]"
      >
        {hiddenCount === 0 ? "all columns shown" : `${hiddenCount} hidden`}
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button data-testid="table-export" onClick={onExport}>
        Export CSV
      </Button>
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="tableview/toolbar-find"
        label={findOpen ? "Hide find (Ctrl+F)" : "Show find (Ctrl+F)"}
        title="Find in table (Ctrl+F)"
        expanded={findOpen}
        onClick={onToggleFind}
      >
        <Search size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Find</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="tableview/toolbar-prev-diff"
        label="Previous difference row"
        title="Previous difference row"
        onClick={onPrevDiff}
        disabled={diffCount === 0}
      >
        <ChevronUp size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Prev</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="tableview/toolbar-next-diff"
        label="Next difference row"
        title="Next difference row"
        onClick={onNextDiff}
        disabled={diffCount === 0}
      >
        <ChevronDown size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Next</span>
      </ToolbarButton>
      <span data-testid="tableview/toolbar-diffs" className="ac-meta text-[var(--ac-fg-muted)]">
        {diffCount === 0
          ? "no differences"
          : `difference ${diffPos + 1} of ${diffCount}`}
      </span>
    </div>
  );
}

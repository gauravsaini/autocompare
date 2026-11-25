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
 *
 * Wave-168b: the two ignore-flag checkboxes grow the row near the
 * 1280px container edge, so the row is `nowrap` + `overflow-x-auto`
 * (single row keeps the chrome budget + `barH < 120`). A
 * scroll-clipped child keeps its full bounding box, which the
 * wave-7 `assertNoOverlap` gate reads as an overlap (e.g.
 * `tableview/toolbar-next-diff`). `[&>*]:min-w-0` lets every item
 * shrink in place instead of scrolling out, so measured boxes always
 * tile with no overlap; the ignore checkboxes ride `shrink-0` so the
 * wave-168 additions never compress away.
 */

import { ChevronDown, ChevronUp, Search } from "lucide-react";
import { Button, Checkbox, StatusText, ToolbarButton } from "../../../ui";

export interface TableToolbarProps {
  readonly byKey: boolean;
  readonly onToggleByKey: (v: boolean) => void;
  /** Wave-168: ignore whitespace / case flags (Beyond Compare parity). */
  readonly ignoreWs: boolean;
  readonly onToggleIgnoreWs: (v: boolean) => void;
  readonly ignoreCase: boolean;
  readonly onToggleIgnoreCase: (v: boolean) => void;
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
  /**
   * Wave-TBL: column-mapping panel state (Beyond Compare style left <->
   * right mapping). Purely presentational — `TableView` owns the map.
   */
  readonly mapOpen: boolean;
  readonly onToggleMap: () => void;
  readonly mapDisabled: boolean;
  readonly mapDisabledReason: string | null;
  readonly mapSummary: string;
  /**
   * Wave-TBL: per-cell/row clipboard copy of the selected cell. Disabled
   * with an honest reason until a cell is selected (no new chrome rows;
   * the receipt rides this toolbar).
   */
  readonly copyCellDisabled: boolean;
  readonly onCopyCell: () => void;
  readonly copyRowDisabled: boolean;
  readonly onCopyRow: () => void;
  readonly copyNote: string | null;
}

export default function TableToolbar(props: TableToolbarProps): React.JSX.Element {
  const {
    byKey,
    onToggleByKey,
    ignoreWs,
    onToggleIgnoreWs,
    ignoreCase,
    onToggleIgnoreCase,
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
    mapOpen,
    onToggleMap,
    mapDisabled,
    mapDisabledReason,
    mapSummary,
    copyCellDisabled,
    onCopyCell,
    copyRowDisabled,
    onCopyRow,
    copyNote,
  } = props;
  return (
    <div
      role="toolbar"
      aria-label="Table controls"
      data-testid="tableview/toolbar"
      className="tv-controls-bar tv-controls-bar--dense"
    >
      <Checkbox id="table-bykey-toggle" data-testid="table-bykey-toggle" label="Align by key" checked={byKey} onChange={(e) => onToggleByKey(e.target.checked)} />
      <StatusText
        data-testid="table-bykey-status"
        role="status"
        variant="code"
      >
        {byKey ? "by-key" : "positional"}
      </StatusText>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Checkbox
        id="table-ignore-ws"
        data-testid="tableview/ignore-ws"
        label="Ignore ws"
        className="shrink-0"
        checked={ignoreWs}
        onChange={(e) => onToggleIgnoreWs(e.target.checked)}
      />
      <Checkbox
        id="table-ignore-case"
        data-testid="tableview/ignore-case"
        label="Ignore case"
        className="shrink-0"
        checked={ignoreCase}
        onChange={(e) => onToggleIgnoreCase(e.target.checked)}
      />
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button
        data-testid="table-cols-toggle"
        size="sm"
        aria-expanded={colsOpen}
        aria-controls="table-cols-menu"
        onClick={onToggleCols}
      >
        {colsOpen ? "Hide columns panel" : "Columns"}
      </Button>
      <Button
        data-testid="table-cols-show-all"
        size="sm"
        onClick={onShowAll}
        disabled={showAllDisabled}
      >
        Show all
      </Button>
      <StatusText
        data-testid="table-cols-visible"
        role="status"
        variant="code"
      >
        {hiddenCount === 0 ? "all columns shown" : `${hiddenCount} hidden`}
      </StatusText>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button data-testid="table-export" size="sm" onClick={onExport}>
        Export CSV
      </Button>
      {/* Wave-TBL: column mapping (Beyond Compare style left <-> right).
          Toggle + summary ride the single toolbar row; the panel itself
          collapses in-flow below (closed by default, like the cols menu)
          so the collapsed-chrome budget holds. */}
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button
        data-testid="table-map-toggle"
        size="sm"
        aria-expanded={mapOpen}
        aria-controls="table-map-panel"
        onClick={onToggleMap}
        disabled={mapDisabled}
        title={mapDisabledReason ?? "Map left columns to right columns"}
      >
        {mapOpen ? "Hide column mapping" : "Map columns"}
      </Button>
      <StatusText
        data-testid="table-map-status"
        role="status"
        variant="code"
      >
        {mapSummary}
      </StatusText>
      {/* Wave-TBL: per-cell/row clipboard copy of the selected grid cell.
          Toolbar-only (no new stacked chrome rows); disabled with an
          honest reason until a cell is selected. */}
      <span aria-hidden="true" className="tv-controls-sep" />
      <Button
        data-testid="table-copy-cell"
        size="sm"
        onClick={onCopyCell}
        disabled={copyCellDisabled}
        title={copyCellDisabled ? "select a cell first" : "Copy selected cell to clipboard"}
      >
        Copy cell
      </Button>
      <Button
        data-testid="table-copy-row"
        size="sm"
        onClick={onCopyRow}
        disabled={copyRowDisabled}
        title={copyRowDisabled ? "select a cell first" : "Copy selected row to clipboard"}
      >
        Copy row
      </Button>
      {copyNote !== null && (
        <StatusText
          data-testid="table-copy-status"
          role="status"
          variant="code"
        >
          {copyNote}
        </StatusText>
      )}
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="tableview/toolbar-find"
        label={findOpen ? "Hide find (Ctrl+F)" : "Show find (Ctrl+F)"}
        title="Find in table (Ctrl+F)"
        expanded={findOpen}
        icon={Search}
        onClick={onToggleFind}
      >
        <span>Find</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="tableview/toolbar-prev-diff"
        label="Previous difference row"
        title="Previous difference row"
        icon={ChevronUp}
        onClick={onPrevDiff}
        disabled={diffCount === 0}
      >
        <span>Prev</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="tableview/toolbar-next-diff"
        label="Next difference row"
        title="Next difference row"
        icon={ChevronDown}
        onClick={onNextDiff}
        disabled={diffCount === 0}
      >
        <span>Next</span>
      </ToolbarButton>
      <StatusText data-testid="tableview/toolbar-diffs" variant="meta">
        {diffCount === 0
          ? "no differences"
          : `difference ${diffPos + 1} of ${diffCount}`}
      </StatusText>
    </div>
  );
}

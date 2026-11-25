/**
 * Wave-71: single-row hex controls toolbar (Beyond Compare style).
 *
 * Collapses the stacked hex chrome (summary line, copy bar, find row,
 * diffs-only row, goto row, pager buttons) into one toolbar row: page
 * prev/next + position, the differences-only toggle, Copy hex / Copy
 * ASCII (inside the legacy `hex-copy-bar` wrapper), and the find +
 * goto toggles. Purely presentational: every value and callback is owned
 * by `HexView` (existing page/find/goto/copy logic untouched). All
 * legacy testids ride the same controls so wave-19/23/36/42/55/60/62/66L
 * specs keep passing unmodified.
 */

import { ArrowDownToLine, ChevronDown, ChevronLeft, ChevronRight, ChevronUp, Search } from "lucide-react";
import { Button, Checkbox, Field, Select, StatusText, ToolbarButton } from "../../../ui";

export interface HexToolbarProps {
  readonly pageLabel: string;
  readonly onPrevPage: () => void;
  readonly onNextPage: () => void;
  readonly prevDisabled: boolean;
  readonly nextDisabled: boolean;
  readonly diffsOnly: boolean;
  readonly onToggleDiffsOnly: (v: boolean) => void;
  readonly onCopyHex: () => void;
  readonly onCopyAscii: () => void;
  readonly findOpen: boolean;
  readonly onToggleFind: () => void;
  readonly gotoOpen: boolean;
  readonly onToggleGoto: () => void;
  // HEX2 (BC parity batch): diff-only navigation (prev/next differing
  // offset via the existing hex page IPC), bytes-per-line layout, and
  // the sync-scroll follow toggle. All live in this ONE toolbar row
  // (chrome budget: no new stacked rows, no popover chrome).
  readonly onPrevDiff: () => void;
  readonly onNextDiff: () => void;
  /**
   * Wave-216 R5: ViewCommandBus hook for diff nav (Text/Merge
   * `onBusCommand` parity). Page Prev/Next stay local; only the
   * Prev/Next-DIFF buttons ride the bus when the prop is provided.
   */
  readonly onBusCommand?: (cmd: "prev" | "next") => void;
  readonly diffNavBusy: boolean;
  readonly diffStatusText: string;
  readonly bytesPerRow: number;
  readonly onBytesPerRow: (v: number) => void;
  readonly syncScroll: boolean;
  readonly onToggleSyncScroll: (v: boolean) => void;
}

// Wave-55 typography contract: the copy buttons keep the ac-chrome-label
// ramp (tight tracking), so `hex-copy-bar`'s first button still measures
// -0.12px letter-spacing.
export default function HexToolbar(props: HexToolbarProps): React.JSX.Element {
  const {
    pageLabel,
    onPrevPage,
    onNextPage,
    prevDisabled,
    nextDisabled,
    diffsOnly,
    onToggleDiffsOnly,
    onCopyHex,
    onCopyAscii,
    findOpen,
    onToggleFind,
    gotoOpen,
    onToggleGoto,
    onPrevDiff,
    onNextDiff,
    onBusCommand,
    diffNavBusy,
    diffStatusText,
    bytesPerRow,
    onBytesPerRow,
    syncScroll,
    onToggleSyncScroll,
  } = props;
  return (
    <div
      role="toolbar"
      aria-label="Hex controls"
      data-testid="hexview/toolbar"
      className="tv-controls-bar"
    >
      <ToolbarButton data-testid="hexview/toolbar-prev-page" onClick={onPrevPage} disabled={prevDisabled} icon={ChevronLeft} label="Prev">
        Prev
      </ToolbarButton>
      <ToolbarButton data-testid="hexview/toolbar-next-page" onClick={onNextPage} disabled={nextDisabled} icon={ChevronRight} label="Next">
        Next
      </ToolbarButton>
      <StatusText data-testid="hexview/page-label">
        {pageLabel}
      </StatusText>
      <span aria-hidden="true" className="tv-controls-sep" />
      {/* HEX2: diff-only navigation (BC next/previous difference).
          Same row, honest busy disable; receipt rides `hex-diff-status`. */}
      <ToolbarButton data-testid="hex-diff-prev" onClick={() => {
        if (onBusCommand) onBusCommand("prev");
        else onPrevDiff();
      }} disabled={diffNavBusy} icon={ChevronUp} label="Prev diff">
        Prev diff
      </ToolbarButton>
      <ToolbarButton data-testid="hex-diff-next" onClick={() => {
        if (onBusCommand) onBusCommand("next");
        else onNextDiff();
      }} disabled={diffNavBusy} icon={ChevronDown} label="Next diff">
        Next diff
      </ToolbarButton>
      <StatusText data-testid="hex-diff-status" role="status">
        {diffStatusText}
      </StatusText>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Checkbox
        id="hexview/diffsonly-toggle"
        data-testid="hexview/diffsonly-toggle"
        label="Differences only"
        checked={diffsOnly}
        onChange={(e) => onToggleDiffsOnly(e.target.checked)}
      />
      {/* HEX2: bytes-per-line layout (BC hex columns parity) + sync-scroll
          follow toggle. Shared Field primitive, same row, no stacked chrome. */}
      <Field label="Bytes/line" htmlFor="hexview/bytes-per-line">
        <Select
          id="hexview/bytes-per-line"
          data-testid="hexview/bytes-per-line"
          aria-label="Bytes per line"
          controlSize="sm"
          value={String(bytesPerRow)}
          onChange={(e) => onBytesPerRow(Number(e.target.value))}
        >
          <option value="8">8</option>
          <option value="16">16</option>
          <option value="32">32</option>
        </Select>
      </Field>
      <Checkbox
        id="hexview/sync-toggle"
        data-testid="hexview/sync-toggle"
        label="Sync scroll"
        title="Follow the selection on both panes together"
        checked={syncScroll}
        onChange={(e) => onToggleSyncScroll(e.target.checked)}
      />
      <span aria-hidden="true" className="tv-controls-sep" />
      <span data-testid="hex-copy-bar" className="ac-control-group">
        <Button data-testid="hex-copy-hex" size="sm" onClick={onCopyHex}>
          Copy hex
        </Button>
        <Button data-testid="hex-copy-ascii" size="sm" onClick={onCopyAscii}>
          Copy ASCII
        </Button>
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="hexview/toolbar-find"
        label={findOpen ? "Hide find (Ctrl+F)" : "Show find (Ctrl+F)"}
        title="Find bytes (Ctrl+F)"
        expanded={findOpen}
        icon={Search}
        onClick={onToggleFind}
      >
        <span>Find</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="hexview/toolbar-goto"
        label={gotoOpen ? "Hide goto (Ctrl+G)" : "Show goto (Ctrl+G)"}
        title="Goto offset (Ctrl+G)"
        expanded={gotoOpen}
        icon={ArrowDownToLine}
        onClick={onToggleGoto}
      >
        <span>Goto</span>
      </ToolbarButton>
    </div>
  );
}

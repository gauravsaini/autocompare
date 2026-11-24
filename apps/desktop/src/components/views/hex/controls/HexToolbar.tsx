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
import { Button, Checkbox, Select, ToolbarButton } from "../../../ui";

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
      <Button onClick={onPrevPage} disabled={prevDisabled}>
        <ChevronLeft size={14} strokeWidth={1.75} aria-hidden="true" /> Prev
      </Button>
      <Button onClick={onNextPage} disabled={nextDisabled}>
        Next <ChevronRight size={14} strokeWidth={1.75} aria-hidden="true" />
      </Button>
      <span
        data-testid="hexview/page-label"
        className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {pageLabel}
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      {/* HEX2: diff-only navigation (BC next/previous difference).
          Same row, honest busy disable; receipt rides `hex-diff-status`. */}
      <Button data-testid="hex-diff-prev" onClick={onPrevDiff} disabled={diffNavBusy}>
        <ChevronUp size={14} strokeWidth={1.75} aria-hidden="true" /> Prev diff
      </Button>
      <Button data-testid="hex-diff-next" onClick={onNextDiff} disabled={diffNavBusy}>
        Next diff <ChevronDown size={14} strokeWidth={1.75} aria-hidden="true" />
      </Button>
      <span
        data-testid="hex-diff-status"
        role="status"
        className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {diffStatusText}
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Checkbox
        id="hexview/diffsonly-toggle"
        data-testid="hexview/diffsonly-toggle"
        label="Differences only"
        checked={diffsOnly}
        onChange={(e) => onToggleDiffsOnly(e.target.checked)}
      />
      {/* HEX2: bytes-per-line layout (BC hex columns parity) + sync-scroll
          follow toggle. Native controls, same row, no stacked chrome. */}
      <label
        htmlFor="hexview/bytes-per-line"
        className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        Bytes/line
      </label>
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
      <Checkbox
        id="hexview/sync-toggle"
        data-testid="hexview/sync-toggle"
        label="Sync scroll"
        title="Follow the selection on both panes together"
        checked={syncScroll}
        onChange={(e) => onToggleSyncScroll(e.target.checked)}
      />
      <span aria-hidden="true" className="tv-controls-sep" />
      <span data-testid="hex-copy-bar" className="inline-flex flex-wrap items-center gap-[var(--ac-space-2)]">
        <Button data-testid="hex-copy-hex" onClick={onCopyHex}>
          Copy hex
        </Button>
        <Button data-testid="hex-copy-ascii" onClick={onCopyAscii}>
          Copy ASCII
        </Button>
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      <ToolbarButton
        data-testid="hexview/toolbar-find"
        label={findOpen ? "Hide find (Ctrl+F)" : "Show find (Ctrl+F)"}
        title="Find bytes (Ctrl+F)"
        expanded={findOpen}
        onClick={onToggleFind}
      >
        <Search size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Find</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="hexview/toolbar-goto"
        label={gotoOpen ? "Hide goto (Ctrl+G)" : "Show goto (Ctrl+G)"}
        title="Goto offset (Ctrl+G)"
        expanded={gotoOpen}
        onClick={onToggleGoto}
      >
        <ArrowDownToLine size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Goto</span>
      </ToolbarButton>
    </div>
  );
}

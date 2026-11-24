import { ArrowDownToLine, Search } from "lucide-react";
import { Button, Checkbox, ToolbarButton } from "../../../ui";
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
}// Wave-55 typography contract: the copy buttons keep the ac-chrome-label
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
  } = props;
  return (
    <div
      role="toolbar"
      aria-label="Hex controls"
      data-testid="hexview/toolbar"
      className="tv-controls-bar"
    >
      <Button onClick={onPrevPage} disabled={prevDisabled}>
        Prev
      </Button>
      <Button onClick={onNextPage} disabled={nextDisabled}>
        Next
      </Button>
      <span
        data-testid="hexview/page-label"
        className="text-[length:var(--ac-font-size-ui)] font-normal leading-[var(--ac-line-height-ui)] tracking-[var(--ac-letter-spacing-ui-tight)] text-[var(--ac-fg-muted)]"
      >
        {pageLabel}
      </span>
      <span aria-hidden="true" className="tv-controls-sep" />
      <Checkbox data-testid="hexview/diffsonly-toggle" label="onToggleDiffsOnly(e.target.checked)}" checked={diffsOnly} onChange={(e) => onToggleDiffsOnly(e.target.checked)} />
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
        aria-label={findOpen ? "Hide find (Ctrl+F)" : "Show find (Ctrl+F)"}
        title="Find bytes (Ctrl+F)"
        aria-expanded={findOpen}
        onClick={onToggleFind}
        
      >
        <Search size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Find</span>
      </ToolbarButton>
      <ToolbarButton
        data-testid="hexview/toolbar-goto"
        label={gotoOpen ? "Hide goto (Ctrl+G)" : "Show goto (Ctrl+G)"}
        aria-label={gotoOpen ? "Hide goto (Ctrl+G)" : "Show goto (Ctrl+G)"}
        title="Goto offset (Ctrl+G)"
        aria-expanded={gotoOpen}
        onClick={onToggleGoto}
        
      >
        <ArrowDownToLine size={14} strokeWidth={1.75} aria-hidden="true" />
        <span>Goto</span>
      </ToolbarButton>
    </div>
  );
}

/**
 * Wave-234: find migrated to ui/DockedBar.
 * Wave-WC: styling-only density pin (DockedBar contract kept byte-identical:
 * in-flow section, Esc + close button ride the single onClose; Prev/Next
 * explicit sm=22, input default 24). No testid / Esc-contract change.
 * In-flow by construction (never absolute/fixed on the bar itself).
 */
import { Button, DockedBar, Input, StatusText } from "../../../ui";

export interface FindOverlayProps {
  readonly open?: boolean;
  readonly query: string;
  readonly onQueryChange: (q: string) => void;
  readonly prevDisabled: boolean;
  readonly nextDisabled: boolean;
  readonly onPrev: () => void;
  readonly onNext: () => void;
  readonly onRun: () => void;
  readonly onClose: () => void;
  readonly statusText: string;
}

export default function FindOverlay(props: FindOverlayProps): React.JSX.Element | null {
  const { open = true, query, onQueryChange, prevDisabled, nextDisabled, onPrev, onNext, onRun, onClose, statusText } = props;
  return (
    <DockedBar
      open={open}
      onClose={onClose}
      label="Find"
      testid="textview/find-panel"
      closeTestId="textview/find-close"
    >
      <Input
        id="textview-find-box"
        data-testid="textview/find-input"
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") onRun();
          else if (event.key === "Escape") onClose();
        }}
        placeholder="text to find"
        autoComplete="off"
        spellCheck={false}
        className="ac-mono w-40 text-[length:var(--ac-font-size-code)]"
      />
      <Button size="sm" data-testid="textview/find-prev" aria-label="Previous find hit" onClick={onPrev} disabled={prevDisabled}>
        Prev
      </Button>
      <Button size="sm" data-testid="textview/find-next" aria-label="Next find hit" onClick={onNext} disabled={nextDisabled}>
        Next
      </Button>
      <StatusText data-testid="textview/find-status">
        {statusText}
      </StatusText>
    </DockedBar>
  );
}

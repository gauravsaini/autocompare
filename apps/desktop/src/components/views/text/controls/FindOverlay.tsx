/**
 * Wave-234: find migrated to ui/DockedBar.
 * In-flow collapsible find bar (never absolute). Purely presentational:
 * query state, hit math and activation live in TextView.
 * Testids byte-identical.
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
      <Button data-testid="textview/find-prev" aria-label="Previous find hit" onClick={onPrev} disabled={prevDisabled}>
        Prev
      </Button>
      <Button data-testid="textview/find-next" aria-label="Next find hit" onClick={onNext} disabled={nextDisabled}>
        Next
      </Button>
      <StatusText data-testid="textview/find-status">
        {statusText}
      </StatusText>
    </DockedBar>
  );
}

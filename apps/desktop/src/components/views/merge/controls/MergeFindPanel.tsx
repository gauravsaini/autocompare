/**
 * Wave-234: merge find migrated to ui/DockedBar.
 * In-flow collapsible find bar (never absolute). Purely presentational:
 * query state, hit math and activation live in MergeView.
 * Testids byte-identical.
 */
import { Button, DockedBar, Input, StatusText } from "../../../ui";

export interface MergeFindPanelProps {
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

export default function MergeFindPanel(props: MergeFindPanelProps): React.JSX.Element | null {
  const {
    open = true,
    query,
    onQueryChange,
    prevDisabled,
    nextDisabled,
    onPrev,
    onNext,
    onRun,
    onClose,
    statusText,
  } = props;
  return (
    <DockedBar
      open={open}
      onClose={onClose}
      label="Find"
      testid="merge/find-panel"
      closeTestId="merge/find-close"
    >
      <Input
        id="merge-find-box"
        data-testid="merge/find-input"
        controlSize="sm"
        value={query}
        onChange={(event) => onQueryChange(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") onRun();
          else if (event.key === "Escape") onClose();
        }}
        placeholder="text to find"
        autoComplete="off"
        spellCheck={false}
        className="ac-code w-40"
      />
      <Button
        size="sm"
        data-testid="merge/find-prev"
        aria-label="Previous find hit"
        onClick={onPrev}
        disabled={prevDisabled}
      >
        Prev
      </Button>
      <Button
        size="sm"
        data-testid="merge/find-next"
        aria-label="Next find hit"
        onClick={onNext}
        disabled={nextDisabled}
      >
        Next
      </Button>
      <StatusText data-testid="merge/find-status">
        {statusText}
      </StatusText>
    </DockedBar>
  );
}

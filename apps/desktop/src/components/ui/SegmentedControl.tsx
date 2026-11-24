export interface SegmentOption<T extends string> {
  readonly value: T;
  readonly label: string;
  readonly testid: string;
  readonly pressed: boolean;
  readonly onSelect: () => void;
}

export interface SegmentedControlProps<T extends string> {
  readonly label: string;
  readonly testid?: string;
  readonly options: readonly SegmentOption<T>[];
}

/** macOS recessed pill track with one elevated active segment. Testids pass through. */
export default function SegmentedControl<T extends string>({ label, testid, options }: SegmentedControlProps<T>): React.JSX.Element {
  return (
    <div role="group" aria-label={label} data-testid={testid} className="inline-flex items-center gap-0 rounded-[6px] bg-[var(--ac-segment-track)] p-0.5">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          data-testid={o.testid}
          aria-pressed={o.pressed}
          onClick={o.onSelect}
          className={
            o.pressed
              ? "inline-flex h-[var(--ac-control-h-sm)] items-center rounded-[5px] bg-[var(--ac-segment-active-bg)] px-2 ac-chrome-label font-medium text-[var(--ac-fg)] shadow-[var(--ac-segment-active-shadow)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
              : "inline-flex h-[var(--ac-control-h-sm)] items-center rounded-[5px] px-2 ac-chrome-label font-medium text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-[var(--ac-focus-ring)]"
          }
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

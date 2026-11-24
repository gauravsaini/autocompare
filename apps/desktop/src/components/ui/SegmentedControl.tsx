export interface SegmentOption<T extends string> {
  readonly value: T;
  readonly label: string;
  readonly testid: string;
  readonly pressed: boolean;
  readonly onSelect: () => void;
}

export type SegmentedControlSize = "sm" | "md";

export interface SegmentedControlProps<T extends string> {
  readonly label: string;
  readonly testid?: string;
  readonly size?: SegmentedControlSize;
  readonly options: readonly SegmentOption<T>[];
}

/** macOS recessed pill track with one elevated active segment, hairline border + bevel. Testids pass through. */
export default function SegmentedControl<T extends string>({
  label,
  testid,
  size = "sm",
  options,
}: SegmentedControlProps<T>): React.JSX.Element {
  const h = size === "md" ? "h-[var(--ac-control-h-md)]" : "h-[var(--ac-control-h-sm)]";
  return (
    <div
      role="group"
      aria-label={label}
      data-testid={testid}
      className="inline-flex items-center gap-0 rounded-[5px] border border-[var(--ac-segment-track-border,transparent)] bg-[var(--ac-segment-track)] p-0.5 shadow-[var(--ac-segment-track-shadow,inset_0_0.5px_1px_rgba(0,0,0,0.08))]"
    >
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          data-testid={o.testid}
          aria-pressed={o.pressed}
          onClick={o.onSelect}
          className={
            o.pressed
              ? `inline-flex ${h} items-center rounded-[5px] border border-[var(--ac-segment-active-border,transparent)] bg-[var(--ac-segment-active-bg)] px-2 ac-chrome-label font-medium text-[var(--ac-fg)] shadow-[var(--ac-segment-active-shadow)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)] select-none`
              : `inline-flex ${h} items-center rounded-[5px] border border-transparent px-2 ac-chrome-label font-medium text-[var(--ac-fg-muted)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)] select-none`
          }
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

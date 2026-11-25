import { useEffect, type ComponentType, type ReactNode } from "react";
import { ChevronDown, ChevronUp } from "lucide-react";
import ToolbarButton from "./ToolbarButton";
import Button from "./Button";
import Checkbox from "./Checkbox";
import Select from "./Select";
import SegmentedControl, { type SegmentOption } from "./SegmentedControl";
// Wave-239: leaf import only (no chrome barrel) so the ui layer stays
// cycle-free — same pattern TextView uses for VIEW_COMMAND_EVENT.
import { dispatchViewCommand, dispatchViewFilter, type ViewFilterKind } from "../chrome/ViewCommandBus";

/**
 * Wave-239: single shared nav/filter/copy cluster primitive.
 *
 * One implementation for the Prev/Next + All/Diffs/Same/Context + Copy
 * addressing sets the unified top Toolbar already owns via
 * `ViewCommandBus`. View-level toolbars render these clusters instead of
 * hand-rolling duplicate buttons — same testids/labels/titles/icons, so
 * every existing spec keeps passing unmodified.
 *
 * Bus contract: nav buttons prefer `onBusCommand` (the App
 * `handleViewCommand` path, same handler as the top Toolbar) when
 * provided, else dispatch `autocompare:view-command` scoped to
 * `viewKind` when given, else fall back to the direct `onPrev`/`onNext`
 * props. Filter/copy clusters are presentational: callers pass the same
 * handlers they use today (local state setters or bus dispatchers) —
 * no new handlers, no duplicate dispatches.
 *
 * Esc contract (mirrors `ui/DockedBar`, never competes with it):
 * `useUnifiedEsc` observes Esc on window capture and calls `onClose`
 * without stopPropagation, so every open overlay collapses per the
 * app-wide Esc contract while DockedBar/Modal (the consuming owners)
 * still run. Inside a `DockedBar` omit `onClose` (the bar already owns
 * Esc) to avoid double handling.
 */

export function useUnifiedEsc(onClose: (() => void) | undefined, open = true): void {
  useEffect(() => {
    if (!onClose || !open) return;
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") onClose();
    };
    // Window capture (not document): capture descends window -> document,
    // so this observes Esc BEFORE any document-capture owner (DockedBar,
    // Modal) consumes it via stopPropagation. Deliberately never calls
    // stopPropagation itself: Esc collapses every open overlay per the
    // app-wide contract (see chrome/shortcuts.ts), and a second
    // document-capture listener provably starves behind the first in
    // this WebKit build (verified empirically).
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [onClose, open]);
}

export type NavIcon = ComponentType<{
  size?: number;
  strokeWidth?: number;
  className?: string;
  "aria-hidden"?: boolean | "true" | "false";
}>;

export interface UnifiedNavClusterProps {
  readonly viewKind?: string;
  readonly prevTestid: string;
  readonly nextTestid: string;
  readonly prevLabel?: string;
  readonly nextLabel?: string;
  readonly prevTitle?: string;
  readonly nextTitle?: string;
  readonly onPrev: () => void;
  readonly onNext: () => void;
  readonly prevDisabled?: boolean;
  readonly nextDisabled?: boolean;
  readonly onBusCommand?: (cmd: "prev" | "next") => void;
  readonly prevIcon?: NavIcon;
  readonly nextIcon?: NavIcon;
  readonly size?: "sm" | "md";
  /**
   * `"toolbar"` renders `ToolbarButton` (text/hex/table chrome);
   * `"button"` renders `ui/Button` with the icon as a child
   * (merge chrome: `Button` + leading/trailing chevron, byte-same look).
   */
  readonly variant?: "toolbar" | "button";
}

export function UnifiedNavCluster(props: UnifiedNavClusterProps): React.JSX.Element {
  const {
    viewKind,
    prevTestid,
    nextTestid,
    prevLabel = "Previous difference",
    nextLabel = "Next difference",
    prevTitle,
    nextTitle,
    onPrev,
    onNext,
    prevDisabled = false,
    nextDisabled = false,
    onBusCommand,
    prevIcon = ChevronUp,
    nextIcon = ChevronDown,
    size = "sm",
    variant = "toolbar",
  } = props;
  const firePrev = (): void => {
    if (onBusCommand) onBusCommand("prev");
    else if (viewKind) dispatchViewCommand("prev", viewKind);
    else onPrev();
  };
  const fireNext = (): void => {
    if (onBusCommand) onBusCommand("next");
    else if (viewKind) dispatchViewCommand("next", viewKind);
    else onNext();
  };
  const PrevIcon = prevIcon;
  const NextIcon = nextIcon;
  const prevShort = prevLabel === "Previous difference" || prevLabel === "Previous difference row" ? "Prev" : prevLabel;
  const nextShort = nextLabel === "Next difference" || nextLabel === "Next difference row" ? "Next" : nextLabel;
  if (variant === "button") {
    return (
      <span data-unified-cluster="nav" className="contents">
        <Button size={size} data-testid={prevTestid} disabled={prevDisabled} onClick={firePrev} {...(prevTitle ? { title: prevTitle } : {})}>
          <PrevIcon size={14} strokeWidth={1.75} aria-hidden="true" /> {prevShort}
        </Button>
        <Button size={size} data-testid={nextTestid} disabled={nextDisabled} onClick={fireNext} {...(nextTitle ? { title: nextTitle } : {})}>
          {nextShort} <NextIcon size={14} strokeWidth={1.75} aria-hidden="true" />
        </Button>
      </span>
    );
  }
  return (
    <span data-unified-cluster="nav" className="contents">
      <ToolbarButton
        size={size}
        data-testid={prevTestid}
        label={prevLabel}
        title={prevTitle ?? prevLabel}
        icon={prevIcon}
        disabled={prevDisabled}
        onClick={firePrev}
      >
        <span>{prevShort}</span>
      </ToolbarButton>
      <ToolbarButton
        size={size}
        data-testid={nextTestid}
        label={nextLabel}
        title={nextTitle ?? nextLabel}
        icon={nextIcon}
        disabled={nextDisabled}
        onClick={fireNext}
      >
        <span>{nextShort}</span>
      </ToolbarButton>
    </span>
  );
}

export interface UnifiedFilterSegmentProps<T extends string> {
  readonly label: string;
  readonly testid?: string;
  readonly size?: "sm" | "md";
  readonly options: readonly SegmentOption<T>[];
  /** Optional bus mirror: when the selected value is a view filter, dispatch it too. */
  readonly busKind?: string;
}

export function UnifiedFilterSegment<T extends string>(props: UnifiedFilterSegmentProps<T>): React.JSX.Element {
  const { label, testid, size = "sm", options, busKind } = props;
  const wrapped = busKind
    ? options.map((o) => ({
        ...o,
        onSelect: (): void => {
          o.onSelect();
          const v = o.value as string;
          if (v === "all" || v === "diffs" || v === "same" || v === "context") {
            dispatchViewFilter(v as ViewFilterKind, busKind);
          }
        },
      }))
    : options;
  return (
    <span data-unified-cluster="filter" className="contents">
      <SegmentedControl<T> size={size} label={label} testid={testid} options={wrapped} />
    </span>
  );
}

export interface UnifiedDiffOnlyClusterProps {
  readonly toggleTestid: string;
  readonly toggleLabel?: string;
  readonly checked: boolean;
  readonly onToggle: (v: boolean) => void;
  readonly toggleId?: string;
  readonly contextTestid: string;
  readonly contextLabel?: string;
  readonly contextValue: number;
  readonly onContext: (n: number) => void;
  readonly contextDisabled?: boolean;
  readonly contextTitle?: string;
}

export function UnifiedDiffOnlyCluster(props: UnifiedDiffOnlyClusterProps): React.JSX.Element {
  const {
    toggleTestid,
    toggleLabel = "Differences only",
    checked,
    onToggle,
    toggleId,
    contextTestid,
    contextLabel = "Context",
    contextValue,
    onContext,
    contextDisabled = false,
    contextTitle,
  } = props;
  return (
    <span data-unified-cluster="filter" className="contents">
      <Checkbox
        id={toggleId}
        data-testid={toggleTestid}
        label={toggleLabel}
        checked={checked}
        onChange={(e) => onToggle(e.target.checked)}
      />
      <label className="inline-flex cursor-pointer items-center gap-2 text-[length:var(--ac-font-size-ui)] font-normal text-[var(--ac-fg)]">
        {contextLabel}
        <Select
          data-testid={contextTestid}
          aria-label="Context lines around differences"
          title={contextTitle ?? (checked ? "Unchanged rows shown on each side of every difference" : "Enable Differences only to choose context lines")}
          value={String(contextValue)}
          disabled={contextDisabled}
          onChange={(e) => {
            const n = Number.parseInt(e.target.value, 10);
            if (n === 0 || n === 1 || n === 3 || n === 5) onContext(n);
          }}
        >
          <option value="0">0</option>
          <option value="1">1</option>
          <option value="3">3</option>
          <option value="5">5</option>
        </Select>
      </label>
    </span>
  );
}

export interface UnifiedCopyClusterProps {
  readonly leftTestid: string;
  readonly rightTestid: string;
  readonly leftLabel?: string;
  readonly rightLabel?: string;
  readonly leftTitle?: string | null;
  readonly rightTitle?: string | null;
  readonly disabled?: boolean;
  readonly disabledReason?: string | null;
  readonly onCopyLeft: () => void;
  readonly onCopyRight: () => void;
  readonly size?: "sm" | "md";
}

export function UnifiedCopyCluster(props: UnifiedCopyClusterProps): React.JSX.Element {
  const {
    leftTestid,
    rightTestid,
    leftLabel = "Copy line left",
    rightLabel = "Copy line right",
    leftTitle,
    rightTitle,
    disabled = false,
    disabledReason = null,
    onCopyLeft,
    onCopyRight,
    size = "sm",
  } = props;
  return (
    <span data-unified-cluster="copy" className="contents">
      <ToolbarButton
        size={size}
        data-testid={leftTestid}
        label={leftLabel}
        title={disabledReason ?? leftTitle ?? leftLabel}
        disabled={disabled}
        onClick={onCopyLeft}
      >
        <span>{leftLabel}</span>
      </ToolbarButton>
      <ToolbarButton
        size={size}
        data-testid={rightTestid}
        label={rightLabel}
        title={disabledReason ?? rightTitle ?? rightLabel}
        disabled={disabled}
        onClick={onCopyRight}
      >
        <span>{rightLabel}</span>
      </ToolbarButton>
    </span>
  );
}

export interface UnifiedViewControlsProps {
  readonly nav?: UnifiedNavClusterProps;
  readonly children?: ReactNode;
  /** Mirrors DockedBar Esc: omitted inside a DockedBar (it already owns Esc). */
  readonly onClose?: () => void;
  readonly open?: boolean;
}

export default function UnifiedViewControls(props: UnifiedViewControlsProps): React.JSX.Element {
  const { nav, children, onClose, open = true } = props;
  useUnifiedEsc(onClose, open);
  return (
    <span data-unified-cluster="controls" className="contents">
      {nav ? <UnifiedNavCluster {...nav} /> : null}
      {children}
    </span>
  );
}

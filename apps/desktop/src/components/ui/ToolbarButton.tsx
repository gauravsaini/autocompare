import type { ButtonHTMLAttributes, ComponentType, ReactNode } from "react";

export interface ToolbarButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  readonly label: string;
  readonly expanded?: boolean;
  readonly orientation?: "horizontal" | "vertical";
  readonly size?: "sm" | "md";
  readonly icon?: ComponentType<{ size?: number; strokeWidth?: number; className?: string; "aria-hidden"?: boolean | "true" | "false" }>;
  readonly children?: ReactNode;
}

/** Wave-189 icon unify: Lucide-only, 14px / strokeWidth 1.75, round caps/joins (Lucide default). Single renderer so Views never drift. */
export const AC_ICON_SIZE = 14;
export const AC_ICON_STROKE_WIDTH = 1.75;
export default function ToolbarButton({
  label,
  expanded,
  orientation = "horizontal",
  size = "sm",
  icon: Icon,
  className = "",
  children,
  ...rest
}: ToolbarButtonProps): React.JSX.Element {
  const vertical = orientation === "vertical";
  // DEBTA-geom: vertical chrome buttons ride the 4px ramp
  // (`px-[var(--ac-space-2)]` = 8px). The `px-[3px]` squeeze (0cb5ba1,
  // pre-Wave-223 shared-band density) broke the geopolish 4px-ramp pins
  // (geopolish-122: "toolbar button px", geopolish-66c: "px = space-2");
  // Wave-223 gives the toolbar its own full-width row with internal
  // scroll, so the 8px contract fits 1280 without clipping.
  const h = size === "md" ? "h-[var(--ac-control-h-md)] px-2" : "h-[var(--ac-control-h-sm)] px-1.5";
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      aria-expanded={expanded}
      data-pressed={expanded === true ? "true" : undefined}
      className={
        (vertical
          ? "inline-flex min-h-[38px] min-w-[40px] shrink-0 flex-col items-center justify-center gap-[var(--ac-space-1)] rounded-[5px] px-[var(--ac-space-2)] py-1 "
          : `inline-flex ${h} items-center gap-1 rounded-[5px] `) +
        "ac-chrome-label font-medium text-[var(--ac-fg)] border border-transparent hover:border-[var(--ac-border-subtle)] hover:bg-[var(--ac-btn-hover-bg)] " +
        "active:bg-[var(--ac-btn-active-bg)] focus-visible:outline-hidden focus-visible:ring-1 " +
        "focus-visible:ring-[var(--ac-focus)] disabled:cursor-not-allowed disabled:opacity-45 select-none " +
        (expanded === true ? "bg-[var(--ac-btn-hover-bg)] border-[var(--ac-border-subtle)] " : "") +
        className
      }
      {...rest}
    >
      {Icon ? <Icon size={AC_ICON_SIZE} strokeWidth={AC_ICON_STROKE_WIDTH} aria-hidden="true" /> : null}
      {children}
    </button>
  );
}

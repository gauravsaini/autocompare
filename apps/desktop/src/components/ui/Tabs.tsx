import type { HTMLAttributes, ReactNode } from "react";

export type TabSize = "sm" | "md";

export interface TabListProps extends HTMLAttributes<HTMLDivElement> {
  readonly label?: string;
  readonly "aria-label"?: string;
  readonly testid?: string;
  readonly newTab?: ReactNode;
  readonly extra?: ReactNode;
  readonly scrollClassName?: string;
  readonly children?: ReactNode;
}

const baseTabClasses =
  "ac-chrome-label flex min-w-[var(--ac-tab-min-w)] max-w-[var(--ac-tab-max-w)] flex-1 basis-0 cursor-pointer select-none items-center gap-[var(--ac-space-1)] rounded-[5px]";

const tabSizes: Record<TabSize, string> = {
  sm: "h-[var(--ac-tab-chip-h-sm,22px)]",
  md: "h-[var(--ac-tab-chip-h)]",
};

const activeTabClasses =
  "border border-b-0 border-[var(--ac-border)] bg-[var(--ac-tab-active-bg)] pl-[var(--ac-space-2)] pr-[var(--ac-space-1)] font-medium text-[var(--ac-tab-active-fg)] shadow-[inset_0_1px_0_rgba(255,255,255,0.35),0_1px_2px_rgb(0_0_0/0.05)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]";

const inactiveTabClasses =
  "pl-[var(--ac-space-2)] pr-[var(--ac-space-1)] font-normal text-[var(--ac-tab-fg)] hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]";

export interface TabProps extends Omit<HTMLAttributes<HTMLDivElement>, "title"> {
  readonly active?: boolean;
  readonly size?: TabSize;
  readonly title?: ReactNode;
  readonly tooltip?: string;
  readonly dirty?: boolean | ReactNode;
  readonly dirtySlot?: ReactNode;
  readonly dirtyTestId?: string;
  readonly dirtyLabel?: string;
  readonly close?: ReactNode;
  readonly closeSlot?: ReactNode;
  readonly testid?: string;
  readonly children?: ReactNode;
}

/**
 * Tab: Desktop chrome tab chip riding design tokens.
 * Renders title span with min-w-0 truncate, dirty dot slot, and close slot.
 */
export function Tab({
  active = false,
  size = "md",
  title,
  dirty = false,
  dirtySlot,
  dirtyTestId,
  dirtyLabel,
  close,
  closeSlot,
  testid,
  className = "",
  children,
  role = "tab",
  tabIndex = 0,
  ...rest
}: TabProps): React.JSX.Element {
  const dataTestId = testid ?? ((rest as Record<string, unknown>)["data-testid"] as string | undefined);

  const hasExplicitTitle = title !== undefined;
  const titleContent = hasExplicitTitle ? title : children;
  const extraChildren = hasExplicitTitle ? children : null;

  const renderDirty = (): ReactNode => {
    if (dirtySlot !== undefined) return dirtySlot;
    if (typeof dirty === "boolean") {
      if (!dirty) return null;
      return (
        <span
          data-testid={dirtyTestId}
          aria-label={dirtyLabel ?? "unsaved changes"}
          title={dirtyLabel ?? "Unsaved changes"}
          className="inline-block size-1.5 shrink-0 rounded-full bg-[var(--ac-tab-dirty)]"
        />
      );
    }
    return dirty ?? null;
  };

  const closeContent = close ?? closeSlot ?? null;
  const variantClass = active ? activeTabClasses : inactiveTabClasses;
  const sizeClass = tabSizes[size];

  return (
    <div
      role={role}
      aria-selected={rest["aria-selected"] ?? active}
      data-testid={dataTestId}
      tabIndex={tabIndex}
      className={`${baseTabClasses} ${sizeClass} ${variantClass}${className ? ` ${className}` : ""}`}
      {...rest}
    >
      <span className="min-w-0 truncate">{titleContent}</span>
      {extraChildren}
      {renderDirty()}
      {closeContent}
    </div>
  );
}

/**
 * TabList: Outer bar container with role="tablist", aria-label, height var(--ac-tabbar-h),
 * padding px-[var(--ac-space-3)], inner scroll container with overflow-x-auto,
 * and slot for newTab (outside scroll container, right-adjacent).
 */
export function TabList({
  label,
  "aria-label": ariaLabelProp,
  testid,
  newTab,
  extra,
  scrollClassName = "",
  className = "",
  children,
  ...rest
}: TabListProps): React.JSX.Element {
  const ariaLabel = ariaLabelProp ?? label ?? "Open tabs";
  const dataTestId = testid ?? ((rest as Record<string, unknown>)["data-testid"] as string | undefined);

  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      data-testid={dataTestId}
      className={`flex h-[var(--ac-tabbar-h)] items-center gap-[var(--ac-space-1)] overflow-hidden bg-[var(--ac-tab-bg)] px-[var(--ac-space-3)]${className ? ` ${className}` : ""}`}
      {...rest}
    >
      <div
        className={`flex min-w-0 flex-initial items-center gap-[var(--ac-space-1)] overflow-x-auto${scrollClassName ? ` ${scrollClassName}` : ""}`}
      >
        {children}
      </div>
      {newTab}
      {extra}
    </div>
  );
}

export default Tab;

import React, { useRef, useState } from "react";
import { Check, Plus, X } from "lucide-react";
import { Button, ToolbarButton } from "../ui";

export interface ChromeTab {
  id: string;
  title: string;
  kind: string;
  dirty: boolean;
}

export interface TabBarProps {
  tabs: readonly ChromeTab[];
  activeTabId: string;
  onSelectTab: (id: string) => void;
  onCloseTab: (id: string) => void;
  onNewTab: (kind: string) => void;
  /**
   * Wave-174: optional band-siting hook. `ChromeShell` renders TabBar +
   * Toolbar side-by-side in ONE 44px chrome band (BC parity: tabs share
   * the toolbar band instead of stacking a third shell row); the band
   * owns the bottom divider, so the caller passes sizing + `border-b-0`.
   * Absent = legacy standalone look. Heights/testids/wiring untouched.
   */
  className?: string;
}

/**
 * Wave-31: honest dirty derivation from existing tab state only (no new
 * IPC). The welcome `home` tab is never dirty, even if a parent passes
 * `dirty: true` by mistake.
 */
export function isTabDirty(tab: ChromeTab): boolean {
  try {
    if (!tab || tab.dirty !== true) return false;
    if (tab.kind === "home" || tab.id === "home") return false;
    return true;
  } catch {
    return false;
  }
}

/**
 * Wave-31: the welcome `home` tab has no close affordance (the workspace
 * `handleCloseTab` ignores it, so rendering an `X` would be dishonest).
 */
export function isTabClosable(tab: ChromeTab): boolean {
  try {
    if (!tab) return false;
    if (tab.kind === "home" || tab.id === "home") return false;
    return true;
  } catch {
    return false;
  }
}

export function TabBar({ tabs, activeTabId, onSelectTab, onCloseTab, onNewTab, className }: TabBarProps): React.JSX.Element {
  // Wave-5A: defensive first-wins dedup by id so a duplicated store row can
  // never multiply the tab row, even if a parent passes dupes.
  const seen = new Set<string>();
  const visibleTabs = tabs.filter((t) => {
    if (seen.has(t.id)) return false;
    seen.add(t.id);
    return true;
  });
  // Wave-31: pending inline close-confirm for one dirty tab (Beyond
  // Compare style: confirm discards, cancel keeps). Derived against the
  // live list so a removed tab never leaves a stale prompt behind.
  const [confirmId, setConfirmId] = useState<string | null>(null);
  const [closeError, setCloseError] = useState<string | null>(null);
  // Wave-31: middle-click arrives as mousedown(1) + auxclick(1) for one
  // gesture; the stamp dedupes so a single press closes/arms exactly once.
  const lastMiddleAtRef = useRef<Record<string, number>>({});
  const armedConfirmId =
    confirmId !== null && visibleTabs.some((t) => t.id === confirmId) ? confirmId : null;

  const safeSelect = (id: string): void => {
    try {
      onSelectTab(id);
    } catch (err: unknown) {
      setCloseError(err instanceof Error ? err.message : String(err));
    }
  };

  const safeClose = (id: string): void => {
    try {
      onCloseTab(id);
    } catch (err: unknown) {
      setCloseError(err instanceof Error ? err.message : String(err));
    }
  };

  // Wave-31: dirty tabs arm an inline confirm instead of closing; clean
  // tabs close directly. Failures render inline, never throw.
  const requestClose = (tab: ChromeTab): void => {
    setCloseError(null);
    try {
      if (!isTabClosable(tab)) return;
      if (isTabDirty(tab)) {
        setConfirmId(tab.id);
        return;
      }
      setConfirmId(null);
      safeClose(tab.id);
    } catch (err: unknown) {
      setCloseError(err instanceof Error ? err.message : String(err));
    }
  };

  const confirmDiscard = (tab: ChromeTab): void => {
    setConfirmId(null);
    safeClose(tab.id);
  };

  const cancelClose = (): void => {
    setConfirmId(null);
  };

  // Wave-31: middle-click closes clean tabs directly; on a dirty tab it
  // arms the same inline confirm as the close button (never discards).
  const handleMiddleClick = (tab: ChromeTab): void => {
    try {
      const now = Date.now();
      const last = lastMiddleAtRef.current[tab.id] ?? 0;
      if (now - last < 600) return;
      lastMiddleAtRef.current[tab.id] = now;
      requestClose(tab);
    } catch (err: unknown) {
      setCloseError(err instanceof Error ? err.message : String(err));
    }
  };

  return (
    // Wave-186: overflow-proof strip (BC parity, chrome-only). The tab
    // chips used to keep intrinsic width (flex min-width:auto), so 3+
    // long mock titles overflowed the 42% strip and their rects landed
    // on the toolbar (tabletree-54 / folderfilter-113b / foldernamefilter-63
    // overlap gate). Now: outer keeps testid/role/sizing, the chips ride
    // an inner scroll strip with `min-w-0` (shrink-to-fit + ellipsis),
    // and `tab-new` is pinned OUTSIDE the scroller (shrink-0) so it is
    // always visible and always contained. No new chrome row, no new
    // primitive, heights/testids/wiring untouched.
    // Wave-213: chips keep a token floor (--ac-tab-min-w, 96px) with the close
    // `ml-auto` right-pinned, so the close stays inside its chip and tab
    // centers stay clickable even at 8 tabs (cap raised in ChromeShell).
    // H1: `tab-new` last tab ke turant bagal me (BC jaisa). Pehle inner
    // strip `flex-1` tha — kam tabs par poori row kha kar `+` ko door
    // right edge par dhakel deta tha (~200px gap). Ab `flex-initial`
    // (grow 0, shrink 1): content hug karta hai (`+` adjacent), zyada
    // tabs par shrink + internal scroll (testid/role/sizing same).
    // TABS2: chips `flex-1 basis-0` (equal share, basis content se free)
    // + min/max-w tokens (96/180 BC clamps): kam tabs par saare chips
    // barabar width lete hain, zyada tabs par uniformly shrink hokar
    // min floor par internal scroll (strip/scroll/heights untouched,
    // close `ml-auto` still right-pinned).
    <div
      data-testid="tabbar"
      role="tablist"
      aria-label="Open tabs"
      className={`flex h-[var(--ac-tabbar-h)] items-center gap-[var(--ac-space-1)] overflow-hidden bg-[var(--ac-tab-bg)] px-[var(--ac-space-3)]${className ? ` ${className}` : ""}`}
    >
      <div className="flex min-w-0 flex-initial items-center gap-[var(--ac-space-1)] overflow-x-auto">
      {visibleTabs.map((tab) => {
        const active = tab.id === activeTabId;
        const dirty = isTabDirty(tab);
        const closable = isTabClosable(tab);
        const confirming = armedConfirmId === tab.id && dirty;
        return (
          <div
            key={tab.id}
            role="tab"
            aria-selected={active}
            data-testid={`tab-${tab.id}`}
            onClick={() => safeSelect(tab.id)}
            onKeyDown={(e) => {
              if (e.key === "Enter" || e.key === " ") safeSelect(tab.id);
            }}
            onMouseDown={(e) => {
              if (e.button === 1) {
                e.preventDefault();
                handleMiddleClick(tab);
              }
            }}
            onAuxClick={(e) => {
              if (e.button === 1) handleMiddleClick(tab);
            }}
            tabIndex={0}
            className={
              active
                ? "ac-chrome-label flex h-[var(--ac-tab-chip-h)] min-w-[var(--ac-tab-min-w)] max-w-[var(--ac-tab-max-w)] flex-1 basis-0 cursor-pointer select-none items-center gap-[var(--ac-space-1)] rounded-[5px] border border-b-0 border-[var(--ac-border)] bg-[var(--ac-tab-active-bg)] pl-[var(--ac-space-2)] pr-[var(--ac-space-1)] font-medium text-[var(--ac-tab-active-fg)] shadow-[inset_0_1px_0_rgba(255,255,255,0.35),0_1px_2px_rgb(0_0_0/0.05)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]"
                : "ac-chrome-label flex h-[var(--ac-tab-chip-h)] min-w-[var(--ac-tab-min-w)] max-w-[var(--ac-tab-max-w)] flex-1 basis-0 cursor-pointer select-none items-center gap-[var(--ac-space-1)] rounded-[5px] pl-[var(--ac-space-2)] pr-[var(--ac-space-1)] font-normal text-[var(--ac-tab-fg)] hover:bg-[var(--ac-toolbar-btn-hover-bg)] hover:text-[var(--ac-fg)] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]"
            }
          >
            <span className="min-w-0 truncate">{tab.title}</span>
            {dirty && (
              <span
                data-testid={`tab-dirty-${tab.id}`}
                aria-label="unsaved changes"
                title="Unsaved changes"
                className="inline-block size-1.5 shrink-0 rounded-full bg-[var(--ac-tab-dirty)]"
              />
            )}
            {closable ? (
              confirming ? (
                <span
                  data-testid={`tab-close-prompt-${tab.id}`}
                  role="group"
                  aria-label={`Unsaved changes in ${tab.title}. Confirm close?`}
                  onKeyDown={(e) => {
                    if (e.key === "Escape") {
                      e.stopPropagation();
                      cancelClose();
                    }
                  }}
                  className="flex shrink-0 items-center gap-[var(--ac-space-1)]"
                >
                  <ToolbarButton
                    data-testid={`tab-close-confirm-${tab.id}`}
                    label={`Discard changes and close ${tab.title}`}
                    title={`Discard changes and close ${tab.title}`}
                    size="sm"
                    icon={Check}
                    onClick={(e) => {
                      e.stopPropagation();
                      confirmDiscard(tab);
                    }}
                    className="shrink-0"
                  />
                  <ToolbarButton
                    data-testid={`tab-close-cancel-${tab.id}`}
                    label={`Keep ${tab.title} open`}
                    title={`Keep ${tab.title} open`}
                    size="sm"
                    icon={X}
                    onClick={(e) => {
                      e.stopPropagation();
                      cancelClose();
                    }}
                    className="shrink-0"
                  />
                </span>
              ) : (
                <ToolbarButton
                  data-testid={`tab-close-${tab.id}`}
                  label={dirty ? `Close ${tab.title} (unsaved changes)` : `Close ${tab.title}`}
                  title={dirty ? `Close ${tab.title} (unsaved changes)` : `Close ${tab.title}`}
                  size="sm"
                  icon={X}
                  onClick={(e) => {
                    e.stopPropagation();
                    requestClose(tab);
                  }}
                  // Wave-213: px-1 via inline style (className px-1 loses
                  // to the primitive's px-1.5 in the utilities layer).
                  // 24px close keeps 8-tab chips' centers clear of the
                  // button so tab clicks select instead of arming
                  // close-confirm; hit-area specs pin chip/new-tab, not
                  // the close affordance.
                  style={{ paddingInline: "var(--ac-space-1)" }}
                  className="ml-auto shrink-0"
                />
              )
            ) : null}
          </div>
        );
      })}
      </div>
      <ToolbarButton
        data-testid="tab-new"
        label="New tab"
        title="New tab"
        size="md"
        icon={Plus}
        onClick={() => {
          try {
            onNewTab("home");
          } catch (err: unknown) {
            setCloseError(err instanceof Error ? err.message : String(err));
          }
        }}
        // Wave-158: token size md (26px) via ToolbarButton; the 32px
        // hit-area exception is retired (tab-chip --ac-tab-chip-h documents
        // the only 32px chrome left). Testid/aria/wiring untouched.
        className="shrink-0"
      />
      {closeError && (
        <span
          role="alert"
          data-testid="tabbar/error"
          className="ml-1 flex max-w-72 shrink-0 items-center gap-1 truncate rounded border border-[var(--ac-alert-border)] bg-[var(--ac-alert-bg)] px-2 py-0.5 text-[11px] font-medium text-[var(--ac-alert-fg)]"
        >
          <span className="truncate">{closeError}</span>
          <Button
            variant="ghost"
            size="sm"
            aria-label="Dismiss tab error"
            onClick={() => setCloseError(null)}
            className="shrink-0 underline hover:no-underline text-[11px] focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-[var(--ac-focus)]"
          >
            Dismiss
          </Button>
        </span>
      )}
    </div>
  );
}

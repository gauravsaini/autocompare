import React, { useEffect, useRef, useState } from "react";
import { Check, ChevronRight } from "lucide-react";
import { listComparators } from "../plugins/registry";
import type { SessionKind, V2Tab, V2ViewCommands } from "../types";

export interface V2MenuBarProps {
  readonly activeTab: V2Tab;
  readonly commands: V2ViewCommands;
  readonly showToolbar: boolean;
  readonly layoutDensity: "standard" | "compact";
  readonly theme: "light" | "dark";
  readonly onNewSessionModal: () => void;
  readonly onOpenSessionKind: (kind: SessionKind, left?: string, right?: string) => void;
  readonly onOpenSessionManager: () => void;
  readonly onSaveSession: () => void;
  readonly onCloseActiveTab: () => void;
  readonly onOpenSettings: () => void;
  readonly onOpenFileFormats: () => void;
  readonly onOpenProfiles: () => void;
  readonly onOpenExportReport: () => void;
  readonly onOpenShortcuts: () => void;
  readonly onOpenPalette: () => void;
  readonly onToggleToolbar: () => void;
  readonly onToggleLayout: () => void;
  readonly onToggleTheme: () => void;
  readonly onExportSettings: () => void;
  readonly onImportSettings: () => void;
  readonly onRestoreDefaults: () => void;
  readonly onShowToast: (msg: string) => void;
}

type MenuKey = "app" | "session" | "search" | "view" | "tools" | "help";

interface MenuItemRowProps {
  readonly label: string;
  readonly shortcut?: string;
  readonly checked?: boolean;
  readonly disabled?: boolean;
  readonly onClick: () => void;
}

function MenuItemRow({
  label,
  shortcut,
  checked,
  disabled,
  onClick,
}: MenuItemRowProps): React.JSX.Element {
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className="flex w-full items-center justify-between gap-4 rounded-[5px] px-2.5 py-1 text-left text-[12px] text-[var(--v2-fg)] hover:bg-[var(--v2-accent)] hover:text-white disabled:pointer-events-none disabled:opacity-40"
    >
      <span className="flex items-center gap-1.5">
        <span className="inline-flex w-3.5 items-center justify-center">
          {checked ? <Check size={11} strokeWidth={2.5} /> : null}
        </span>
        <span>{label}</span>
      </span>
      {shortcut ? (
        <span className="font-mono text-[10.5px] opacity-65">{shortcut}</span>
      ) : null}
    </button>
  );
}

function MenuSep(): React.JSX.Element {
  return <div className="my-1 h-px bg-[var(--v2-border)]" />;
}

export default function V2MenuBar({
  activeTab,
  commands,
  showToolbar,
  layoutDensity,
  theme,
  onNewSessionModal,
  onOpenSessionKind,
  onOpenSessionManager,
  onSaveSession,
  onCloseActiveTab,
  onOpenSettings,
  onOpenFileFormats,
  onOpenProfiles,
  onOpenExportReport,
  onOpenShortcuts,
  onOpenPalette,
  onToggleToolbar,
  onToggleLayout,
  onToggleTheme,
  onExportSettings,
  onImportSettings,
  onRestoreDefaults,
  onShowToast,
}: V2MenuBarProps): React.JSX.Element {
  const [openMenu, setOpenMenu] = useState<MenuKey | null>(null);
  const [openSub, setOpenSub] = useState<"new-session" | "compare-using" | null>(null);
  const navRef = useRef<HTMLElement | null>(null);
  const comparators = listComparators();
  const inComparison = activeTab.kind !== "home";

  useEffect(() => {
    if (!openMenu) return;
    const onDown = (e: MouseEvent): void => {
      if (navRef.current && !navRef.current.contains(e.target as Node)) {
        setOpenMenu(null);
        setOpenSub(null);
      }
    };
    const onKey = (e: KeyboardEvent): void => {
      if (e.key === "Escape") {
        setOpenMenu(null);
        setOpenSub(null);
      }
    };
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [openMenu]);

  const run = (fn: () => void): void => {
    setOpenMenu(null);
    setOpenSub(null);
    fn();
  };

  const menus: readonly { key: MenuKey; label: string; bold?: boolean }[] = [
    { key: "app", label: "AutoCompare", bold: true },
    { key: "session", label: "Session" },
    { key: "search", label: "Search" },
    { key: "view", label: "View" },
    { key: "tools", label: "Tools" },
    { key: "help", label: "Help" },
  ];

  return (
    <nav
      ref={navRef}
      data-testid="v2/menubar"
      aria-label="Application menu bar"
      className="relative z-30 flex h-[26px] shrink-0 items-center justify-between border-b border-[var(--v2-border)] bg-[var(--v2-chrome-elevated)] px-2 text-[12px] select-none"
    >
      <div className="flex items-center gap-0.5">
        {menus.map((m) => {
          const active = openMenu === m.key;
          return (
            <div key={m.key} className="relative">
              <button
                type="button"
                data-testid={`v2/menu-${m.key}`}
                onClick={() => {
                  setOpenMenu(active ? null : m.key);
                  setOpenSub(null);
                }}
                onMouseEnter={() => {
                  if (openMenu && openMenu !== m.key) {
                    setOpenMenu(m.key);
                    setOpenSub(null);
                  }
                }}
                className={`rounded-[5px] px-2 py-0.5 transition ${
                  m.bold ? "font-semibold" : "font-normal"
                } ${
                  active
                    ? "bg-[var(--v2-accent)] text-white"
                    : "text-[var(--v2-fg)] hover:bg-[var(--v2-btn-hover)]"
                }`}
              >
                {m.label}
              </button>

              {active ? (
                <div className="absolute left-0 top-full mt-0.5 min-w-[230px] rounded-[8px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-1 shadow-lg">
                  {m.key === "app" ? (
                    <>
                      <MenuItemRow
                        label="About AutoCompare (Free & OSS)"
                        onClick={() =>
                          run(() =>
                            onShowToast(
                              `AutoCompare v0.5.0 · Free & Open Source · ${comparators.length} plugins loaded`,
                            ),
                          )
                        }
                      />
                      <MenuSep />
                      <MenuItemRow label="Settings…" shortcut="⌘," onClick={() => run(onOpenSettings)} />
                      <MenuItemRow label="File Formats…" onClick={() => run(onOpenFileFormats)} />
                      <MenuItemRow label="Profiles…" onClick={() => run(onOpenProfiles)} />
                      <MenuSep />
                      <MenuItemRow
                        label="Install Command Line Tools…"
                        onClick={() =>
                          run(() => {
                            const cmd = "cargo install --path crates/cli";
                            void navigator.clipboard?.writeText(cmd).catch(() => undefined);
                            onShowToast(`Copied CLI install command: ${cmd}`);
                          })
                        }
                      />
                    </>
                  ) : null}

                  {m.key === "session" ? (
                    <>
                      {/* Dynamic Plugin-Driven New Session Submenu */}
                      <div
                        className="relative"
                        onMouseEnter={() => setOpenSub("new-session")}
                      >
                        <button
                          type="button"
                          onClick={() => run(onNewSessionModal)}
                          className="flex w-full items-center justify-between rounded-[5px] px-2.5 py-1 text-left text-[12px] text-[var(--v2-fg)] hover:bg-[var(--v2-accent)] hover:text-white"
                        >
                          <span className="pl-5">New Session</span>
                          <ChevronRight size={12} />
                        </button>
                        {openSub === "new-session" ? (
                          <div className="absolute left-full top-0 ml-1 min-w-[215px] rounded-[8px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-1 shadow-lg">
                            {comparators.map((plugin) => (
                              <MenuItemRow
                                key={plugin.id}
                                label={plugin.title}
                                shortcut={plugin.shortcut}
                                onClick={() => run(() => onOpenSessionKind(plugin.id))}
                              />
                            ))}
                          </div>
                        ) : null}
                      </div>

                      <MenuItemRow label="New Tab (Grid)…" shortcut="⌘T" onClick={() => run(onNewSessionModal)} />
                      <MenuItemRow label="Open Session…" shortcut="⌃⌘O" onClick={() => run(onOpenSessionManager)} />
                      <MenuSep />
                      <MenuItemRow
                        label="Close Tab"
                        shortcut="⌘W"
                        disabled={!inComparison}
                        onClick={() => run(onCloseActiveTab)}
                      />
                      <MenuItemRow
                        label="Save Session"
                        shortcut="⌘S"
                        disabled={!inComparison}
                        onClick={() => run(onSaveSession)}
                      />
                      <MenuItemRow
                        label="Session Settings…"
                        disabled={!commands.openRules}
                        onClick={() => run(() => commands.openRules?.())}
                      />
                      <MenuSep />
                      <MenuItemRow
                        label="Swap Sides"
                        disabled={!commands.swapSides}
                        onClick={() => run(() => commands.swapSides?.())}
                      />
                      <MenuItemRow
                        label="Reload Files"
                        shortcut="⌘R"
                        disabled={!commands.reload}
                        onClick={() => run(() => commands.reload?.())}
                      />
                      <MenuItemRow
                        label="Compare Report…"
                        shortcut="⌘P"
                        disabled={!inComparison}
                        onClick={() => run(onOpenExportReport)}
                      />
                      <MenuSep />

                      {/* Dynamic Plugin-Driven Compare Files Using Submenu */}
                      <div
                        className="relative"
                        onMouseEnter={() => setOpenSub("compare-using")}
                      >
                        <button
                          type="button"
                          disabled={!inComparison}
                          className="flex w-full items-center justify-between rounded-[5px] px-2.5 py-1 text-left text-[12px] text-[var(--v2-fg)] hover:bg-[var(--v2-accent)] hover:text-white disabled:pointer-events-none disabled:opacity-40"
                        >
                          <span className="pl-5">Compare Files Using</span>
                          <ChevronRight size={12} />
                        </button>
                        {openSub === "compare-using" && inComparison ? (
                          <div className="absolute left-full top-0 ml-1 min-w-[215px] rounded-[8px] border border-[var(--v2-border)] bg-[var(--v2-surface)] p-1 shadow-lg">
                            {comparators.map((plugin) => (
                              <MenuItemRow
                                key={plugin.id}
                                label={plugin.title}
                                checked={activeTab.kind === plugin.id}
                                onClick={() =>
                                  run(() =>
                                    onOpenSessionKind(plugin.id, activeTab.leftPath, activeTab.rightPath),
                                  )
                                }
                              />
                            ))}
                          </div>
                        ) : null}
                      </div>
                    </>
                  ) : null}

                  {m.key === "search" ? (
                    <>
                      <MenuItemRow
                        label="Next Difference"
                        shortcut="⌥↓"
                        disabled={!commands.hasSections}
                        onClick={() => run(() => commands.nextSection?.())}
                      />
                      <MenuItemRow
                        label="Previous Difference"
                        shortcut="⌥↑"
                        disabled={!commands.hasSections}
                        onClick={() => run(() => commands.prevSection?.())}
                      />
                      <MenuSep />
                      <MenuItemRow
                        label="Find…"
                        shortcut="⌘F"
                        disabled={!commands.openFind}
                        onClick={() => run(() => commands.openFind?.())}
                      />
                      <MenuItemRow label="Command Palette…" shortcut="⌘K" onClick={() => run(onOpenPalette)} />
                    </>
                  ) : null}

                  {m.key === "view" ? (
                    <>
                      <MenuItemRow
                        label="Show All"
                        checked={commands.filterMode === "all"}
                        disabled={!commands.setFilterMode}
                        onClick={() => run(() => commands.setFilterMode?.("all"))}
                      />
                      <MenuItemRow
                        label="Show Differences"
                        checked={commands.filterMode === "diffs"}
                        disabled={!commands.setFilterMode}
                        onClick={() => run(() => commands.setFilterMode?.("diffs"))}
                      />
                      <MenuItemRow
                        label="Show Same"
                        checked={commands.filterMode === "same"}
                        disabled={!commands.setFilterMode}
                        onClick={() => run(() => commands.setFilterMode?.("same"))}
                      />
                      <MenuSep />
                      <MenuItemRow
                        label="Ignore Unimportant / Minor"
                        checked={commands.minorEnabled === true}
                        disabled={!commands.toggleMinor}
                        onClick={() => run(() => commands.toggleMinor?.())}
                      />
                      <MenuItemRow
                        label="Context Lines"
                        checked={commands.contextEnabled === true}
                        disabled={!commands.toggleContext}
                        onClick={() => run(() => commands.toggleContext?.())}
                      />
                      <MenuSep />
                      <MenuItemRow label="Toolbar" checked={showToolbar} onClick={() => run(onToggleToolbar)} />
                      <MenuItemRow
                        label="Compact Layout"
                        checked={layoutDensity === "compact"}
                        onClick={() => run(onToggleLayout)}
                      />
                      <MenuItemRow label="Dark Mode" checked={theme === "dark"} onClick={() => run(onToggleTheme)} />
                      <MenuSep />
                      <MenuItemRow
                        label="Toggle Full Screen"
                        onClick={() =>
                          run(() => {
                            if (document.fullscreenElement) {
                              void document.exitFullscreen().catch(() => undefined);
                            } else {
                              void document.documentElement.requestFullscreen?.().catch(() => undefined);
                            }
                          })
                        }
                      />
                    </>
                  ) : null}

                  {m.key === "tools" ? (
                    <>
                      <MenuItemRow label="Export Settings…" onClick={() => run(onExportSettings)} />
                      <MenuItemRow label="Import Settings…" onClick={() => run(onImportSettings)} />
                      <MenuItemRow label="Restore Factory Defaults…" onClick={() => run(onRestoreDefaults)} />
                      <MenuSep />
                      <MenuItemRow label="File Formats…" onClick={() => run(onOpenFileFormats)} />
                      <MenuItemRow label="Profiles…" onClick={() => run(onOpenProfiles)} />
                      <MenuSep />
                      <MenuItemRow
                        label="View Patch / Export Report…"
                        disabled={!inComparison}
                        onClick={() => run(onOpenExportReport)}
                      />
                    </>
                  ) : null}

                  {m.key === "help" ? (
                    <>
                      <MenuItemRow label="Keyboard Shortcuts" shortcut="⌘/" onClick={() => run(onOpenShortcuts)} />
                      <MenuItemRow label="Command Palette" shortcut="⌘K" onClick={() => run(onOpenPalette)} />
                      <MenuSep />
                      <MenuItemRow
                        label="About AutoCompare (Free & Open Source)"
                        onClick={() =>
                          run(() =>
                            onShowToast(
                              `AutoCompare v0.5.0 · 100% Free & Open Source · ${comparators.length} comparators`,
                            ),
                          )
                        }
                      />
                    </>
                  ) : null}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>

      <span className="text-[10.5px] text-[var(--v2-fg-muted)]">
        {comparators.length} plugins · Free &amp; Open Source
      </span>
    </nav>
  );
}

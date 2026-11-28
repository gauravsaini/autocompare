import React, { useEffect, useRef, useState } from "react";
import { Check, ChevronRight } from "lucide-react";
import {
  MENU_SCHEMA,
  isMenuItemChecked,
  isMenuItemEnabled,
  isViewAllowed,
  type MenuEvalContext,
} from "../menus/menuSchema";
import { listComparators } from "../plugins/registry";
import type { SessionKind, V2Tab, V2ViewCommands } from "../types";

export interface V2MenuBarProps {
  readonly activeTab: V2Tab;
  readonly commands: V2ViewCommands;
  readonly showToolbar: boolean;
  readonly layoutDensity: "standard" | "compact";
  readonly paneOrientation: "side-by-side" | "over-under";
  readonly theme: "light" | "dark";
  readonly onCommand: (command: string) => void;
  readonly onOpenSessionKind: (kind: SessionKind, left?: string, right?: string) => void;
}

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
      className="flex w-full items-center justify-between gap-4 rounded-lg px-2.5 py-1.5 text-left text-[12px] text-[var(--v2-fg)] transition-colors hover:bg-[var(--v2-accent)] hover:text-white disabled:pointer-events-none disabled:opacity-40"
    >
      <span className="flex items-center gap-1.5">
        <span className="inline-flex w-3.5 items-center justify-center">
          {checked ? <Check size={11} strokeWidth={2.5} /> : null}
        </span>
        <span>{label}</span>
      </span>
      {shortcut ? (
        <span className="v2-mono text-[10.5px] opacity-65">{shortcut}</span>
      ) : null}
    </button>
  );
}

function MenuSep(): React.JSX.Element {
  return <div className="my-1 h-px bg-[var(--v2-border-subtle)]" />;
}

export default function V2MenuBar({
  activeTab,
  commands,
  showToolbar,
  layoutDensity,
  paneOrientation,
  theme,
  onCommand,
  onOpenSessionKind,
}: V2MenuBarProps): React.JSX.Element {
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const [openSub, setOpenSub] = useState<string | null>(null);
  const navRef = useRef<HTMLElement | null>(null);
  const comparators = listComparators();

  const evalCtx: MenuEvalContext = {
    activeViewId: activeTab.kind,
    commands,
    showToolbar,
    layoutDensity,
    paneOrientation,
    theme,
  };

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

  return (
    <nav
      ref={navRef}
      data-testid="v2/menubar"
      aria-label="Application menu bar"
      className="relative z-30 flex h-[28px] shrink-0 items-center justify-between border-b border-[var(--v2-border)] bg-[var(--v2-surface)] px-2.5 text-[12px] select-none"
    >
      <div className="flex items-center gap-0.5">
        {MENU_SCHEMA.menus.map((menu) => {
          const active = openMenu === menu.id;
          return (
            <div key={menu.id} className="relative">
              <button
                type="button"
                data-testid={`v2/menu-${menu.id}`}
                onClick={() => {
                  setOpenMenu(active ? null : menu.id);
                  setOpenSub(null);
                }}
                onMouseEnter={() => {
                  if (openMenu && openMenu !== menu.id) {
                    setOpenMenu(menu.id);
                    setOpenSub(null);
                  }
                }}
                className={`rounded-md px-2 py-0.5 transition-colors ${
                  menu.bold ? "font-semibold" : "font-medium"
                } ${
                  active
                    ? "bg-[var(--v2-accent)] text-white"
                    : "text-[var(--v2-fg-secondary)] hover:bg-[var(--v2-surface-subtle)] hover:text-[var(--v2-fg)]"
                }`}
              >
                {menu.label}
              </button>

              {active ? (
                <div
                  style={{ boxShadow: "var(--v2-shadow-md)" }}
                  className="absolute left-0 top-full mt-1 min-w-[240px] rounded-xl border border-[var(--v2-border)] bg-[var(--v2-surface)] p-1.5"
                >
                  {menu.items.map((item, idx) => {
                    if (item.kind === "separator") {
                      return item.nativeOnly ? null : <MenuSep key={`sep-${idx}`} />;
                    }
                    if (item.kind === "action") {
                      if (item.nativeOnly) return null;
                      const enabled = isMenuItemEnabled(item, evalCtx);
                      const checked = isMenuItemChecked(item.checkedBy, evalCtx);
                      return (
                        <MenuItemRow
                          key={item.id}
                          label={item.label}
                          shortcut={item.shortcutDisplay}
                          checked={checked}
                          disabled={!enabled}
                          onClick={() => run(() => onCommand(item.command))}
                        />
                      );
                    }
                    // Dynamic plugin-driven submenu slot
                    const allowed = isViewAllowed(item.requiresView, activeTab.kind);
                    return (
                      <div
                        key={item.id}
                        className="relative"
                        onMouseEnter={() => setOpenSub(item.id)}
                      >
                        <button
                          type="button"
                          disabled={!allowed}
                          onClick={() => {
                            if (item.slot === "new-session-plugins") {
                              run(() => onCommand("session_new"));
                            }
                          }}
                          className="flex w-full items-center justify-between rounded-lg px-2.5 py-1.5 text-left text-[12px] text-[var(--v2-fg)] transition-colors hover:bg-[var(--v2-accent)] hover:text-white disabled:pointer-events-none disabled:opacity-40"
                        >
                          <span className="pl-5">{item.label}</span>
                          <ChevronRight size={12} />
                        </button>
                        {openSub === item.id && allowed ? (
                          <div
                            style={{ boxShadow: "var(--v2-shadow-md)" }}
                            className="absolute left-full top-0 ml-1.5 min-w-[220px] rounded-xl border border-[var(--v2-border)] bg-[var(--v2-surface)] p-1.5"
                          >
                            {comparators.map((plugin) => (
                              <MenuItemRow
                                key={plugin.id}
                                label={plugin.title}
                                shortcut={
                                  item.slot === "new-session-plugins" ? plugin.shortcut : undefined
                                }
                                checked={
                                  item.slot === "compare-using-plugins"
                                    ? activeTab.kind === plugin.id
                                    : undefined
                                }
                                onClick={() =>
                                  run(() =>
                                    item.slot === "compare-using-plugins"
                                      ? onOpenSessionKind(
                                          plugin.id,
                                          activeTab.leftPath,
                                          activeTab.rightPath,
                                        )
                                      : onOpenSessionKind(plugin.id),
                                  )
                                }
                              />
                            ))}
                          </div>
                        ) : null}
                      </div>
                    );
                  })}
                </div>
              ) : null}
            </div>
          );
        })}
      </div>

      <span className="v2-mono text-[10.5px] text-[var(--v2-fg-muted)]">
        {comparators.length} plugins · Free &amp; Open Source
      </span>
    </nav>
  );
}

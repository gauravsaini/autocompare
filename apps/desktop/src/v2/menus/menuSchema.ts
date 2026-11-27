import rawSchema from "./menuSchema.json" with { type: "json" };
import type { SessionKind, V2ViewCommands } from "../types";

export type MenuCapabilityKey =
  | "setFilterMode"
  | "toggleContext"
  | "toggleMinor"
  | "openRules"
  | "openFormat"
  | "openFind"
  | "canCopy"
  | "hasSections"
  | "swapSides"
  | "reload";

export type MenuCheckedKey =
  | "filterMode:all"
  | "filterMode:diffs"
  | "filterMode:same"
  | "minorEnabled"
  | "contextEnabled"
  | "showToolbar"
  | "compactLayout"
  | "darkMode";

export interface MenuActionItemDef {
  readonly kind: "action";
  readonly id: string;
  readonly command: string;
  readonly label: string;
  readonly accelerator?: string;
  readonly shortcutDisplay?: string;
  readonly requiresView?: "comparison" | readonly SessionKind[];
  readonly requiresCapability?: MenuCapabilityKey;
  readonly checkedBy?: MenuCheckedKey;
  readonly nativeRole?: "about" | "quit" | "fullscreen";
  readonly nativeOnly?: boolean;
}

export interface MenuSeparatorDef {
  readonly kind: "separator";
  readonly nativeOnly?: boolean;
}

export interface MenuDynamicSubmenuDef {
  readonly kind: "dynamic-submenu";
  readonly id: string;
  readonly label: string;
  readonly slot: "new-session-plugins" | "compare-using-plugins";
  readonly requiresView?: "comparison" | readonly SessionKind[];
}

export type MenuItemDef = MenuActionItemDef | MenuSeparatorDef | MenuDynamicSubmenuDef;

export interface TopLevelMenuDef {
  readonly id: string;
  readonly label: string;
  readonly bold?: boolean;
  readonly items: readonly MenuItemDef[];
}

export interface MenuSchemaDef {
  readonly version: number;
  readonly menus: readonly TopLevelMenuDef[];
}

export interface MenuEvalContext {
  readonly activeViewId: SessionKind;
  readonly commands: V2ViewCommands;
  readonly showToolbar: boolean;
  readonly layoutDensity: "standard" | "compact";
  readonly theme: "light" | "dark";
}

export const MENU_SCHEMA: MenuSchemaDef = rawSchema as MenuSchemaDef;

export function isViewAllowed(
  requiresView: MenuActionItemDef["requiresView"],
  activeViewId: SessionKind,
): boolean {
  if (!requiresView) return true;
  if (requiresView === "comparison") return activeViewId !== "home";
  return requiresView.includes(activeViewId);
}

export function hasCapability(
  capability: MenuCapabilityKey | undefined,
  commands: V2ViewCommands,
): boolean {
  if (!capability) return true;
  switch (capability) {
    case "setFilterMode":
      return typeof commands.setFilterMode === "function";
    case "toggleContext":
      return typeof commands.toggleContext === "function";
    case "toggleMinor":
      return typeof commands.toggleMinor === "function";
    case "openRules":
      return typeof commands.openRules === "function";
    case "openFormat":
      return typeof commands.openFormat === "function";
    case "openFind":
      return typeof commands.openFind === "function";
    case "canCopy":
      return commands.canCopy === true;
    case "hasSections":
      return commands.hasSections === true;
    case "swapSides":
      return typeof commands.swapSides === "function";
    case "reload":
      return typeof commands.reload === "function";
  }
}

export function isMenuItemEnabled(
  item: Pick<MenuActionItemDef, "requiresView" | "requiresCapability">,
  ctx: Pick<MenuEvalContext, "activeViewId" | "commands">,
): boolean {
  return (
    isViewAllowed(item.requiresView, ctx.activeViewId) &&
    hasCapability(item.requiresCapability, ctx.commands)
  );
}

export function isMenuItemChecked(
  checkedBy: MenuCheckedKey | undefined,
  ctx: MenuEvalContext,
): boolean {
  if (!checkedBy) return false;
  switch (checkedBy) {
    case "filterMode:all":
      return ctx.commands.filterMode === "all";
    case "filterMode:diffs":
      return ctx.commands.filterMode === "diffs";
    case "filterMode:same":
      return ctx.commands.filterMode === "same";
    case "minorEnabled":
      return ctx.commands.minorEnabled === true;
    case "contextEnabled":
      return ctx.commands.contextEnabled === true;
    case "showToolbar":
      return ctx.showToolbar;
    case "compactLayout":
      return ctx.layoutDensity === "compact";
    case "darkMode":
      return ctx.theme === "dark";
  }
}

/**
 * Computes the complete `{ [menuItemId]: enabled }` map for syncing native
 * Tauri menus (`menu.rs`) with the active view and capabilities.
 */
export function computeNativeMenuEnabledMap(
  ctx: Pick<MenuEvalContext, "activeViewId" | "commands">,
): Record<string, boolean> {
  const map: Record<string, boolean> = {};
  for (const menu of MENU_SCHEMA.menus) {
    for (const item of menu.items) {
      if (item.kind === "action") {
        map[item.id] = isMenuItemEnabled(item, ctx);
      }
    }
  }
  return map;
}

export function findCommandByMenuId(menuId: string): string | undefined {
  for (const menu of MENU_SCHEMA.menus) {
    for (const item of menu.items) {
      if (item.kind === "action" && item.id === menuId) {
        return item.command;
      }
    }
  }
  return undefined;
}

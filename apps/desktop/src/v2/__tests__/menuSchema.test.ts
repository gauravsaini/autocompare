import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  MENU_SCHEMA,
  computeNativeMenuEnabledMap,
  findCommandByMenuId,
  isMenuItemChecked,
  isMenuItemEnabled,
} from "../menus/menuSchema.ts";

describe("Unified Menu Schema (V2MenuBar + native menu.rs single source)", () => {
  it("contains valid namespaced ids and commands for every action item", () => {
    const seenIds = new Set<string>();
    assert.ok(MENU_SCHEMA.menus.length >= 6);
    for (const menu of MENU_SCHEMA.menus) {
      for (const item of menu.items) {
        if (item.kind === "action") {
          assert.ok(item.id.includes("."), `expected namespaced id: ${item.id}`);
          assert.ok(item.command.length > 0, `missing command on ${item.id}`);
          assert.equal(seenIds.has(item.id), false, `duplicate menu id: ${item.id}`);
          seenIds.add(item.id);
          assert.equal(findCommandByMenuId(item.id), item.command);
        }
      }
    }
  });

  it("filters enabled/disabled states strictly by active ViewId and capabilities", () => {
    const homeMap = computeNativeMenuEnabledMap({
      activeViewId: "home",
      commands: {},
    });
    assert.equal(homeMap["tools.settings"], true);
    assert.equal(homeMap["session.new"], true);
    assert.equal(homeMap["file.save-session"], false);
    assert.equal(homeMap["search.next-diff"], false);
    assert.equal(homeMap["session.take-left"], false);

    const textMap = computeNativeMenuEnabledMap({
      activeViewId: "text",
      commands: {
        hasSections: true,
        canCopy: true,
        setFilterMode: () => undefined,
        filterMode: "diffs",
      },
    });
    assert.equal(textMap["file.save-session"], true);
    assert.equal(textMap["search.next-diff"], true);
    assert.equal(textMap["session.take-left"], true);
    assert.equal(textMap["session.rules"], false);
    assert.equal(textMap["view.filter-diffs"], true);

    assert.equal(
      isMenuItemEnabled({ requiresView: ["text", "hex"] }, { activeViewId: "folder", commands: {} }),
      false,
    );
    assert.equal(
      isMenuItemChecked("filterMode:diffs", {
        activeViewId: "text",
        commands: { filterMode: "diffs" },
        showToolbar: true,
        layoutDensity: "compact",
        theme: "dark",
      }),
      true,
    );
  });
});

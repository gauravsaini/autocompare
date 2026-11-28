import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  DEFAULT_SIDEBAR_WIDTH,
  MAX_SIDEBAR_WIDTH,
  MIN_SIDEBAR_WIDTH,
  clampSidebarWidth,
  loadDemoModePref,
  loadHomeSidebarPrefs,
  saveDemoModePref,
  saveHomeSidebarPrefs,
} from "../utils/homePrefs.ts";
import {
  matchSavedSessions,
  parseSpotlightPaths,
} from "../utils/spotlightSearch.ts";

class MemoryStorage {
  private readonly map = new Map<string, string>();
  getItem(key: string): string | null {
    return this.map.has(key) ? (this.map.get(key) ?? null) : null;
  }
  setItem(key: string, value: string): void {
    this.map.set(key, value);
  }
}

describe("homePrefs (Sidebar width, dock & Demo Mode persistence)", () => {
  it("clamps sidebar widths safely within bounds", () => {
    assert.equal(clampSidebarWidth(100), MIN_SIDEBAR_WIDTH);
    assert.equal(clampSidebarWidth(999), MAX_SIDEBAR_WIDTH);
    assert.equal(clampSidebarWidth(312.6), 313);
    assert.equal(clampSidebarWidth(Number.NaN), DEFAULT_SIDEBAR_WIDTH);
  });

  it("round-trips open/collapsed state and width through storage", () => {
    const store = new MemoryStorage();
    assert.deepEqual(loadHomeSidebarPrefs(store), { open: true, width: DEFAULT_SIDEBAR_WIDTH });

    saveHomeSidebarPrefs({ open: false, width: 340 }, store);
    assert.deepEqual(loadHomeSidebarPrefs(store), { open: false, width: 340 });

    saveHomeSidebarPrefs({ open: true, width: 9999 }, store);
    assert.deepEqual(loadHomeSidebarPrefs(store), { open: true, width: MAX_SIDEBAR_WIDTH });
  });

  it("falls back cleanly on corrupted storage JSON", () => {
    const store = new MemoryStorage();
    store.setItem("autocompare.v2.home.sidebar.v1", "{bad-json");
    assert.deepEqual(loadHomeSidebarPrefs(store), { open: true, width: DEFAULT_SIDEBAR_WIDTH });
  });

  it("persists Demo Mode toggle defaulting to false", () => {
    const store = new MemoryStorage();
    assert.equal(loadDemoModePref(store), false);
    saveDemoModePref(true, store);
    assert.equal(loadDemoModePref(store), true);
    saveDemoModePref(false, store);
    assert.equal(loadDemoModePref(store), false);
  });
});

describe("spotlightSearch (Path parsing & saved session matching)", () => {
  it("parses space-separated and separator-delimited path pairs", () => {
    const jsonPair = parseSpotlightPaths("/tmp/a.json /tmp/b.json");
    assert.deepEqual(jsonPair, {
      left: "/tmp/a.json",
      right: "/tmp/b.json",
      inferredKind: "structured",
    });

    const arrowPair = parseSpotlightPaths("/tmp/left.rs <-> /tmp/right.rs");
    assert.deepEqual(arrowPair, {
      left: "/tmp/left.rs",
      right: "/tmp/right.rs",
      inferredKind: "text",
    });

    const quotedPair = parseSpotlightPaths('"/Users/me/My Left.csv" vs "/Users/me/My Right.csv"');
    assert.deepEqual(quotedPair, {
      left: "/Users/me/My Left.csv",
      right: "/Users/me/My Right.csv",
      inferredKind: "table",
    });
  });

  it("detects folder pairs and ignores plain plugin search keywords", () => {
    const folderPair = parseSpotlightPaths("/tmp/dir_left/ /tmp/dir_right/");
    assert.equal(folderPair?.inferredKind, "folder");

    assert.equal(parseSpotlightPaths("hex"), null);
    assert.equal(parseSpotlightPaths("folder compare"), null);
    assert.equal(parseSpotlightPaths("json yaml diff"), null);
  });

  it("matches saved sessions case-insensitively with a result cap", () => {
    const sessions = [
      { id: "s1", name: "Prod API Configs" },
      { id: "s2", name: "Release Folder Sync" },
      { id: "s3", name: "API Schema Table" },
    ];
    const hits = matchSavedSessions(sessions, "api", 2);
    assert.deepEqual(
      hits.map((h) => h.id),
      ["s1", "s3"],
    );
    assert.deepEqual(matchSavedSessions(sessions, ""), []);
  });
});

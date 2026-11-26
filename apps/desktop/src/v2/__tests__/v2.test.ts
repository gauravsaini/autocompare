import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { computeVirtualWindow } from "../hooks/useVirtualRows.ts";
import { inferDropKind } from "../utils/dropKind.ts";
import { isMissingFixtureError } from "../utils/demoGuard.ts";
import { detectChangedPaths } from "../utils/fileWatch.ts";
import {
  extractSideLines,
  insertPastedText,
  joinWithNextLine,
  joinWithPreviousLine,
  rowIndexToSideLineIndex,
  sideLineIndexToRowIndex,
  splitLineAt,
} from "../utils/textBuffer.ts";
import {
  DEFAULT_UNIMPORTANT_PRESETS,
  matchesUnderSessionRules,
  matchesWithReplacements,
  stripUnimportantText,
} from "../utils/textReplacements.ts";

describe("computeVirtualWindow", () => {
  it("computes bounded start/end/offset for 10,000 rows", () => {
    const win = computeVirtualWindow(10000, 21, 0, 630, 10);
    assert.equal(win.startIndex, 0);
    assert.equal(win.endIndex, 50);
    assert.equal(win.totalHeight, 210000);
    assert.equal(win.offsetTop, 0);
  });

  it("shifts window cleanly mid-scroll and clamps at bottom", () => {
    const mid = computeVirtualWindow(10000, 20, 4000, 400, 5);
    assert.equal(mid.startIndex, 195);
    assert.equal(mid.endIndex, 225);
    assert.equal(mid.offsetTop, 195 * 20);

    const end = computeVirtualWindow(100, 20, 5000, 400, 5);
    assert.equal(end.startIndex, 100);
    assert.equal(end.endIndex, 100);
  });

  it("handles empty or zero-height gracefully", () => {
    const empty = computeVirtualWindow(0, 21, 100, 600, 16);
    assert.equal(empty.startIndex, 0);
    assert.equal(empty.endIndex, 0);
    assert.equal(empty.totalHeight, 0);
  });
});

describe("inferDropKind", () => {
  it("routes files and directories to the matching comparison mode", () => {
    assert.equal(inferDropKind(["/tmp/data.csv"]), "table");
    assert.equal(inferDropKind(["/tmp/sheet.TSV"]), "table");
    assert.equal(inferDropKind(["/tmp/shot.png"]), "media");
    assert.equal(inferDropKind(["/tmp/clip.wav"]), "media");
    assert.equal(inferDropKind(["/tmp/app.wasm"]), "hex");
    assert.equal(inferDropKind(["/tmp/my-folder/"]), "folder");
    assert.equal(inferDropKind(["/tmp/folder_no_ext"]), "folder");
    assert.equal(inferDropKind(["/tmp/main.rs"]), "text");
  });
});

describe("isMissingFixtureError", () => {
  it("detects missing fixture errors while letting real errors surface", () => {
    assert.equal(isMissingFixtureError(new Error("No such file or directory (os error 2)")), true);
    assert.equal(isMissingFixtureError("ENOENT: file not found"), true);
    assert.equal(isMissingFixtureError(new Error("Permission denied")), false);
  });
});

describe("detectChangedPaths", () => {
  it("detects mtime or byte-size modifications and ignores unchanged or null stats", () => {
    const paths = ["/tmp/left.txt", "/tmp/right.txt"];
    const base = [
      { bytes: 100, mtimeUnix: 1700000000 },
      { bytes: 200, mtimeUnix: 1700000000 },
    ];
    assert.deepEqual(detectChangedPaths(paths, base, base), []);
    assert.deepEqual(
      detectChangedPaths(paths, base, [
        { bytes: 100, mtimeUnix: 1700000005 },
        { bytes: 200, mtimeUnix: 1700000000 },
      ]),
      ["/tmp/left.txt"],
    );
    assert.deepEqual(
      detectChangedPaths(paths, base, [
        { bytes: 100, mtimeUnix: 1700000000 },
        { bytes: 242, mtimeUnix: 1700000000 },
      ]),
      ["/tmp/right.txt"],
    );
    assert.deepEqual(detectChangedPaths(paths, base, [null, null]), []);
  });
});

describe("textBuffer notepad operations", () => {
  const sampleRows = [
    { index: 0, left: "alpha", right: "alpha" },
    { index: 1, left: "only-left", right: null },
    { index: 2, left: "beta", right: "BETA" },
  ];

  it("extracts side lines and maps row <-> side line indices across orphans", () => {
    assert.deepEqual(extractSideLines(sampleRows, "left"), ["alpha", "only-left", "beta"]);
    assert.deepEqual(extractSideLines(sampleRows, "right"), ["alpha", "BETA"]);
    assert.equal(rowIndexToSideLineIndex(sampleRows, 2, "right"), 1);
    assert.equal(rowIndexToSideLineIndex(sampleRows, 1, "right"), -1);
    assert.equal(sideLineIndexToRowIndex(sampleRows, 1, "right"), 2);
  });

  it("splits line on Enter and joins on Backspace/Delete", () => {
    const split = splitLineAt(["hello world", "next"], 0, 5);
    assert.deepEqual(split.lines, ["hello", " world", "next"]);
    assert.equal(split.nextLine, 1);
    assert.equal(split.nextCol, 0);

    const joinedPrev = joinWithPreviousLine(split.lines, 1);
    assert.deepEqual(joinedPrev.lines, ["hello world", "next"]);
    assert.equal(joinedPrev.nextLine, 0);
    assert.equal(joinedPrev.nextCol, 5);

    const joinedNext = joinWithNextLine(["foo", "bar", "baz"], 0);
    assert.deepEqual(joinedNext.lines, ["foobar", "baz"]);
    assert.equal(joinedNext.nextLine, 0);
    assert.equal(joinedNext.nextCol, 3);
  });

  it("handles multi-line paste cleanly", () => {
    const res = insertPastedText(["start end"], 0, 6, 6, "one\ntwo\nthree ");
    assert.deepEqual(res.lines, ["start one", "two", "three end"]);
    assert.equal(res.nextLine, 2);
    assert.equal(res.nextCol, 6);
  });
});

describe("matchesWithReplacements", () => {
  it("matches literal replacements respecting case sensitivity and enabled flag", () => {
    const rules = [
      { id: "1", left: "staging.example.com", right: "prod.example.com", regex: false, caseSensitive: false, enabled: true },
      { id: "2", left: "disabled", right: "ignored", regex: false, caseSensitive: false, enabled: false },
    ];
    assert.equal(
      matchesWithReplacements("https://STAGING.EXAMPLE.COM/api", "https://prod.example.com/api", rules),
      true,
    );
    assert.equal(matchesWithReplacements("disabled", "ignored", rules), false);
  });

  it("supports regex capture groups and safely ignores invalid regex", () => {
    const rules = [
      { id: "r1", left: "v(\\d+)\\.0", right: "release-$1", regex: true, caseSensitive: true, enabled: true },
      { id: "r2", left: "[invalid", right: "x", regex: true, caseSensitive: false, enabled: true },
    ];
    assert.equal(matchesWithReplacements("build v12.0 ok", "build release-12 ok", rules), true);
    assert.equal(matchesWithReplacements("build V12.0 ok", "build release-12 ok", rules), false);
  });
});

describe("unimportant text / grammar rules", () => {
  it("strips ISO timestamps and line comments cleanly", () => {
    const tsRule = { id: "ts", ...DEFAULT_UNIMPORTANT_PRESETS[0] };
    const commentRule = { id: "cm", ...DEFAULT_UNIMPORTANT_PRESETS[1] };
    assert.equal(
      stripUnimportantText("[2026-11-27T08:30:00Z] started // local", [tsRule, commentRule]),
      "[] started ",
    );
  });

  it("folds rows under minorEnabled + unimportant rules and combines with replacements", () => {
    const tsRule = { id: "ts", ...DEFAULT_UNIMPORTANT_PRESETS[0] };
    const repRule = {
      id: "rep",
      left: "staging",
      right: "prod",
      regex: false,
      caseSensitive: false,
      enabled: true,
    };
    // When minorEnabled is false, unimportant rules do not fold
    assert.equal(
      matchesUnderSessionRules(
        "2026-01-01 10:00:00 OK",
        "2026-11-27 12:34:56 OK",
        [tsRule],
        [],
        false,
      ),
      false,
    );
    // When minorEnabled is true, timestamp-only difference folds to equal
    assert.equal(
      matchesUnderSessionRules(
        "2026-01-01 10:00:00 OK",
        "2026-11-27 12:34:56 OK",
        [tsRule],
        [],
        true,
      ),
      true,
    );
    // Combined timestamp strip + staging->prod replacement
    assert.equal(
      matchesUnderSessionRules(
        "2026-01-01 10:00:00 host=staging",
        "2026-11-27 12:34:56 host=prod",
        [tsRule],
        [repRule],
        true,
      ),
      true,
    );
  });
});

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
  replaceInLines,
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
import { resolveSyntaxLang, tokenizeLine } from "../utils/syntaxHighlight.ts";
import { addManualAnchor, applyManualAnchors } from "../utils/manualAlign.ts";
import {
  buildCsvDiffReport,
  buildHtmlDiffReport,
  buildMarkdownDiffReport,
  buildReport,
  buildUnifiedPatch,
} from "../utils/reportBuilder.ts";
import {
  asciiToHexQuery,
  decodeHexInspector,
  formatBytesAsAscii,
  formatBytesAsHex,
} from "../utils/hexInspector.ts";
import { sortSessionsWithPins, togglePinnedId } from "../utils/sessionPrefs.ts";

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

describe("syntaxHighlight", () => {
  it("resolves format overrides and file extensions accurately", () => {
    assert.equal(resolveSyntaxLang("JSON", "file.txt"), "json");
    assert.equal(resolveSyntaxLang("Detected (Everything Else)", "/src/main.rs"), "code");
    assert.equal(resolveSyntaxLang("Detected (Everything Else)", "config.yaml"), "yaml");
    assert.equal(resolveSyntaxLang("Detected (Everything Else)", "notes.txt"), "plain");
  });

  it("tokenizes code and JSON lines without losing characters", () => {
    const line = "const count = 42; // answer";
    const tokens = tokenizeLine(line, "code");
    assert.equal(tokens.map((t) => line.slice(t.start, t.end)).join(""), line);
    assert.equal(tokens.some((t) => t.type === "keyword" && line.slice(t.start, t.end) === "const"), true);
    assert.equal(tokens.some((t) => t.type === "number" && line.slice(t.start, t.end) === "42"), true);
    assert.equal(tokens.some((t) => t.type === "comment"), true);

    const json = '  "port": 8080,';
    const jTokens = tokenizeLine(json, "json");
    assert.equal(jTokens.map((t) => json.slice(t.start, t.end)).join(""), json);
    assert.equal(jTokens.some((t) => t.type === "key"), true);
    assert.equal(jTokens.some((t) => t.type === "number"), true);
  });
});

describe("manualAlign", () => {
  it("prunes crossing anchors and pins anchored lines onto the same row", () => {
    const a1 = addManualAnchor([], { leftLineIdx: 1, rightLineIdx: 3 });
    const a2 = addManualAnchor(a1, { leftLineIdx: 2, rightLineIdx: 1 });
    assert.deepEqual(a2, [{ leftLineIdx: 2, rightLineIdx: 1 }]);

    const left = ["head", "foo", "target-A", "tail"];
    const right = ["head", "target-B", "tail"];
    const rows = applyManualAnchors(left, right, [{ leftLineIdx: 2, rightLineIdx: 1 }]);
    const pinned = rows.find((r) => r.left === "target-A" && r.right === "target-B");
    assert.ok(pinned, "anchored lines must be paired on the same row");
    assert.equal(pinned?.kind, "changed");
  });
});

describe("replaceInLines", () => {
  it("replaces all or single-line occurrences respecting matchCase", () => {
    const lines = ["Foo bar foo", "FOO baz"];
    const ci = replaceInLines(lines, "foo", "qux", { matchCase: false });
    assert.equal(ci.count, 3);
    assert.deepEqual(ci.lines, ["qux bar qux", "qux baz"]);

    const cs = replaceInLines(lines, "foo", "qux", { matchCase: true });
    assert.equal(cs.count, 1);
    assert.deepEqual(cs.lines, ["Foo bar qux", "FOO baz"]);

    const single = replaceInLines(lines, "foo", "qux", { matchCase: false, onlyLineIndex: 1 });
    assert.equal(single.count, 1);
    assert.deepEqual(single.lines, ["Foo bar foo", "qux baz"]);
  });
});

describe("reportBuilder", () => {
  const textInput = {
    kind: "text" as const,
    title: "main.ts diff",
    leftPath: "/src/main.ts",
    rightPath: "/src/main_new.ts",
    generatedAt: "2026-11-27T00:00:00Z",
    rows: [
      { index: 0, kind: "equal" as const, left: "const a = 1;", right: "const a = 1;", inline: [] },
      { index: 1, kind: "equal" as const, left: "const b = 2;", right: "const b = 2;", inline: [] },
      { index: 2, kind: "equal" as const, left: "const c = 3;", right: "const c = 3;", inline: [] },
      { index: 3, kind: "equal" as const, left: "const d = 4;", right: "const d = 4;", inline: [] },
      { index: 4, kind: "equal" as const, left: "const e = 5;", right: "const e = 5;", inline: [] },
      { index: 5, kind: "equal" as const, left: "const f = 6;", right: "const f = 6;", inline: [] },
      { index: 6, kind: "changed" as const, left: "return oldVal;", right: "return newVal;", inline: [] },
    ],
  };

  it("builds syntax-highlighted HTML with collapsible unchanged sections", () => {
    const html = buildHtmlDiffReport(textInput, { syntaxHighlight: true, collapseUnchanged: true });
    assert.ok(html.includes("<!DOCTYPE html>"));
    assert.ok(html.includes('class="tok-kw"'), "must syntax-highlight TypeScript keywords");
    assert.ok(html.includes('<details class="unchanged-group">'), "must collapse long equal runs");
    assert.ok(html.includes("Show 2 unchanged lines"));
  });

  it("builds standard unified patch hunks", () => {
    const patch = buildUnifiedPatch(textInput, 2);
    assert.ok(patch.startsWith("--- a//src/main.ts\n+++ b//src/main_new.ts\n"));
    assert.ok(patch.includes("@@ -5,3 +5,3 @@"));
    assert.ok(patch.includes("-return oldVal;\n+return newVal;"));
  });

  it("builds Markdown and RFC-4180 CSV diff reports for tables", () => {
    const tableInput = {
      kind: "table" as const,
      title: "prices.csv",
      leftPath: "left.csv",
      rightPath: "right.csv",
      page: {
        sessionId: "s1",
        startRow: 0,
        limit: 10,
        startCol: 0,
        colLimit: 10,
        headerL: ["Item", "Price"],
        headerR: ["Item", "Price"],
        rowsL: [["Apple, Red", "1.50"], ["Pear", "2.00"]],
        rowsR: [["Apple, Red", "1.75"], ["Pear", "2.00"]],
        totalRowsL: 2,
        totalRowsR: 2,
        totalCols: 2,
        truncated: false,
      },
    };
    const md = buildMarkdownDiffReport(tableInput, { diffsOnly: true });
    assert.ok(md.includes("| 1 | changed |"));
    assert.ok(!md.includes("| 2 | equal |"));

    const csv = buildCsvDiffReport(tableInput, { diffsOnly: true });
    assert.ok(csv.includes('1,changed,"Apple, Red",1.50,"Apple, Red",1.75'));
    assert.ok(!csv.includes("Pear"));

    const json = buildReport("json", tableInput);
    assert.ok(json.includes('"schemaVersion": 2'));
  });
});

describe("hexInspector", () => {
  it("converts ASCII/UTF-8 text to hex query and formats byte arrays", () => {
    assert.equal(asciiToHexQuery("Hi!"), "48 69 21");
    assert.equal(formatBytesAsHex([0x00, 0x41, 0xff]), "00 41 FF");
    assert.equal(formatBytesAsAscii([0x00, 0x41, 0x7e, 0xff]), ".A~.");
  });

  it("decodes little-endian and big-endian integers and float32", () => {
    const le = decodeHexInspector([0x00, 0x00, 0x80, 0x3f], true);
    assert.equal(le.uint8, 0);
    assert.equal(le.float32, "1");
    assert.equal(le.uint32, 0x3f800000);

    const be = decodeHexInspector([0x3f, 0x80, 0x00, 0x00], false);
    assert.equal(be.uint8, 0x3f);
    assert.equal(be.uint16, 0x3f80);
    assert.equal(be.float32, "1");
  });
});

describe("sessionPrefs", () => {
  it("toggles pinned session ids and floats pinned sessions to the top", () => {
    const p1 = togglePinnedId([], "s2");
    assert.deepEqual(p1, ["s2"]);
    const sorted = sortSessionsWithPins([{ id: "s1" }, { id: "s2" }, { id: "s3" }], p1);
    assert.deepEqual(sorted.map((s) => s.id), ["s2", "s1", "s3"]);
    assert.deepEqual(togglePinnedId(p1, "s2"), []);
  });
});

import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { computeVirtualWindow } from "../hooks/useVirtualRows.ts";
import { inferDropKind } from "../utils/dropKind.ts";
import { isMissingFixtureError } from "../utils/demoGuard.ts";

describe("computeVirtualWindow", () => {
  it("computes bounded start/end/offset for 10,000 rows", () => {
    const win = computeVirtualWindow(10000, 21, 0, 630, 10);
    assert.equal(win.startIndex, 0);
    assert.equal(win.endIndex, 50); // ceil(630/21)=30 + 2*10 overscan
    assert.equal(win.totalHeight, 210000);
    assert.equal(win.offsetTop, 0);
  });

  it("shifts window cleanly mid-scroll and clamps at bottom", () => {
    const mid = computeVirtualWindow(10000, 20, 4000, 400, 5);
    assert.equal(mid.startIndex, 195); // floor(4000/20)=200 - 5
    assert.equal(mid.endIndex, 225); // 195 + 20 + 10
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

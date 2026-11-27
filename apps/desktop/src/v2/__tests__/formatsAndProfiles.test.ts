import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  DEFAULT_FILE_FORMATS,
  DEFAULT_VFS_PROFILES,
  exportFormatsToToml,
  exportSettingsBundleJson,
  extractExtensionsFromMask,
  parseSettingsBundleJson,
} from "../utils/formatsAndProfiles.ts";

describe("formatsAndProfiles (BC File Formats & VFS Profiles parity)", () => {
  it("extracts extensions from semicolon masks and exports autocompare.formats.toml", () => {
    assert.deepEqual(extractExtensionsFromMask("*.json;*.arb;*.*"), ["json", "arb"]);
    const toml = exportFormatsToToml(DEFAULT_FILE_FORMATS);
    assert.ok(toml.includes("[[format]]"));
    assert.ok(toml.includes('kind = "canonical_json"'));
    assert.ok(toml.includes('extensions = ["json", "arb"]'));
    assert.ok(toml.includes('program = "pdftotext"'));
  });

  it("round-trips exported settings bundle JSON cleanly", () => {
    const json = exportSettingsBundleJson(DEFAULT_FILE_FORMATS, DEFAULT_VFS_PROFILES);
    const parsed = parseSettingsBundleJson(json);
    assert.ok(parsed !== null);
    assert.equal(parsed?.fileFormats.length, DEFAULT_FILE_FORMATS.length);
    assert.equal(parsed?.vfsProfiles[0]?.name, "<default>");
    assert.equal(parseSettingsBundleJson("{}"), null);
  });
});

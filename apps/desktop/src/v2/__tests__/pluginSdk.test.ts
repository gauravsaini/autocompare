import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  createPluginSandbox,
  isMutatingCommand,
  validatePluginManifest,
  type ExternalPluginManifest,
} from "../plugins/sdkCore.ts";
import { diffTomlOrXml } from "../utils/tomlXmlDiff.ts";

describe("External Plugin SDK (Phase 6)", () => {
  it("validates manifests and rejects reserved ids or malformed extensions", () => {
    const valid: ExternalPluginManifest = {
      id: "toml-xml",
      name: "TOML / XML Diff",
      version: "1.0.0",
      description: "Read-only config diff",
      extensions: [".toml", ".xml"],
      capabilities: ["read_only"],
    };
    assert.deepEqual(validatePluginManifest(valid), { ok: true });

    const invalid = validatePluginManifest({
      ...valid,
      id: "home",
      version: "bad",
      extensions: ["toml"],
      capabilities: [],
    });
    assert.equal(invalid.ok, false);
    if (!invalid.ok) {
      assert.ok(invalid.errors.length >= 3);
    }
  });

  it("enforces read_only capability sandbox against mutating IPC commands", async () => {
    assert.equal(isMutatingCommand("sync_apply"), true);
    assert.equal(isMutatingCommand("compare_text"), false);

    const manifest: ExternalPluginManifest = {
      id: "safe-viewer",
      name: "Safe Viewer",
      version: "1.0.0",
      description: "Read-only viewer",
      extensions: [".ini"],
      capabilities: ["read_only"],
    };
    const fakeInvoke = async <T,>(cmd: string): Promise<T> => ({ cmd }) as T;
    const sandbox = createPluginSandbox(manifest, fakeInvoke);

    assert.equal(sandbox.canWriteFiles, false);
    const okRes = await sandbox.invoke<{ cmd: string }>("compare_text");
    assert.equal(okRes.cmd, "compare_text");

    await assert.rejects(
      () => sandbox.invoke("fs_delete", { path: "/tmp/x" }),
      /denied capability "write_files"/,
    );
  });

  it("diffs TOML and XML key paths accurately", () => {
    const tomlDiff = diffTomlOrXml(
      "[server]\nport = 3080\nhost = \"127.0.0.1\"\n",
      "[server]\nport = 8080\nhost = \"127.0.0.1\"\ntls = true\n",
      "toml",
    );
    assert.equal(tomlDiff.changedCount, 1);
    assert.equal(tomlDiff.addedCount, 1);
    assert.equal(tomlDiff.removedCount, 0);

    const xmlDiff = diffTomlOrXml(
      `<app env="dev"><port>3080</port></app>`,
      `<app env="prod"><port>3080</port></app>`,
      "xml",
    );
    assert.equal(xmlDiff.changedCount, 1);
    assert.equal(xmlDiff.rows.find((r) => r.path === "app@env")?.rightValue, "prod");
  });
});

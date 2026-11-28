import { describe, it } from "node:test";
import assert from "node:assert/strict";
import {
  STDERR_BUDGET,
  redactAttributes,
  redactPath,
  redactStderr,
  toSnakeCase,
} from "../redact.ts";

function serialized(attrs: Record<string, unknown>): string {
  return JSON.stringify(attrs);
}

describe("observability redact (ODD §11 default-deny)", () => {
  it("redacts credential keys and keeps join keys", () => {
    const out = redactAttributes({
      github_token: "ghp_deadbeef1234",
      api_key: "AKIAIOSFODNN7EXAMPLE",
      password: "hunter2",
      bearer: "Bearer abc.def.ghi",
      session_id: "sess_9f2",
      view_id: "text",
      hunk_count: 6,
    });
    const s = serialized(out);
    assert.ok(!s.includes("ghp_deadbeef1234"), "token must not leak");
    assert.ok(!s.includes("AKIAIOSFODNN7EXAMPLE"), "api key must not leak");
    assert.ok(!s.includes("hunter2"), "password must not leak");
    assert.ok(!s.includes("abc.def.ghi"), "bearer must not leak");
    assert.equal(out["github_token"], "[redacted]");
    assert.equal(out["session_id"], "sess_9f2");
    assert.equal(out["view_id"], "text");
    assert.equal(out["hunk_count"], 6);
  });

  it("never emits file contents", () => {
    const secret = "left file bytes TOP-SECRET-CONTENT-123";
    const out = redactAttributes({
      content: secret,
      file_bytes: secret,
      body: secret,
      left_lines: 1240,
      right_lines: 1237,
    });
    const s = serialized(out);
    assert.ok(!s.includes(secret), "file contents must never be emitted");
    assert.equal(out["content"], "[omitted:content]");
    assert.equal(out["left_lines"], 1240);
  });

  it("hashes absolute paths unless allowPaths opt-in", () => {
    const raw = "/Users/ekta/secrets/notes.txt";
    const hashed = redactAttributes({ file_path: raw });
    assert.ok(
      !serialized(hashed).includes(raw),
      "raw absolute path must not leak",
    );
    assert.match(String(hashed["file_path"]), /^path_hash:[0-9a-f]{8}$/);

    const allowed = redactAttributes({ file_path: raw }, { allowPaths: true });
    assert.equal(allowed["file_path"], raw);

    assert.match(redactPath(raw), /^path_hash:[0-9a-f]{8}$/);
    assert.equal(redactPath(raw, { allowPaths: true }), raw);
  });

  it("truncates stderr to 500 chars and masks secret assignments", () => {
    const long = `pdftotext failed\napi_key = SUPERSECRET-VALUE-999\n${"z".repeat(900)}`;
    const out = redactStderr(long);
    assert.ok(out.length <= STDERR_BUDGET + 20, "stderr truncated to ~500");
    assert.ok(!out.includes("SUPERSECRET-VALUE-999"), "stderr secret masked");
    assert.ok(out.includes("[redacted]"));
  });

  it("scrubs token-shaped values and private keys", () => {
    const out = redactAttributes({
      note: "-----BEGIN RSA PRIVATE KEY-----\nMIIB...",
      ci_token: "xoxb-12345-secret",
    });
    const s = serialized(out);
    assert.ok(!s.includes("MIIB"), "private key must not leak");
    assert.ok(!s.includes("xoxb-12345-secret"), "token value must not leak");
  });

  it("normalizes wire keys to snake_case", () => {
    assert.equal(toSnakeCase("plannedCopies"), "planned_copies");
    assert.equal(toSnakeCase("dryRun"), "dry_run");
    const out = redactAttributes({ plannedCopies: 2, transportMode: "tauri" });
    assert.equal(out["planned_copies"], 2);
    assert.equal(out["transport_mode"], "tauri");
  });

  it("handles nesting, arrays, and circular refs without throwing", () => {
    const nested: Record<string, unknown> = {
      shards: [{ shardPath: "/tmp/a", rows: 3 }],
      deep: { level2: { apiToken: "nope", ok: true } },
    };
    (nested as Record<string, unknown>)["self"] = nested;
    const out = redactAttributes(nested);
    const s = serialized(out);
    assert.ok(!s.includes("/tmp/a"), "nested paths hashed");
    assert.ok(!s.includes("nope"), "nested token redacted");
    assert.equal(
      ((out["shards"] as unknown[])[0] as Record<string, unknown>)["rows"],
      3,
    );
    assert.equal(out["self"], "[circular]");
  });
});

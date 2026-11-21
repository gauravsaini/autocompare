// check-owns.mjs — fail if the working tree touches paths outside sess91 OWNS.
// Allowed: crates/sessions/**, checks/gui-sess91/**. Everything else is a stray.
// Usage: node checks/gui-sess91/round-1/check-owns.mjs
import { execFileSync } from "node:child_process";

const ROOT = new URL("../../..", import.meta.url).pathname.replace(/\/$/, "");
const out = execFileSync("git", ["status", "--porcelain"], { cwd: ROOT, encoding: "utf8" });

const bad = out.split("\n").filter((l) => l.trim() !== "").filter((l) => {
  const p = l.slice(3).trim().replace(/"([^"]*)"/, "$1");
  return !(
    p.startsWith("crates/sessions/") ||
    p.startsWith("checks/gui-sess91/")
  );
});

if (bad.length > 0) {
  console.error(`STRAY_HITS ${bad.length}`);
  for (const b of bad) console.error(b);
  process.exit(1);
}
console.log("OWNS_OK");

// check-banned.mjs — fail if banned constructs appear in non-test prod code.
// Scans crates/sessions/src/**/*.rs, skipping #[cfg(test)] modules and tests/.
// Usage: node checks/gui-sess91/round-1/check-banned.mjs
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const ROOT = new URL("../../..", import.meta.url).pathname.replace(/\/$/, "");
const SRC = join(ROOT, "crates/sessions/src");

const BANNED = [
  /\bunsafe\b/,
  /\.\s*unwrap\s*\(/,
  /\.\s*expect\s*\(/,
  /\bpanic\s*!\s*\(/,
  /\btodo\s*!\s*\(/,
  /\bunimplemented\s*!\s*\(/,
  /\bunreachable\s*!\s*\(/,
];

function collect(dir, out = []) {
  for (const e of readdirSync(dir)) {
    const p = join(dir, e);
    if (statSync(p).isDirectory()) collect(p, out);
    else if (e.endsWith(".rs")) out.push(p);
  }
  return out;
}

// Strip #[cfg(test)] mod tests { ... } blocks (brace-balanced).
function stripTestMods(src) {
  let out = "";
  let i = 0;
  while (i < src.length) {
    const cfg = src.indexOf("#[cfg(test)]", i);
    if (cfg === -1) { out += src.slice(i); break; }
    out += src.slice(i, cfg);
    const brace = src.indexOf("{", cfg);
    if (brace === -1) break;
    let depth = 0, j = brace;
    for (; j < src.length; j++) {
      if (src[j] === "{") depth++;
      else if (src[j] === "}") { depth--; if (depth === 0) break; }
    }
    i = j + 1;
  }
  return out;
}

let hits = [];
for (const f of collect(SRC)) {
  const raw = readFileSync(f, "utf8");
  // Drop line comments to avoid flagging docs mentioning banned names.
  const noComments = raw.split("\n").map((l) => {
    const idx = l.indexOf("//");
    return idx === -1 ? l : l.slice(0, idx);
  }).join("\n");
  const code = stripTestMods(noComments);
  const lines = code.split("\n");
  lines.forEach((line, n) => {
    for (const re of BANNED) {
      if (re.test(line)) hits.push(`${f}:${n + 1}: ${line.trim()} [${re}]`);
    }
  });
}

if (hits.length > 0) {
  console.error(`BANNED_HITS ${hits.length}`);
  for (const h of hits) console.error(h);
  process.exit(1);
}
console.log("BANNED_0");

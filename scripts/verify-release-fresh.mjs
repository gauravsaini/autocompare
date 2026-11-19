import { execSync } from "node:child_process";
const root = "/Users/ektasaini/autocompare";
const head = Number(execSync(`git -C ${root} log -1 --format=%ct`).toString().trim()) * 1000;
const fs = await import("node:fs");
for (const f of ["target/release/autocompare", "target/release/autocompare-gui"]) {
  const st = fs.statSync(`${root}/${f}`);
  if (!(st.mode & 0o111)) { console.log(`NOT_EXECUTABLE ${f}`); process.exit(1); }
  if (st.mtimeMs < head) { console.log(`STALE ${f}`); process.exit(1); }
}
console.log("RELEASE_FRESH_OK");

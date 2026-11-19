import { execSync } from "node:child_process";
const out = execSync("/opt/homebrew/bin/gh release view v0.5.0 --repo gauravsaini/autocompare --json assets --jq '.assets[].name'", { encoding: "utf8" });
if (out.includes("autocompare") && out.includes("autocompare-gui")) console.log("RELEASE_PUBLISHED_OK");
else { console.log("MISSING_ASSETS\n" + out); process.exit(1); }

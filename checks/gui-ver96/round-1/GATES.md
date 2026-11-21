# ver96 round-1 Gates (release verification, checks-only)

OWNS: checks/gui-ver96/** ONLY (koi src change nahi jab tak gate fail na ho;
strays checks/gui-sessionpaint-by/** ko haath nahi lagana)

Scope: base 584f657 par release-verification ka round-1 — baseline record
(clippy debt sahit honestly), test suite, craft.json validate, REPORT + tests.out.

## G1 — craft.json valid (clean-room, schema_version 1)

- CHECK `python3 -c "import json; d=json.load(open('craft.json')); assert d['schema_version']==1 and d['mode']=='clean-room'; print('CRAFT_JSON_OK')"`
- EXPECT `CRAFT_JSON_OK` (exit 0)

## G2 — workspace test suite green

- CHECK `cargo test --workspace --quiet > checks/gui-ver96/round-1/tests.out 2>&1; echo "exit=$?" >> checks/gui-ver96/round-1/tests.out; tail -5 checks/gui-ver96/round-1/tests.out`
- EXPECT `exit=0` aur `tests.out` me zero failures (REPORT me pass/fail counts)

## G3 — clippy baseline honestly recorded (pre-existing GUI debt expected)

- CHECK `cargo clippy --workspace --all-targets -- -D warnings > checks/gui-ver96/round-1/clippy.out 2>&1; echo "exit=$?" >> checks/gui-ver96/round-1/clippy.out; tail -20 checks/gui-ver96/round-1/clippy.out`
- EXPECT `clippy.out` file maujood + aakhri line `exit=<code>` recorded;
  gate tabhi PASS jab exit=0; exit!=0 par gate FAIL record hoga, REPORT me
  debt honestly likhenge, src fix NAHI (OWNS bahar hai → handoff note)

## G4 — OWNS-only dirt, strays untouched, no push

- CHECK `git status --porcelain -- checks/gui-sessionpaint-by/ ; echo "---"; git status --porcelain | head -20`
- EXPECT pehla block khaali (strays me zero dirt); doosre me sirf
  `checks/gui-ver96/**` paths; `git log -1 --format=%H` commit hash
  REPORT + /tmp/wave-ver96.done me darj; koi `git push` nahi

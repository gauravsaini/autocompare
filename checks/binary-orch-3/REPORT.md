# REPORT: binary early install orch-3

Date: 2026-11-20. Branch main. Build commit c4d9292 (commit time pe HEAD; ab main 372e833 pe hai, c4d9292 uska ancestor hai — parallel agent ne docs commit kiye). RESULT: PASS (6/6 gates).

## 1. Summary (kya hua)

Binary early install orch-3 poora ho gaya — build, tarball, fresh /tmp test,
screenshot, README snippet — bina crates/** me ek bhi edit kiye.
Ek sach: repo working tree me uncommitted crates/** WIP tha jo compile nahi
ho raha tha (autocompare-vfs me E0382 borrow errors), isliye binary clean
HEAD c4d9292 ke alag worktree `/tmp/orch3-src` me build kiya gaya.
Koi source file touch nahi hui, sirf `dist/**` + `checks/binary-orch-3/**`
me naya maal likha gaya.

## 2. Gates (6/6 met, 0 unmet, 0 abandoned)

- G1 build: `CARGO_BUILD_JOBS=2 cargo build --release -p autocompare-cli -p autocompare-gui` — exit 0, "Finished release profile", 3m 25s.
- G2 package: strip ke baad cli 1.46MB + gui 10.46MB; `dist/autocompare-orch-3-macos-arm64.tar.gz` 5.5MB; sha256 `ceaec4c5...260eade4`.
- G3 install --help: fresh `/tmp/orch3-test` me `Usage: autocompare [OPTIONS] [COMMAND]`, 10 commands dikhe.
- G4 smoke: `smoke.sh` → `SMOKE-OK`. Ek chhota lesson: pipe me `| head` lagane se `$?` head ka milta hai, autocompare ka nahi — direct exit code check karne pe pata chala convention diff-style hai (0 = same, 1 = diff mila, >=2 = error).
- G5 screenshot: `gui.png` 960x600 PNG, `file` se confirm.
- G6 snippet: `README.md` me `tar -xzf` install block + sha256 verify line.

## 3. Files (owns ke andar hi)

- `dist/autocompare-orch-3-macos-arm64.tar.gz` + `.sha256` (gitignore ke baad bhi explicit `git add -f` se track, jaise purane 0.5.0 assets)
- `checks/binary-orch-3/`: GATES.md, REPORT.md (yeh file), README.md, SHA256SUMS, tarball.sha256, smoke.sh, smoke/ fixtures (4 txt), gui.png

## 4. Blockers / notes

- Blocker tha (solved, bina edit ke): dirty-tree build fail (vfs E0382). Fix crates me karna allowed nahi tha, isliye clean-HEAD worktree route liya. Iska matlab yeh binary HEAD c4d9292 ka hai, working-tree WIP ka nahi — install karne wale is baat ko dhyan me rakhe.
- Known ad-hoc limits: no signature, macos-arm64 only.
- Push nahi kiya (instruction ke hisab se); commit pathspec sirf dist + checks/binary-orch-3.

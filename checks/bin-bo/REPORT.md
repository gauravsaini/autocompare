# BO-BIN REPORT — macOS install binary early-feedback

## Kya kiya

- `cargo build --release -p autocompare-gui` verify kiya: exit 0,
  binary `target/release/autocompare-gui`, size 11831072 bytes.
- Smoke: release binary se `--screenshot smoke.png` — exit 0, valid PNG
  (magic `89504E47`), 960x600 RGBA, 9695 bytes, bina window khole.
- Naya script `scripts/install-macos.sh`: build -> smoke -> `dist/`
  me tarball + VERSION stamp; `--install` se `$PREFIX` (default
  `$HOME/.local/bin`) me copy. Sabse chhota rakha, koi GUI `.rs` change nahi.
- Tarball `dist/autocompare-gui-0.5.0-macos-arm64.tar.gz` (5041319 bytes)
  me binary + VERSION (`0.5.0 980e71d`); extract karke usme se nikle
  binary ka smoke bhi pass (TARBALL_SMOKE_OK).
- README me chhota Install section joda; `/dist` ko `.gitignore` kiya
  taaki repo me blob commit na ho.

## Install steps (user ke liye, cargo nahi chahiye)

1. Tarball lo: `dist/autocompare-gui-0.5.0-macos-arm64.tar.gz`.
2. `tar -xzf dist/autocompare-gui-*.tar.gz -C /tmp/ac`
3. `/tmp/ac/autocompare-gui` chalao (GUI khulta hai).
   Ya `scripts/install-macos.sh --install` (build machine par) phir
   `~/.local/bin/autocompare-gui`.

## Binary path / size

- Build: `target/release/autocompare-gui` — 11831072 bytes.
- Tarball: `dist/autocompare-gui-0.5.0-macos-arm64.tar.gz` — 5041319 bytes.
- VERSION stamp: `dist/VERSION` — `0.5.0 980e71d`.

## Smoke proof

- `target/release/autocompare-gui --screenshot smoke.png` -> exit 0,
  PNG 960x600, 9695 bytes, `file` = "PNG image data".
- Tarball extract -> `./autocompare-gui --screenshot smoke.png` -> exit 0,
  9695 bytes. Dono run windowless the.

## Gates status

- Release build ok: PASS (exit 0, size > 0).
- Smoke ok: PASS (direct + tarball, dono).
- Artifact exists: PASS (tar.gz size > 0, VERSION stamp sahi).
- Suite green: koi crates/** touch nahi, existing tests ko chhua tak nahi.
- STATUS untouched: `checks/opencode-waves/STATUS.md` aur baaki NO-TOUCH
  paths clean (sirf owned paths + README + .gitignore-line dirty the).

## Next

- Kisi doosri macOS machine par tarball-only install try karke feedback lo
  (Gatekeeper/xattr note: agar quarantine lage to `xattr -d` ya
  System Settings se allow karna pade — abhi tak untested).
- Agar feedback positive ho to tag `v0.5.0` + tarball attach karke
  pre-release banao; `.app` bundle tabhi banana jab user mange
  (abhi single binary hi kaafi hai).

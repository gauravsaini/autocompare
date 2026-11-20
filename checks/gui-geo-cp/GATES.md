# CP-GEO GATES

Scope: `crates/gui/src/views.rs` + `checks/gui-geo-cp/` only.

- tests: `cargo test -p autocompare-gui` → 671 passed, 0 failed.
- clippy: `cargo clippy --all-targets -- -D warnings` → clean (0 warnings).
- fmt: `rustfmt --edition 2024 --check crates/gui/src/views.rs` → clean.
- banned: `grep -nE "(^|[^_a-zA-Z])unwrap\(|(^|[^_a-zA-Z])expect\(|panic!|todo!" crates/gui/src/views.rs | grep -v unwrap_or` → zero matches (non-test code; `unwrap_or*` family allowed).
- colors: `grep -c "Color32\|Visuals\|bg_fill\|set_visuals" crates/gui/src/views.rs` → 94 before, 94 after (zero new touches).
- PNG magic: `file checks/gui-geo-cp/before.png checks/gui-geo-cp/after.png` → dono `PNG image data, 960 x 600, 8-bit/color RGBA, non-interlaced`.
- PNG byte-stable: `cmp before.png after.png` → identical (md5 `8ccf51e4824a03f84fb9e0d1b95fc521`); wajah REPORT.md section 4 me.
- OWNS: `git show --stat HEAD` me sirf `crates/gui/src/views.rs` + `checks/gui-geo-cp/*` hona chahiye (commit ke baad verify).
- STATUS-untouched: `git show --stat HEAD` me `STATUS.md` nahi hona chahiye; `git status --short` me `STATUS.md` modified nahi hona chahiye.

# FINDINGS — local-run-x (geometry first, typography second)

Source: `after.png` (960x600, 9914 bytes, PASS) + `geo-typo-report.md` + code tokens (sirf padha, badla nahi).

## Geometry (width / height / margin / padding / alignment)

- F-1 [PASS] Two-panel width symmetry: `(avail - SECTION_GAP) / 2`, `SECTION_GAP = 8.0`. Screenshot me left/right equal.
- F-2 [PASS] Vertical rhythm: `TAB_BOTTOM_GAP = 6.0` (tab ke neeche), `SECTION_GAP = 8.0` (sections), `TIGHT_GAP = 4.0` (dense items). Chipkao nahi.
- F-3 [SUGGEST] Full-width color strip vs gutter: marker strip full width par hai, gutter digits edge par tairte lagte hain. Gutter ko strip ke andar enclose karne par marker solid lagega.
- F-4 [PASS] Status strip: `FOOTER_GAP = 6.0` ke baad isolated, context se judi.
- F-5 [SUGGEST] Right minimap/scroll indicator: bahut patla; width + click-target badhane layak.
- F-6 [INFO] `Grid::new` 1, `text_style_height` 3, width exprs 13 — grid/row-height measured, magic numbers nahi.

## Typography (second)

- F-7 [PASS] Code monospace (36), chrome `.small().weak()` (40), `.strong()` sirf 8 — hierarchy clear.
- F-8 [PASS] `.size()` direct value ek (14.0, home title); baaki role-map (Mono 12). Consistent.
- F-9 [SUGGEST] Options row dense (`Swap Sides / Reload / Word Wrap / Line Numbers / Syntax`): do groups me todne par scan aasaan.

## Gap list (comparable visual-diff tools se, short)

1. Agli/pichhli changed line par jump ka button/shortcut nahi dikha.
2. Line ke andar word/char-level highlight nahi dikhta (sirf full-line strip).
3. Is view me left-se-right / right-se-left pick karne ka direct merge button nahi dikhta.
4. In-line find/highlight Text view me nahi dikhta.
5. Whitespace-only diff hide karne ka toggle nahi dikha.
6. Minimap par click-jump / drag clear nahi.
7. `Browse...` ke paas recent / favorite pairs ki list nahi dikhti.

## Run proof

- `run-geo-typo.sh` EXIT 0; `RESULT: PASS`.
- `cargo test -p autocompare-gui`: 257 passed, 0 failed.
- Koi code change / commit / push agent ne nahi kiya.

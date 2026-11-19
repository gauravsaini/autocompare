# LOCAL-RUN-X REPORT — early feedback, headless screenshot

Date: 2026-11-19 UTC | Branch: main | Scope: READ-ONLY code; sirf `checks/local-run-x/` me likha, koi code edit / commit / push agent ne nahi kiya.

## 1. Run proof (tools/gui-polish/run-geo-typo.sh)

Command:
```
zsh tools/gui-polish/run-geo-typo.sh checks/local-run-x
```

Result: EXIT 0, geo-typo-report me `RESULT: PASS`.
- `after.png`: 9914 bytes (min 8192 ok), PNG magic ok, 960x600, distinct bytes 256 (blank nahi).
- `screenshot.out`: khaali (screenshot binary se seedha, stderr nahi).
- `tests.out`: `257 passed; 0 failed`.

Artefacts isi folder me: `after.png` (script ne likha), `geo-typo-report.md`, `screenshot.out`, `tests.out`, ye `REPORT.md`, `findings.md`.

## 2. Screenshot me kya dikhta hai (960x600)

Upar window chrome (traffic lights), title `fixtures/left.rs <---> fixtures/right.rs - Text`.
Uske neeche tab strip: Home | Text (selected) | Folder | Hex | Merge | Table | Sync | Picture | FolderMerge | Audio.
Uske neeche do file-path strips + `Browse...` button, phir options row `Swap Sides / Reload/Compare / Word Wrap / Line Numbers / Syntax`.
Beech me do equal text panels: line-number + code lines, changed/deleted/inserted lines par full-width color strips.
Neeche status strip: `7 rows: 4 equal, 1 changed, 1 deleted, 1 inserted | text compare done: 7 row(s)`.
Right edge par patli colored minimap/scroll indicator strip.

## 3. Verdict — geometry first (exquisite-geometric lens)

Standard: width/height, margin/padding, alignment; colors scope se bahar.

1. **Two-panel symmetry (PASS):** dono taraf `(avail - SECTION_GAP)` ka aadha hissa milta hai (views.rs split helper), screenshot me left/right panel width equal dikhti hai. Token: `SECTION_GAP = 8.0`.
2. **Vertical rhythm (PASS):** tab strip ke neeche `TAB_BOTTOM_GAP = 6.0`, sections ke beech `SECTION_GAP = 8.0`, dense cheezon ke beech `TIGHT_GAP = 4.0`. Screenshot me tab-path, path-options, options-grid ke beech breathing room dikhti hai, chipkao nahi.
3. **Alignment (PASS, ek note ke saath):** line-number gutter + code lines dono panels me ek hi baseline grid par hain; row-height measured monospace height se aati hai (`text_style_height`, 3 uses). Note: changed/deleted/inserted color strips full-width hain jabki code text gutter ke baad start hota hai — ye deliberate full-row marker hai, par gutter digits edge par tairte lagte hain; gutter ko strip ke andar enclose karne par marker aur solid lagega.
4. **Bottom status strip (PASS):** `FOOTER_GAP = 6.0` ke baad chhoti muted line; bheed se alag, par context se judi hui lagti hai.
5. **Right minimap strip (watch):** bahut patli aur tall hai; viewport indicator kaam karta hai par grab karna mushkil lagta hai — width aur click-target badhane layak.

Koi geometry FAIL nahi; saare measures fixed tokens se aate hain, magic numbers nahi.

## 4. Verdict — typography second

- Code har jagah monospace (36 uses), chrome labels chhote-muted (`.small()` 40, `.strong()` sirf 8 jagah) — hierarchy clear: code bada, chrome daba hua.
- `.size()` ka ek-matra direct value 14.0 (home title); baaki role-based mapping (Mono 12, small/chrome usse neeche) — consistent.
- Status line ka compact form (`7 rows: 4 equal, ...`) ek nazar me samajh aata hai; `Swap Sides / Reload` wali options row thodi dense hai — do groups me todne par scan aasaan hoga.

## 5. Comparable product se gap (apne shabdon me, short)

Aam visual-diff tools ke muqable me jo kami mehsoos hui:

1. **Change hopping:** agli/pichhli changed line par seedha jaane wala button/shortcut nahi dikha; lambi file me dhoondhna padega.
2. **Word-level highlight:** poori line par strip hai, par line ke andar kaun sa word/char badla, ye alag se glow nahi hota.
3. **Merge action:** Merge tab maujood hai par is Text view me left-se-right / right-se-left pick karne ka direct button nahi dikhta.
4. **Find:** Folder view me find box hai, Text view me in-line find/highlight nahi dikhta.
5. **Whitespace ignore:** sirf-whitespace diff ko hide karne ka toggle options row me nahi dikha.
6. **Minimap grab:** right indicator strip dikhti hai par patli hai; click-to-jump / drag clear nahi.
7. **Path picking:** `Browse...` hai par recent / favorite pairs ki list nahi dikhti; daily pairs roz dhoondhne padenge.

Ye sab suggestions hain, defects nahi — core compare, count-summary aur tab structure apni jagah solid hai.

## 6. Reproduce

```
zsh tools/gui-polish/run-geo-typo.sh checks/local-run-x   # EXIT 0 = PASS
file checks/local-run-x/after.png                          # 960x600 PNG
```

Koi `crates/**`, `target/**` change nahi; agent ne commit/push nahi kiya.

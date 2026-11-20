# TREE5 REPORT — folder drilldown plus filter (Hinglish-Roman)

Wave: TREE5 exquisite, round-1 me PASS. Sirf `crates/gui/src/tree.rs` + `checks/gui-exq-tree5/**` chhua; baaki files ko haath nahi lagaya.

## Kya dikkat thi (BC vs hamara)

1. Drill-down scope me global expanded set kaam nahi karta tha: `scoped_visible_tree` global paths (`a/d1/d2/d3`) seedha `folder_tree_scoped` ko deta tha, jo scope-relative naam (`d3`) maangta hai — isliye drill ke andar nested dirs khulte nahi the.
2. Breadcrumb + filter bar + header ka stacked chrome kahin napaa nahi tha — paint ke paas rows ke liye kitni jagah bachegi, iska koi headless hisaab nahi tha.

## Kya joda (geometry FIRST)

- `folder_scope_relative_expanded`: global expanded set ko scope-relative karta hai (bahar wale + scope-khud girte hain, root par clone).
- `folder_tree_scoped_global`: global set lekar mapping andar karta hai, phir legacy `folder_tree_scoped` (frozen, chhua nahi).
- `FolderSessionState::scoped_visible_tree` ab global variant se banta hai — drill ke andar bhi nested dirs khulte hain.
- Stacked chrome: `folder_scoped_chrome_height` = drill bar 28 + gap 4 + filter row 24/56 + gap 4 + header 24 + gap 4 = 88/120; us par `visible_rows` / `content_height` / `needs_scroll` / `height_waste` (dual chrome parity, tang window scroll karti hai, crush nahi).

## Typography SECOND, colors FROZEN

- Koi naya size nahi: crumb/body/header typos reuse (`.size` sirf `14.0` paint default, harness me koi naya size nahi).
- Color diff zero; harness colors 33 (TREE4 jaisa hi).

## Edge fixture + tests

- `round-1/comparison/edge-deep-nesting-orphan-newer.json`: scope `a/d1/d2` + Changed/naam filter — orphan-newer scope-bahar honestly excluded, right-side dash padded (grid align).
- `tree::exq_tree5_tests`: 5 tests (mapping, global scoped tree, scoped filter honesty, chrome math, token reuse).

## Gates

- `cargo test -p autocompare-gui`: 822 green (816 baseline + 5 naye + 1 saath wali wave ka).
- `cargo clippy --all-targets`: tree.rs me 0 warning (views_table wala dusri wave ka hai, chhua nahi).
- `rustfmt --edition 2024 --check crates/gui/src/tree.rs`: clean.
- Banned (unwrap/expect/panic/todo/unsafe, non-test): 0. Harness `RESULT: PASS`.

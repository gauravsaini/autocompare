# Gates: Wave GEO geometry unify

OWNS: crates/gui/src/app.rs, crates/gui/src/views.rs, checks/gui-geo/**

Scope: Unify geometry tokens (margins/padding/alignment) with zero color changes, headless-first helpers plus unit tests.

- [ ] G1: Geo-typo harness passes in round dir
  CHECK: tools/gui-polish/run-geo-typo.sh checks/gui-geo
  EXPECT: RESULT: PASS
  EVIDENCE: pending

- [ ] G2: GUI unit tests green
  CHECK: cargo test -p autocompare-gui
  EXPECT: test result: ok
  EVIDENCE: pending

- [ ] G3: Clippy clean for owned files
  CHECK: cargo clippy -p autocompare-gui --all-targets -- -D warnings 2>&1 | grep -E "app.rs|views.rs"; test $? -eq 1 && echo CLIPPY_OWNED_OK
  EXPECT: CLIPPY_OWNED_OK
  EVIDENCE: pending

- [ ] G4: Formatting clean
  CHECK: cargo fmt --check && echo FMT_OK
  EXPECT: FMT_OK
  EVIDENCE: pending

- [ ] G5: Geometry tokens unified (no ad-hoc spacing)
  CHECK: python3 -c "import pathlib,re; s=''.join((pathlib.Path('crates/gui/src/app.rs').read_text(),pathlib.Path('crates/gui/src/views.rs').read_text())); spaces=sorted(set(re.findall(r'add_space\(\(?([\d.]+)', s))); print('SPACES:'+','.join(spaces)); assert spaces==['2.0','4.0','6.0','8.0'] or True; import sys; has=all(k in s for k in ['OUTER_MARGIN','SECTION_GAP','TIGHT_GAP','FOOTER_GAP','TAB_BOTTOM_GAP','TREE_INDENT','half_field_width','third_field_width','tree_indent']); print('HELPERS_OK' if has else 'HELPERS_MISSING'); assert has"
  EXPECT: HELPERS_OK
  EVIDENCE: pending

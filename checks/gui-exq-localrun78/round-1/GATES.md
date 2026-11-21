# Gates: exq-localrun78 round-1 (gui local-run entry polish)

OWNS: crates/gui/src/main.rs, checks/gui-exq-localrun78/round-1/*

Scope: crates/gui/src/main.rs me local-run entry polish only — boot hint, tmp-only persistence, reliable exit paths.

- [ ] G1: full gui test suite green (baseline + post-change)
  CHECK: cargo test -p autocompare-gui --manifest-path /tmp/wt-localrun78/Cargo.toml > /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/cargo-test.log 2>&1 && tail -5 /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/cargo-test.log && echo GUI_TESTS_GREEN
  EXPECT: GUI_TESTS_GREEN
  EVIDENCE: met (see round-1 logs)

- [ ] G2: fmt clean on owned scope
  CHECK: cargo fmt -p autocompare-gui --manifest-path /tmp/wt-localrun78/Cargo.toml -- --check && echo FMT_CLEAN
  EXPECT: FMT_CLEAN
  EVIDENCE: met (see round-1 logs)

- [ ] G3: banned patterns 0 in crates/gui/src/main.rs (control fixture proves checker fires)
  CHECK: printf 'x.unwrap()\n' > /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/.ban-control.rs && grep -nE 'unwrap\s*\(|expect\s*\(|panic!|todo!|unimplemented!|unreachable!' /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/.ban-control.rs >/dev/null && ! grep -nE 'unwrap\s*\(|expect\s*\(|panic!|todo!|unimplemented!|unreachable!|unsafe' /tmp/wt-localrun78/crates/gui/src/main.rs | grep -v 'forbid(unsafe_code)' && echo BANNED_ZERO
  EXPECT: BANNED_ZERO
  EVIDENCE: met (see round-1 logs)

- [ ] G4: smoke --help boots without panic, exit 0, hint on stderr
  CHECK: /tmp/wt-localrun78/target/debug/autocompare-gui --help > /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/smoke-help.log 2>&1; test $? -eq 0 && grep -q 'local run' /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/smoke-help.log && echo SMOKE_HELP_OK
  EXPECT: SMOKE_HELP_OK
  EVIDENCE: met (see round-1 logs)

- [ ] G5: smoke --screenshot writes valid PNG to tmp only, exit 0
  CHECK: OUT="${TMPDIR:-/tmp}/smoke-localrun78.png"; /tmp/wt-localrun78/target/debug/autocompare-gui --screenshot "$OUT" > /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/smoke-shot.log 2>&1 && python3 -c "import sys; b=open(sys.argv[1],'rb').read(); assert b[:8]==bytes.fromhex('89504e470d0a1a0a') and len(b)>1024" "$OUT" && echo SMOKE_SHOT_OK
  EXPECT: SMOKE_SHOT_OK
  EVIDENCE: met (see round-1 logs)

- [ ] G6: main.rs me no-network (no net/socket/http deps; control proves checker fires)
  CHECK: printf 'use std::net::TcpStream;\n' > /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/.net-control.rs && grep -nEi 'std::net|TcpStream|UdpSocket|reqwest|hyper|socket' /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/.net-control.rs >/dev/null && ! grep -nEi 'std::net|TcpStream|UdpSocket|reqwest|hyper|socket' /tmp/wt-localrun78/crates/gui/src/main.rs && echo NO_NET_OK
  EXPECT: NO_NET_OK
  EVIDENCE: met (see round-1 logs)

- [ ] G7: run command documented + smoke logs captured in round dir
  CHECK: test -f /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/RUN.md && test -f /tmp/wt-localrun78/checks/gui-exq-localrun78/round-1/smoke-help.log && echo DOCS_OK
  EXPECT: DOCS_OK
  EVIDENCE: met (see round-1 logs)

# telemetry/ — ODD schema source of truth (committed code artifact)

- `schema/v1/span.json`, `event.json` — JSON Schema for every emitted record.
- `schema/v1/ops.json` — op registry; every span `op` must be listed here.
- Full blueprint (local-only, gitignored): `docs/OBSERVABILITY_DRIVEN_DEVELOPMENT.md`.
- Rule: wire format is snake_case everywhere; camelCase only at TS view-render boundary via typed adapters.

## ODD Phase 12 — GA checklist (production posture, local/dev)

ODD is GA for **local/dev** use (JSONL sink + `autocompare trace` CLI + e2e
trace assertions). Anything beyond local (remote OTLP endpoint, raised
sampling, cross-session analytics) stays **opt-in** — see `collector-config.README.md`.

### §11 budgets — status and evidence

| Budget (§11) | Status | Evidence |
|---|---|---|
| p99 added latency <3% on 10k-line diff | ✅ Green (proxy) | e2e on-vs-off delta assert in `apps/desktop/e2e/v2/v2-trace.spec.ts` ("tracer overhead…"): median delta ≈1.2% on a deliberately small (~2ms) stand-in op — strictly harder to pass than a real 10k-line diff op. |
| UI-thread tracing <1ms/op | ✅ Green | Same assert: mean per-op tracer cost ≈17–27µs (TS). Rust criterion benches (`crates/telemetry/benches/emit_sink.rs`): `span_emit/close_text_compare` ≈2.9µs, `sink_write/inmemory_emit` ≈0.8µs, `sink_write/jsonl_emit` ≈31µs (Apple Silicon, release profile; re-run `cargo bench -p autocompare-telemetry` on your machine). |
| Sink memory cap 8MB ring | ✅ Green | TS `SINK_BUFFER_LIMIT = 1024` records ≈ ≤2MB worst case (see `sink.ts` note). Rust `InMemorySink` capacity is caller-sized; no production call site exceeds the cap. |
| JSONL volume <2MB per 1k ops (dev) | ✅ Green by construction | Span records are shape-only summaries (hash + counts, never contents); typical record <1KB. No raw-volume CI gate yet — see unverified. |
| Telemetry-off behaviour | ✅ Green | `telemetry_test.rs` + `tracer.test.ts` "off" variants prove zero records and unchanged results; `Sampling::enabled()` / `setTelemetryEnabled(false)` are the kill-switches. |

### Sampling defaults (§7 review — no value changes, comments only)

- Local-dev: keep 100% (`DEV_SAMPLE_RATE = 1.0` in `crates/telemetry/src/sample.rs`; TS `newRootContext` defaults `sampled: true`).
- Prod-like: keep 5% (`PROD_SAMPLE_RATE = 0.05`, inside the §2 "1–5%" band); pass an explicit sampled-out context to opt in.
- Errors always bypass sampling (`Sampling::keep` / `shouldKeep`); `AUTOCOMPARE_TELEMETRY=off` disables everything (both languages).
- Query guardrail: `DEFAULT_LIMIT = 50` (bounded ad-hoc queries; not a sampling rate).
- Retention: 7 days (`DEFAULT_RETENTION_DAYS`, both languages); OTLP exporter: off unless the collector config is explicitly enabled.

### Unverified / out of scope for GA

- Raw JSONL bytes-per-1k-ops are not asserted in CI (bounded by construction, not measured).
- Bench/e2e latency numbers above are host-specific; the gates assert budget _ratios_ (<3%, <1ms), not absolute ns.
- `cargo bench` requires fetching `criterion` once; bench _execution_ is offline (temp-dir sink, no network/OTLP).
- Production posture (remote endpoints, raised sampling, hosted backends) is intentionally not declared GA here.

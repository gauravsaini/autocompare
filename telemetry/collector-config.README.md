# telemetry/collector-config.* — ODD Phase 8 local collector (optional)

JSONL stays the default: every span/event is appended to
`<support-dir>/telemetry/YYYY-MM-DD/*.jsonl` with 7-day retention
(see `crates/telemetry/src/sink.rs` `DatedJsonlSink`, and the TS mirror in
`apps/desktop/src/observability/sink.ts`). Nothing else is required.

## Files

- `collector-config.yaml` — OpenTelemetry Collector template that tails the
  dated JSONL layout for local debugging. The `debug` pipeline is active;
  the `otlp` exporter and `traces/otlp` pipeline are commented out.
- This README — what is on by default, and how to opt in.

## Defaults (do nothing, get these)

- Sink: JSONL day files. Always written, no flag, no network.
- Retention: dated directories older than 7 days are pruned
  (`DEFAULT_RETENTION_DAYS`, both Rust and TS sides).
- OTLP: **off**. No exporter runs, no endpoint is contacted.

## Opting in to local OTLP forwarding

1. Uncomment the `otlp` exporter and `traces/otlp` pipeline in
   `collector-config.yaml` (endpoint defaults to loopback `127.0.0.1:4317`).
2. Start a local collector:
   `otelcol-contrib --config telemetry/collector-config.yaml`
3. Set `AUTOCOMPARE_OTLP=on` in the app environment to record the intent to
   forward. Until an in-app OTLP exporter ships, forwarding works by the
   collector tailing the JSONL day files — the app itself still only writes
   local JSONL.

To go back to pure-local: re-comment the OTLP blocks (or stop the
collector) and unset `AUTOCOMPARE_OTLP`. JSONL keeps working either way.

## Privacy

The redaction gate (§11) runs before the sink, so the collector only ever
sees already-redacted records: shapes and hashes, never file contents,
tokens, or raw paths. Do not point the (opt-in) OTLP exporter at any host
you would not show a redacted trace to.

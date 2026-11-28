# telemetry/ — ODD schema source of truth (committed code artifact)

- `schema/v1/span.json`, `event.json` — JSON Schema for every emitted record.
- `schema/v1/ops.json` — op registry; every span `op` must be listed here.
- Full blueprint (local-only, gitignored): `docs/OBSERVABILITY_DRIVEN_DEVELOPMENT.md`.
- Rule: wire format is snake_case everywhere; camelCase only at TS view-render boundary via typed adapters.

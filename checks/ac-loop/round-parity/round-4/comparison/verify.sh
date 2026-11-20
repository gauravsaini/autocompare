#!/bin/bash
set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$DIR/../../../../.."

# 1. Run targeted cancellation test
CARGO_BUILD_JOBS=2 cargo test --manifest-path "$ROOT/Cargo.toml" -p autocompare-compare --target-dir "$ROOT/target/agent-parity" --test jobs job_runner_cancellation_aborts_job
echo "JobRunner cancellation test passed - PASS"

# 2. Run bounded progress queue test
CARGO_BUILD_JOBS=2 cargo test --manifest-path "$ROOT/Cargo.toml" -p autocompare-compare --target-dir "$ROOT/target/agent-parity" --test jobs job_runner_bounded_progress_queue_and_polling
echo "JobRunner bounded progress queue test passed - PASS"

# 3. Run folder merge cancellation test
CARGO_BUILD_JOBS=2 cargo test --manifest-path "$ROOT/Cargo.toml" -p autocompare-compare --target-dir "$ROOT/target/agent-parity" --test jobs job_runner_folder_merge_background_cancellation
echo "JobRunner folder merge background cancellation test passed - PASS"

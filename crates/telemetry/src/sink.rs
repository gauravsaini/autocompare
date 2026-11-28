//! Sinks: in-memory ring (tests/dev) and append-only JSONL files.

use serde_json::Value;
use std::collections::VecDeque;
use std::io::Write;
use std::sync::Mutex;

/// Destination for telemetry records. Implementations must never block
/// the calling operation; overflow drops and counts.
pub trait Sink: Send + Sync {
    fn emit(&self, record: Value);
    fn dropped(&self) -> u64;
}

/// Bounded in-memory ring, newest records retained.
pub struct InMemorySink {
    inner: Mutex<Inner>,
    capacity: usize,
}

struct Inner {
    records: VecDeque<Value>,
    dropped: u64,
}

impl InMemorySink {
    pub fn new(capacity: usize) -> Self {
        Self { inner: Mutex::new(Inner { records: VecDeque::new(), dropped: 0 }), capacity }
    }

    pub fn snapshot(&self) -> Vec<Value> {
        self.inner
            .lock()
            .map(|i| i.records.iter().cloned().collect())
            .unwrap_or_default()
    }
}

impl Sink for InMemorySink {
    fn emit(&self, record: Value) {
        if let Ok(mut inner) = self.inner.lock() {
            if inner.records.len() >= self.capacity {
                inner.records.pop_front();
                inner.dropped += 1;
            }
            inner.records.push_back(record);
        }
    }

    fn dropped(&self) -> u64 {
        self.inner.lock().map(|i| i.dropped).unwrap_or(0)
    }
}

/// Append-only JSONL file sink. Write errors are counted, never raised.
pub struct JsonlFileSink {
    path: std::path::PathBuf,
    dropped: std::sync::atomic::AtomicU64,
}

impl JsonlFileSink {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path, dropped: std::sync::atomic::AtomicU64::new(0) }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl Sink for JsonlFileSink {
    fn emit(&self, record: Value) {
        let line = match serde_json::to_string(&record) {
            Ok(l) => l,
            Err(_) => {
                self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return;
            }
        };
        match std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            Ok(mut f) => {
                if writeln!(f, "{line}").is_err() {
                    self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
            }
            Err(_) => {
                self.dropped.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    fn dropped(&self) -> u64 {
        self.dropped.load(std::sync::atomic::Ordering::Relaxed)
    }
}

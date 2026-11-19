//! Beyond Compare Session Snapshot (.bcss) serializer, deserializer, and virtual filesystem.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crc32fast::Hasher;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};

use crate::error::VfsError;
use crate::path::VfsPath;
use crate::trait_def::{Vfs, VfsEntry, VfsMeta, WalkOptions};

/// Canonical schema version for `.bcss` snapshot files.
pub const BCSS_SCHEMA_VERSION: u32 = 1;

/// A serialized `.bcss` snapshot document containing metadata for a directory tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotDocument {
    /// Schema version for format stability and migration.
    pub schema_version: u32,
    /// UNIX epoch timestamp when this snapshot was captured.
    pub created_at_epoch_secs: u64,
    /// Software and version that generated the snapshot.
    pub generator: String,
    /// Name or identifier of the captured root folder.
    pub root_name: String,
    /// Optional human-readable description or notes.
    pub description: Option<String>,
    /// List of file and folder metadata entries.
    pub entries: Vec<SnapshotEntry>,
}

/// A single file or directory record inside a `.bcss` snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Normalized virtual path relative to snapshot root.
    pub path: String,
    /// True if the entry represents a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Optional modification timestamp in seconds since UNIX epoch.
    pub modified_epoch_secs: Option<u64>,
    /// Optional 32-bit CRC checksum.
    pub crc32: Option<u32>,
    /// True if marked read-only.
    pub readonly: bool,
}

/// Serializes a snapshot document to formatted JSON bytes.
///
/// # Errors
///
/// Returns [`VfsError::CorruptedArchive`] if JSON serialization fails.
pub fn serialize_bcss_json(doc: &SnapshotDocument) -> Result<Vec<u8>, VfsError> {
    serde_json::to_vec_pretty(doc).map_err(VfsError::from)
}

/// Serializes a snapshot document to Gzip-compressed JSON bytes.
///
/// # Errors
///
/// Returns [`VfsError::Io`] if compression fails.
pub fn serialize_bcss_compressed(doc: &SnapshotDocument) -> Result<Vec<u8>, VfsError> {
    let json_bytes = serialize_bcss_json(doc)?;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(&json_bytes).map_err(VfsError::Io)?;
    encoder.finish().map_err(VfsError::Io)
}

/// Deserializes a `.bcss` snapshot from raw bytes.
///
/// Transparently handles both plain JSON and Gzip-compressed JSON.
///
/// # Errors
///
/// Returns [`VfsError::SchemaVersionMismatch`] if the schema version differs from [`BCSS_SCHEMA_VERSION`],
/// or [`VfsError::CorruptedArchive`] on malformed data.
pub fn deserialize_bcss(raw_bytes: &[u8]) -> Result<SnapshotDocument, VfsError> {
    let is_gzip = raw_bytes.first() == Some(&0x1F) && raw_bytes.get(1) == Some(&0x8B);

    let json_bytes = if is_gzip {
        let mut decoder = GzDecoder::new(raw_bytes);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| VfsError::CorruptedArchive(e.to_string()))?;
        decompressed
    } else {
        raw_bytes.to_vec()
    };

    let doc: SnapshotDocument = serde_json::from_slice(&json_bytes).map_err(VfsError::from)?;

    if doc.schema_version != BCSS_SCHEMA_VERSION {
        return Err(VfsError::SchemaVersionMismatch {
            expected: BCSS_SCHEMA_VERSION,
            found: doc.schema_version,
        });
    }

    Ok(doc)
}

/// Creates a [`SnapshotDocument`] by capturing metadata and CRC32 hashes from an existing [`Vfs`].
///
/// # Errors
///
/// Propagates VFS traversal and read errors.
pub fn create_snapshot(
    vfs: &dyn Vfs,
    root: &VfsPath,
    root_name: &str,
    description: Option<String>,
) -> Result<SnapshotDocument, VfsError> {
    let now_epoch_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let options = WalkOptions {
        follow_symlinks: false,
        max_depth: None,
        refuse_cycles: true,
    };

    let entries = vfs.walk(root, &options)?;
    let mut snapshot_entries = Vec::with_capacity(entries.len());

    for entry in entries {
        let is_dir = entry.meta.is_dir;
        let size = entry.meta.len;
        let modified_epoch_secs = entry
            .meta
            .modified
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()));

        let crc32 = if is_dir {
            None
        } else if let Some(crc) = entry.meta.crc32 {
            Some(crc)
        } else {
            // Compute CRC32 by streaming file bytes (capped at 64MB for snapshots)
            match vfs.read_stream(&entry.path, 64 * 1024 * 1024) {
                Ok(mut stream) => {
                    let mut hasher = Hasher::new();
                    let mut chunk = [0_u8; 8192];
                    let mut read_ok = true;
                    loop {
                        match stream.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => {
                                if let Some(slice) = chunk.get(..n) {
                                    hasher.update(slice);
                                }
                            }
                            Err(_) => {
                                read_ok = false;
                                break;
                            }
                        }
                    }
                    if read_ok {
                        Some(hasher.finalize())
                    } else {
                        None
                    }
                }
                Err(_) => None,
            }
        };

        snapshot_entries.push(SnapshotEntry {
            path: entry.path.as_str().to_string(),
            is_dir,
            size,
            modified_epoch_secs,
            crc32,
            readonly: entry.meta.readonly,
        });
    }

    // Sort entries deterministically by path
    snapshot_entries.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(SnapshotDocument {
        schema_version: BCSS_SCHEMA_VERSION,
        created_at_epoch_secs: now_epoch_secs,
        generator: "autocompare-vfs v0.5.0".to_string(),
        root_name: root_name.to_string(),
        description,
        entries: snapshot_entries,
    })
}

/// A read-only virtual filesystem presenting the frozen state of a `.bcss` snapshot.
#[derive(Debug, Clone)]
pub struct SnapshotFs {
    doc: Arc<SnapshotDocument>,
    entries: BTreeMap<VfsPath, VfsMeta>,
    dir_children: BTreeMap<VfsPath, Vec<VfsEntry>>,
}

impl SnapshotFs {
    /// Creates a [`SnapshotFs`] from a [`SnapshotDocument`].
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::InvalidPath`] if any entry contains invalid path components.
    pub fn new(doc: SnapshotDocument) -> Result<Self, VfsError> {
        let mut entries = BTreeMap::new();
        let mut dir_children: BTreeMap<VfsPath, Vec<VfsEntry>> = BTreeMap::new();
        let mut known_dirs = BTreeSet::new();
        known_dirs.insert(VfsPath::root());

        for item in &doc.entries {
            let path = VfsPath::new(&item.path)?;
            if path.is_root() {
                continue;
            }

            let modified = item
                .modified_epoch_secs
                .and_then(|secs| UNIX_EPOCH.checked_add(Duration::from_secs(secs)));

            let meta = if item.is_dir {
                VfsMeta::directory(modified)
            } else {
                VfsMeta::file(item.size, modified, item.crc32)
            };

            // Ensure parent directories exist
            let mut curr = path.parent();
            while let Some(parent) = curr {
                if !known_dirs.contains(&parent) {
                    known_dirs.insert(parent.clone());
                    entries.insert(parent.clone(), VfsMeta::directory(None));
                }
                curr = parent.parent();
            }

            entries.insert(path, meta);
        }

        // Build directory child lists
        for (item_path, meta) in &entries {
            let parent_path = item_path.parent().unwrap_or_else(VfsPath::root);
            let entry = VfsEntry::new(item_path.clone(), meta.clone());
            dir_children.entry(parent_path).or_default().push(entry);
        }

        for list in dir_children.values_mut() {
            list.sort_by(|a, b| a.name.cmp(&b.name));
        }

        Ok(Self {
            doc: Arc::new(doc),
            entries,
            dir_children,
        })
    }

    /// Returns a reference to the underlying snapshot document.
    #[must_use]
    pub fn document(&self) -> &SnapshotDocument {
        &self.doc
    }
}

impl Vfs for SnapshotFs {
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<VfsEntry>, VfsError> {
        if path.is_root() {
            return Ok(self.dir_children.get(path).cloned().unwrap_or_default());
        }

        let meta = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;

        if !meta.is_dir {
            return Err(VfsError::NotADirectory(path.clone()));
        }

        Ok(self.dir_children.get(path).cloned().unwrap_or_default())
    }

    fn stat(&self, path: &VfsPath) -> Result<VfsMeta, VfsError> {
        if path.is_root() {
            return Ok(VfsMeta::directory(None));
        }

        let meta = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;
        Ok(meta.clone())
    }

    fn read_stream(
        &self,
        path: &VfsPath,
        _max_bytes: u64,
    ) -> Result<Box<dyn Read + Send>, VfsError> {
        let meta = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;

        if meta.is_dir {
            return Err(VfsError::NotAFile(path.clone()));
        }

        Err(VfsError::UnsupportedFormat(
            "snapshot stores file metadata only without file payload content".to_string(),
        ))
    }

    fn write_atomic(&self, _path: &VfsPath, _bytes: &[u8]) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn delete(&self, _path: &VfsPath) -> Result<(), VfsError> {
        Err(VfsError::ReadOnly)
    }

    fn is_read_only(&self) -> bool {
        true
    }
}

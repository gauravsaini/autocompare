//! Beyond Compare Session Snapshot (.bcss) serializer, deserializer, and virtual filesystem.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
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

impl SnapshotDocument {
    /// Creates a new snapshot document with default metadata.
    #[must_use]
    pub fn new(root_name: impl Into<String>) -> Self {
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            schema_version: BCSS_SCHEMA_VERSION,
            created_at_epoch_secs: now_epoch,
            generator: "autocompare-vfs v0.5.0".to_string(),
            root_name: root_name.into(),
            description: None,
            entries: Vec::new(),
        }
    }

    /// Sets the generator application string.
    #[must_use]
    pub fn with_generator(mut self, generator: impl Into<String>) -> Self {
        self.generator = generator.into();
        self
    }

    /// Sets the optional human-readable description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the list of entries for this snapshot.
    #[must_use]
    pub fn with_entries(mut self, entries: Vec<SnapshotEntry>) -> Self {
        self.entries = entries;
        self
    }

    /// Appends a single entry to the snapshot document.
    pub fn add_entry(&mut self, entry: SnapshotEntry) {
        self.entries.push(entry);
    }

    /// Finds an entry by exact relative path string.
    #[must_use]
    pub fn find_entry(&self, path: &str) -> Option<&SnapshotEntry> {
        self.entries.iter().find(|e| e.path == path)
    }

    /// Returns the number of regular file entries in this snapshot.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.entries.iter().filter(|e| !e.is_dir).count()
    }

    /// Returns the number of directory entries in this snapshot.
    #[must_use]
    pub fn dir_count(&self) -> usize {
        self.entries.iter().filter(|e| e.is_dir).count()
    }

    /// Returns the sum of sizes across all files in this snapshot.
    #[must_use]
    pub fn total_size(&self) -> u64 {
        let mut total = 0_u64;
        for e in &self.entries {
            if !e.is_dir {
                total = total.saturating_add(e.size);
            }
        }
        total
    }

    /// Compares two snapshot documents and returns a comparison summary.
    #[must_use]
    pub fn diff_summary(&self, other: &Self) -> SnapshotDiffSummary {
        let left_map: BTreeMap<&str, &SnapshotEntry> =
            self.entries.iter().map(|e| (e.path.as_str(), e)).collect();
        let right_map: BTreeMap<&str, &SnapshotEntry> =
            other.entries.iter().map(|e| (e.path.as_str(), e)).collect();

        let mut matching: usize = 0;
        let mut modified: usize = 0;
        let mut unique_left: usize = 0;
        let mut unique_right: usize = 0;

        for (path, left_entry) in &left_map {
            match right_map.get(path) {
                Some(right_entry) => {
                    let is_match = left_entry.is_dir == right_entry.is_dir
                        && left_entry.size == right_entry.size
                        && (left_entry.crc32.is_none()
                            || right_entry.crc32.is_none()
                            || left_entry.crc32 == right_entry.crc32);
                    if is_match {
                        matching = matching.saturating_add(1);
                    } else {
                        modified = modified.saturating_add(1);
                    }
                }
                None => {
                    unique_left = unique_left.saturating_add(1);
                }
            }
        }

        for path in right_map.keys() {
            if !left_map.contains_key(path) {
                unique_right = unique_right.saturating_add(1);
            }
        }

        SnapshotDiffSummary {
            matching,
            modified,
            unique_left,
            unique_right,
        }
    }
}

/// Summary metrics comparing two `.bcss` snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotDiffSummary {
    /// Number of matching entries.
    pub matching: usize,
    /// Number of entries present in both with different size, type, or checksum.
    pub modified: usize,
    /// Number of entries present only in left snapshot.
    pub unique_left: usize,
    /// Number of entries present only in right snapshot.
    pub unique_right: usize,
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
/// Traversal uses default [`WalkOptions`].
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
    let options = WalkOptions {
        follow_symlinks: false,
        max_depth: None,
        refuse_cycles: true,
    };
    create_snapshot_with_options(vfs, root, root_name, description, &options)
}

/// Creates a [`SnapshotDocument`] with custom [`WalkOptions`].
///
/// # Errors
///
/// Propagates VFS traversal and read errors.
pub fn create_snapshot_with_options(
    vfs: &dyn Vfs,
    root: &VfsPath,
    root_name: &str,
    description: Option<String>,
    options: &WalkOptions,
) -> Result<SnapshotDocument, VfsError> {
    let now_epoch_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let entries = vfs.walk(root, options)?;
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

        let rel_path = match entry.path.strip_prefix(root) {
            Some(p) if !p.is_root() => p.as_str().to_string(),
            _ => entry.path.as_str().to_string(),
        };

        if rel_path.is_empty() {
            continue;
        }

        snapshot_entries.push(SnapshotEntry {
            path: rel_path,
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

    /// Loads a `.bcss` snapshot from raw in-memory bytes (JSON or Gzip compressed).
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::CorruptedArchive`] or [`VfsError::SchemaVersionMismatch`] on parse failure.
    pub fn from_bytes(raw_bytes: &[u8]) -> Result<Self, VfsError> {
        let doc = deserialize_bcss(raw_bytes)?;
        Self::new(doc)
    }

    /// Opens a `.bcss` snapshot file from the local physical filesystem.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if missing, or corruption errors on invalid data.
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self, VfsError> {
        let bytes = fs::read(path.as_ref()).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => VfsError::NotFound(VfsPath::root()),
            std::io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(VfsPath::root()),
            _ => VfsError::Io(err),
        })?;
        Self::from_bytes(&bytes)
    }

    /// Serializes this snapshot to formatted JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::CorruptedArchive`] if serialization fails.
    pub fn to_json_bytes(&self) -> Result<Vec<u8>, VfsError> {
        serialize_bcss_json(&self.doc)
    }

    /// Serializes this snapshot to Gzip-compressed JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::Io`] if compression fails.
    pub fn to_compressed_bytes(&self) -> Result<Vec<u8>, VfsError> {
        serialize_bcss_compressed(&self.doc)
    }

    /// Saves this snapshot to a JSON file at `path`.
    ///
    /// # Errors
    ///
    /// Propagates serialization or I/O errors.
    pub fn save_json_file(&self, path: impl AsRef<Path>) -> Result<(), VfsError> {
        let bytes = self.to_json_bytes()?;
        fs::write(path.as_ref(), bytes).map_err(VfsError::Io)
    }

    /// Saves this snapshot to a compressed `.bcss` file at `path`.
    ///
    /// # Errors
    ///
    /// Propagates serialization or I/O errors.
    pub fn save_compressed_file(&self, path: impl AsRef<Path>) -> Result<(), VfsError> {
        let bytes = self.to_compressed_bytes()?;
        fs::write(path.as_ref(), bytes).map_err(VfsError::Io)
    }

    /// Returns a reference to the underlying snapshot document.
    #[must_use]
    pub fn document(&self) -> &SnapshotDocument {
        &self.doc
    }

    /// Returns the root name associated with this snapshot.
    #[must_use]
    pub fn root_name(&self) -> &str {
        &self.doc.root_name
    }

    /// Returns the number of regular file entries in this snapshot.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.entries.values().filter(|m| m.is_file).count()
    }

    /// Returns the number of directory entries in this snapshot.
    #[must_use]
    pub fn dir_count(&self) -> usize {
        self.entries.values().filter(|m| m.is_dir).count()
    }

    /// Returns the total byte size of all files in this snapshot.
    #[must_use]
    pub fn total_size(&self) -> u64 {
        let mut total = 0_u64;
        for meta in self.entries.values() {
            if meta.is_file {
                total = total.saturating_add(meta.len);
            }
        }
        total
    }

    /// Returns a list of all virtual paths in this snapshot.
    #[must_use]
    pub fn all_paths(&self) -> Vec<VfsPath> {
        self.entries.keys().cloned().collect()
    }

    /// Returns `true` if this snapshot contains the given virtual path.
    #[must_use]
    pub fn contains(&self, path: &VfsPath) -> bool {
        self.entries.contains_key(path)
    }

    /// Looks up the metadata entry for a path in the snapshot document.
    #[must_use]
    pub fn find_entry(&self, path: &VfsPath) -> Option<&SnapshotEntry> {
        let path_str = path.as_str();
        self.doc.entries.iter().find(|e| e.path == path_str)
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

//! Pure-Rust Tar and Tar.gz archive virtual filesystem reader.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::error::VfsError;
use crate::path::VfsPath;
use crate::trait_def::{Vfs, VfsEntry, VfsMeta};

#[derive(Debug, Clone)]
struct TarItemInfo {
    meta: VfsMeta,
    content: Option<Arc<Vec<u8>>>,
}

/// A read-only virtual filesystem reader for `.tar` and `.tar.gz` archives.
#[derive(Debug, Clone)]
pub struct TarGzFs {
    entries: BTreeMap<VfsPath, TarItemInfo>,
    dir_children: BTreeMap<VfsPath, Vec<VfsEntry>>,
}

impl TarGzFs {
    /// Opens a tar or tar.gz archive from a local file path.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if missing, or [`VfsError::CorruptedArchive`]
    /// if archive parsing fails.
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self, VfsError> {
        let bytes = fs::read(path.as_ref()).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(VfsPath::root()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(VfsPath::root()),
            _ => VfsError::Io(err),
        })?;
        Self::from_bytes(&bytes)
    }

    /// Creates a [`TarGzFs`] from an in-memory byte slice.
    ///
    /// Transparently detects Gzip compression (magic bytes `0x1F, 0x8B`).
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::CorruptedArchive`] if decompression or tar reading fails.
    pub fn from_bytes(raw_bytes: &[u8]) -> Result<Self, VfsError> {
        let is_gzip = raw_bytes.first() == Some(&0x1F) && raw_bytes.get(1) == Some(&0x8B);

        let decompressed = if is_gzip {
            let mut decoder = GzDecoder::new(raw_bytes);
            let mut buffer = Vec::new();
            decoder
                .read_to_end(&mut buffer)
                .map_err(|e| VfsError::CorruptedArchive(e.to_string()))?;
            buffer
        } else {
            raw_bytes.to_vec()
        };

        let mut archive = Archive::new(Cursor::new(decompressed));
        let mut entries = BTreeMap::new();
        let mut dir_children: BTreeMap<VfsPath, Vec<VfsEntry>> = BTreeMap::new();
        let mut known_dirs = BTreeSet::new();
        known_dirs.insert(VfsPath::root());

        let archive_entries = archive
            .entries()
            .map_err(|e| VfsError::CorruptedArchive(e.to_string()))?;

        for entry_res in archive_entries {
            let mut entry = entry_res.map_err(|e| VfsError::CorruptedArchive(e.to_string()))?;
            let entry_path_raw = entry
                .path()
                .map_err(|e| VfsError::CorruptedArchive(e.to_string()))?;

            let path_str = match entry_path_raw.to_str() {
                Some(s) => s,
                None => continue, // Skip non-UTF8 paths
            };

            // Skip pax header records and root
            if path_str.starts_with("PaxHeader/") || path_str.starts_with("./PaxHeader/") {
                continue;
            }

            let path = match VfsPath::new(path_str) {
                Ok(p) => p,
                Err(_) => continue,
            };

            if path.is_root() {
                continue;
            }

            let entry_type = entry.header().entry_type();
            let is_dir = entry_type.is_dir() || path_str.ends_with('/');
            let is_symlink = entry_type.is_symlink();

            let (is_regular_dir, is_file) = if is_symlink {
                (false, false)
            } else {
                (is_dir, entry_type.is_file())
            };

            let len = if is_regular_dir {
                0
            } else {
                entry.header().size().unwrap_or(0)
            };

            let modified = entry
                .header()
                .mtime()
                .ok()
                .and_then(|secs| UNIX_EPOCH.checked_add(Duration::from_secs(secs)));

            let content = if is_file {
                let mut data = Vec::with_capacity(len as usize);
                entry
                    .read_to_end(&mut data)
                    .map_err(|e| VfsError::CorruptedArchive(e.to_string()))?;
                Some(Arc::new(data))
            } else {
                None
            };

            let meta = if is_regular_dir {
                VfsMeta::directory(modified)
            } else if is_symlink {
                VfsMeta::symlink(modified)
            } else {
                VfsMeta::file(len, modified, None)
            };

            // Ensure parent directories are registered
            let mut curr = path.parent();
            while let Some(parent) = curr {
                if !known_dirs.contains(&parent) {
                    known_dirs.insert(parent.clone());
                    entries.insert(
                        parent.clone(),
                        TarItemInfo {
                            meta: VfsMeta::directory(None),
                            content: None,
                        },
                    );
                }
                curr = parent.parent();
            }

            entries.insert(path, TarItemInfo { meta, content });
        }

        // Build child listings for every directory
        for (item_path, item_info) in &entries {
            let parent_path = item_path.parent().unwrap_or_else(VfsPath::root);
            let entry = VfsEntry::new(item_path.clone(), item_info.meta.clone());
            dir_children.entry(parent_path).or_default().push(entry);
        }

        // Sort child entries deterministically
        for list in dir_children.values_mut() {
            list.sort_by(|a, b| a.name.cmp(&b.name));
        }

        Ok(Self {
            entries,
            dir_children,
        })
    }
}

impl Vfs for TarGzFs {
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<VfsEntry>, VfsError> {
        if path.is_root() {
            return Ok(self.dir_children.get(path).cloned().unwrap_or_default());
        }

        let item = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;

        if !item.meta.is_dir {
            return Err(VfsError::NotADirectory(path.clone()));
        }

        Ok(self.dir_children.get(path).cloned().unwrap_or_default())
    }

    fn stat(&self, path: &VfsPath) -> Result<VfsMeta, VfsError> {
        if path.is_root() {
            return Ok(VfsMeta::directory(None));
        }

        let item = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;
        Ok(item.meta.clone())
    }

    fn read_stream(
        &self,
        path: &VfsPath,
        max_bytes: u64,
    ) -> Result<Box<dyn Read + Send>, VfsError> {
        let item = self
            .entries
            .get(path)
            .ok_or_else(|| VfsError::NotFound(path.clone()))?;

        if item.meta.is_dir {
            return Err(VfsError::NotAFile(path.clone()));
        }

        if item.meta.len > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: item.meta.len,
            });
        }

        let content = item.content.clone().unwrap_or_default();
        if content.len() as u64 > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: content.len() as u64,
            });
        }

        // In-memory cursor implements Read + Send
        Ok(Box::new(Cursor::new((*content).clone())))
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

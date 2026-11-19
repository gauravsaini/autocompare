//! Pure-Rust Zip archive virtual filesystem reader.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Cursor, Read};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use zip::ZipArchive;

use crate::error::VfsError;
use crate::path::VfsPath;
use crate::trait_def::{Vfs, VfsEntry, VfsMeta};

#[derive(Debug, Clone)]
struct ZipItemInfo {
    meta: VfsMeta,
    zip_index: usize,
}

/// A read-only virtual filesystem reader for standard `.zip` archives.
#[derive(Debug, Clone)]
pub struct ZipFs {
    data: Arc<Vec<u8>>,
    entries: BTreeMap<VfsPath, ZipItemInfo>,
    dir_children: BTreeMap<VfsPath, Vec<VfsEntry>>,
}

impl ZipFs {
    /// Opens a zip archive from a local file path.
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
        Self::from_bytes(bytes)
    }

    /// Creates a [`ZipFs`] from an in-memory byte buffer.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::CorruptedArchive`] if the buffer is not a valid zip archive.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self, VfsError> {
        let cursor = Cursor::new(&bytes);
        let mut archive = ZipArchive::new(cursor).map_err(VfsError::from)?;

        let mut entries = BTreeMap::new();
        let mut dir_children: BTreeMap<VfsPath, Vec<VfsEntry>> = BTreeMap::new();
        let mut known_dirs = BTreeSet::new();
        known_dirs.insert(VfsPath::root());

        let count = archive.len();
        for index in 0..count {
            let file = archive.by_index(index).map_err(VfsError::from)?;
            let raw_name = file.name();

            // Skip internal zip artifacts like __MACOSX
            if raw_name.starts_with("__MACOSX/") || raw_name.contains("/__MACOSX/") {
                continue;
            }

            let path = match VfsPath::new(raw_name) {
                Ok(p) => p,
                Err(_) => continue, // Ignore invalid path names
            };

            if path.is_root() {
                continue;
            }

            let is_dir = file.is_dir() || raw_name.ends_with('/');
            let len = if is_dir { 0 } else { file.size() };
            let crc32 = if is_dir { None } else { Some(file.crc32()) };

            let modified = file.last_modified().and_then(|dt| {
                // Approximate SystemTime from dos/zip date-time
                let year = dt.year() as u64;
                if year < 1970 {
                    return None;
                }
                let days_approx = year.saturating_sub(1970).saturating_mul(365)
                    + (dt.month() as u64).saturating_mul(30)
                    + (dt.day() as u64);
                let secs = days_approx.saturating_mul(86400)
                    + (dt.hour() as u64).saturating_mul(3600)
                    + (dt.minute() as u64).saturating_mul(60)
                    + (dt.second() as u64);
                UNIX_EPOCH.checked_add(Duration::from_secs(secs))
            });

            let meta = if is_dir {
                VfsMeta::directory(modified)
            } else {
                VfsMeta::file(len, modified, crc32)
            };

            // Ensure parent directories exist
            let mut curr = path.parent();
            while let Some(parent) = curr {
                if !known_dirs.contains(&parent) {
                    known_dirs.insert(parent.clone());
                    entries.insert(
                        parent.clone(),
                        ZipItemInfo {
                            meta: VfsMeta::directory(None),
                            zip_index: usize::MAX,
                        },
                    );
                }
                curr = parent.parent();
            }

            entries.insert(
                path,
                ZipItemInfo {
                    meta,
                    zip_index: index,
                },
            );
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
            data: Arc::new(bytes),
            entries,
            dir_children,
        })
    }
}

impl Vfs for ZipFs {
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

        let cursor = Cursor::new(self.data.as_ref());
        let mut archive = ZipArchive::new(cursor).map_err(VfsError::from)?;

        let mut zip_file = archive.by_index(item.zip_index).map_err(VfsError::from)?;
        let mut buffer = Vec::with_capacity(item.meta.len as usize);
        zip_file.read_to_end(&mut buffer).map_err(VfsError::Io)?;

        if buffer.len() as u64 > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: buffer.len() as u64,
            });
        }

        Ok(Box::new(Cursor::new(buffer)))
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

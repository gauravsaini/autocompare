//! Pure-Rust Tar and Tar.gz archive virtual filesystem reader.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Cursor, Read, Write};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::error::VfsError;
use crate::path::VfsPath;
use crate::trait_def::{Vfs, VfsEntry, VfsMeta, WalkOptions};

#[derive(Debug, Clone)]
struct TarItemInfo {
    meta: VfsMeta,
    content: Option<Arc<Vec<u8>>>,
}

/// A read-only virtual filesystem reader for `.tar` and `.tar.gz` archives.
#[derive(Debug, Clone)]
pub struct TarGzFs {
    is_gzip_source: bool,
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
            if path_str.starts_with("PaxHeader/")
                || path_str.starts_with("./PaxHeader/")
                || path_str.contains("/PaxHeader/")
                || path_str.starts_with("PaxHeaders.")
                || path_str.contains("/PaxHeaders.")
            {
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

            let readonly = entry.header().mode().ok().is_some_and(|m| (m & 0o222) == 0);

            let content = if is_file {
                let mut data = Vec::with_capacity(len as usize);
                entry
                    .read_to_end(&mut data)
                    .map_err(|e| VfsError::CorruptedArchive(e.to_string()))?;
                Some(Arc::new(data))
            } else {
                None
            };

            let crc32 = content
                .as_ref()
                .map(|data| crc32fast::hash(data.as_slice()));

            let meta = if is_regular_dir {
                VfsMeta::directory(modified)
            } else if is_symlink {
                VfsMeta::symlink(modified)
            } else {
                let mut m = VfsMeta::file(len, modified, crc32);
                m.readonly = readonly;
                m
            };

            if is_regular_dir {
                known_dirs.insert(path.clone());
            }

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
            is_gzip_source: is_gzip,
            entries,
            dir_children,
        })
    }

    /// Returns `true` if the archive source was gzip-compressed.
    #[must_use]
    pub const fn is_gzip(&self) -> bool {
        self.is_gzip_source
    }

    /// Returns the number of regular file entries in the archive.
    #[must_use]
    pub fn file_count(&self) -> usize {
        self.entries
            .values()
            .filter(|info| info.meta.is_file)
            .count()
    }

    /// Returns the number of directory entries in the archive.
    #[must_use]
    pub fn dir_count(&self) -> usize {
        self.entries
            .values()
            .filter(|info| info.meta.is_dir)
            .count()
    }

    /// Returns the total byte size of regular files in the archive.
    #[must_use]
    pub fn total_bytes(&self) -> u64 {
        self.entries
            .values()
            .filter(|info| info.meta.is_file)
            .map(|info| info.meta.len)
            .fold(0_u64, u64::saturating_add)
    }

    /// Returns `true` if the archive contains the given virtual path.
    #[must_use]
    pub fn contains(&self, path: &VfsPath) -> bool {
        path.is_root() || self.entries.contains_key(path)
    }

    /// Case-insensitive lookup of a virtual path by raw string.
    #[must_use]
    pub fn find_case_insensitive(&self, query: &str) -> Option<VfsPath> {
        let needle = query.to_lowercase();
        self.entries
            .keys()
            .find(|p| p.as_str().to_lowercase() == needle)
            .cloned()
    }

    /// Returns the number of distinct items (files and directories) in the archive.
    #[must_use]
    pub fn entries_count(&self) -> usize {
        self.entries.len()
    }

    /// Returns the total uncompressed bytes of regular files in the archive.
    #[must_use]
    pub fn total_uncompressed_bytes(&self) -> u64 {
        self.entries
            .values()
            .filter(|info| info.meta.is_file)
            .map(|info| info.meta.len)
            .fold(0_u64, u64::saturating_add)
    }

    /// Returns a list of all normalized virtual paths present in the archive.
    #[must_use]
    pub fn file_names(&self) -> Vec<VfsPath> {
        self.entries.keys().cloned().collect()
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

        if item.meta.is_dir || item.meta.is_symlink {
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

/// Packs explicit path-to-byte entries into a plain tar archive.
///
/// # Errors
///
/// Returns [`VfsError::Io`] if tar serialization fails.
pub fn create_tar_archive(entries: &[(&VfsPath, &[u8])]) -> Result<Vec<u8>, VfsError> {
    let mut builder = tar::Builder::new(Vec::new());

    for (path, bytes) in entries {
        if path.is_root() {
            continue;
        }
        append_bytes_entry(&mut builder, path.as_str(), bytes)?;
    }

    builder.into_inner().map_err(VfsError::Io)
}

/// Packs a [`Vfs`] subtree into a plain tar archive by walking with `options`.
///
/// Directory entries are recorded explicitly; symlinks are skipped.
///
/// # Errors
///
/// Propagates VFS traversal and read errors, or tar serialization failures.
pub fn create_tar_from_vfs(
    vfs: &dyn Vfs,
    root: &VfsPath,
    options: &WalkOptions,
) -> Result<Vec<u8>, VfsError> {
    let walked = vfs.walk(root, options)?;
    let mut builder = tar::Builder::new(Vec::new());

    for entry in &walked {
        if entry.meta.is_symlink {
            continue;
        }
        let rel = relative_archive_path(&entry.path, root)?;
        if entry.meta.is_dir {
            append_dir_entry(&mut builder, &rel)?;
        } else if entry.meta.is_file {
            let bytes = vfs.read_all(&entry.path, u64::MAX)?;
            append_bytes_entry(&mut builder, &rel, &bytes)?;
        }
    }

    builder.into_inner().map_err(VfsError::Io)
}

/// Packs explicit path-to-byte entries into a gzip-compressed tar archive.
///
/// # Errors
///
/// Returns [`VfsError::Io`] if tar serialization or compression fails.
pub fn create_tar_gz_archive(entries: &[(&VfsPath, &[u8])]) -> Result<Vec<u8>, VfsError> {
    let tar_bytes = create_tar_archive(entries)?;
    gzip_compress(&tar_bytes)
}

/// Packs a [`Vfs`] subtree into a gzip-compressed tar archive.
///
/// # Errors
///
/// Propagates VFS traversal and read errors, or serialization failures.
pub fn create_tar_gz_from_vfs(
    vfs: &dyn Vfs,
    root: &VfsPath,
    options: &WalkOptions,
) -> Result<Vec<u8>, VfsError> {
    let tar_bytes = create_tar_from_vfs(vfs, root, options)?;
    gzip_compress(&tar_bytes)
}

/// Appends a regular file entry with the given raw bytes to the tar builder.
fn append_bytes_entry(
    builder: &mut tar::Builder<Vec<u8>>,
    name: &str,
    bytes: &[u8],
) -> Result<(), VfsError> {
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, name, bytes)
        .map_err(VfsError::Io)
}

/// Appends an explicit directory entry to the tar builder.
fn append_dir_entry(builder: &mut tar::Builder<Vec<u8>>, name: &str) -> Result<(), VfsError> {
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_cksum();
    builder
        .append_data(&mut header, name, &[] as &[u8])
        .map_err(VfsError::Io)
}

/// Gzip-compresses raw tar bytes.
fn gzip_compress(tar_bytes: &[u8]) -> Result<Vec<u8>, VfsError> {
    use flate2::Compression;
    use flate2::write::GzEncoder;
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(tar_bytes).map_err(VfsError::Io)?;
    encoder.finish().map_err(VfsError::Io)
}

/// Resolves an archive-relative name for `path` under `root`.
fn relative_archive_path(path: &VfsPath, root: &VfsPath) -> Result<String, VfsError> {
    match path.strip_prefix(root) {
        Some(rel) if !rel.is_root() => Ok(rel.as_str().to_string()),
        _ => path
            .file_name()
            .map(ToString::to_string)
            .ok_or_else(|| VfsError::InvalidPath(format!("cannot archive path: {path}"))),
    }
}

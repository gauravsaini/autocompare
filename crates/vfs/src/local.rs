//! Physical filesystem implementation with symlink safety and atomic writes.

#![forbid(unsafe_code)]

use std::collections::{HashSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::VfsError;
use crate::path::VfsPath;
use crate::trait_def::{Vfs, VfsEntry, VfsMeta, WalkOptions};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// A virtual filesystem backed by a directory on the local physical filesystem.
#[derive(Debug, Clone)]
pub struct LocalFs {
    root_dir: PathBuf,
    canonical_root: Option<PathBuf>,
    readonly: bool,
}

impl LocalFs {
    /// Creates a new [`LocalFs`] rooted at the given base directory.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::NotFound`] if `base_dir` does not exist or
    /// [`VfsError::NotADirectory`] if it is not a directory.
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self, VfsError> {
        let path = base_dir.as_ref().to_path_buf();
        let meta = fs::metadata(&path).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(VfsPath::root()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(VfsPath::root()),
            _ => VfsError::Io(err),
        })?;

        if !meta.is_dir() {
            return Err(VfsError::NotADirectory(VfsPath::root()));
        }

        let canonical_root = fs::canonicalize(&path).ok();

        Ok(Self {
            root_dir: path,
            canonical_root,
            readonly: false,
        })
    }

    /// Sets the read-only mode for this filesystem (builder style).
    #[must_use]
    pub fn with_readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    /// Mutates the read-only mode in-place.
    pub fn set_readonly(&mut self, readonly: bool) {
        self.readonly = readonly;
    }

    /// Returns the root path on the local filesystem.
    #[must_use]
    pub fn root_path(&self) -> &Path {
        &self.root_dir
    }

    /// Returns the canonicalized root path, if available.
    #[must_use]
    pub fn canonical_root(&self) -> Option<&Path> {
        self.canonical_root.as_deref()
    }

    /// Resolves a [`VfsPath`] to a physical path and verifies it stays inside `root_dir`.
    fn resolve(&self, path: &VfsPath) -> Result<PathBuf, VfsError> {
        let physical = path.to_physical_path(&self.root_dir);
        // Verify prefix containment without following external symlinks
        if !physical.starts_with(&self.root_dir) {
            return Err(VfsError::TraversalEscape(path.clone()));
        }
        Ok(physical)
    }

    /// Verifies that a target physical path does not escape the canonical root via symlinks.
    fn check_traversal_escape(&self, physical: &Path, vfs_path: &VfsPath) -> Result<(), VfsError> {
        if vfs_path.is_root() {
            return Ok(());
        }
        let canon_root = match &self.canonical_root {
            Some(r) => r,
            None => return Ok(()),
        };

        if let Ok(canon) = fs::canonicalize(physical)
            && !canon.starts_with(canon_root)
        {
            return Err(VfsError::TraversalEscape(vfs_path.clone()));
        } else {
            let mut curr = physical.parent();
            while let Some(parent) = curr {
                if parent.exists() {
                    if let Ok(canon) = fs::canonicalize(parent)
                        && !canon.starts_with(canon_root)
                    {
                        return Err(VfsError::TraversalEscape(vfs_path.clone()));
                    }
                    break;
                }
                curr = parent.parent();
            }
        }
        Ok(())
    }

    /// Symlink-safe walk that actively detects and refuses symlink cycles.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::SymlinkCycle`] if a directory symlink loop is detected.
    pub fn walk_symlink_safe(
        &self,
        root: &VfsPath,
        options: &WalkOptions,
    ) -> Result<Vec<VfsEntry>, VfsError> {
        let root_physical = self.resolve(root)?;
        let root_meta = self.stat(root)?;

        if !root_meta.is_dir {
            return Ok(vec![VfsEntry::new(root.clone(), root_meta)]);
        }

        let mut results = Vec::new();
        let mut visited_canonical_dirs = HashSet::new();

        let canon_root = fs::canonicalize(&root_physical).ok();
        if let Some(ref cr) = canon_root {
            visited_canonical_dirs.insert(cr.clone());
        }

        let mut queue = VecDeque::new();
        queue.push_back((root.clone(), root_physical.clone(), 0_usize));

        while let Some((current_vfs, current_phys, current_depth)) = queue.pop_front() {
            if let Some(max) = options.max_depth
                && current_depth >= max
            {
                continue;
            }

            let read_dir = match fs::read_dir(&current_phys) {
                Ok(rd) => rd,
                Err(err) => match err.kind() {
                    io::ErrorKind::PermissionDenied => {
                        return Err(VfsError::PermissionDenied(current_vfs));
                    }
                    io::ErrorKind::NotFound => return Err(VfsError::NotFound(current_vfs)),
                    _ => return Err(VfsError::Io(err)),
                },
            };

            let mut dir_entries = Vec::new();
            for entry_res in read_dir {
                let entry = entry_res.map_err(VfsError::Io)?;
                let file_name = match entry.file_name().into_string() {
                    Ok(name) => name,
                    Err(_) => continue, // Skip non-UTF8 filenames
                };

                let child_vfs = current_vfs.join(&file_name)?;
                let child_phys = entry.path();
                let sym_meta = fs::symlink_metadata(&child_phys).map_err(VfsError::Io)?;
                let is_symlink = sym_meta.file_type().is_symlink();

                let (is_dir, is_file) = if is_symlink {
                    (false, false)
                } else {
                    (sym_meta.is_dir(), sym_meta.is_file())
                };

                let vfs_meta = VfsMeta {
                    is_dir,
                    is_file,
                    is_symlink,
                    len: if is_dir { 0 } else { sym_meta.len() },
                    modified: sym_meta.modified().ok(),
                    readonly: sym_meta.permissions().readonly(),
                    crc32: None,
                };

                dir_entries.push(VfsEntry {
                    path: child_vfs.clone(),
                    name: file_name,
                    meta: vfs_meta,
                });

                if is_dir {
                    if let Ok(canon) = fs::canonicalize(&child_phys) {
                        visited_canonical_dirs.insert(canon);
                    }
                    queue.push_back((child_vfs, child_phys, current_depth.saturating_add(1)));
                } else if is_symlink && options.follow_symlinks {
                    // Check if symlink target is a directory and whether it forms a cycle or escapes root
                    match fs::canonicalize(&child_phys) {
                        Ok(target_canon) => {
                            if let Some(ref cr) = canon_root
                                && !target_canon.starts_with(cr)
                            {
                                if options.refuse_traversal_escape {
                                    return Err(VfsError::TraversalEscape(child_vfs));
                                }
                                continue;
                            }
                            if visited_canonical_dirs.contains(&target_canon) {
                                if options.refuse_cycles {
                                    return Err(VfsError::SymlinkCycle(child_vfs));
                                }
                                // Skip cycle if not refusing
                            } else if target_canon.is_dir() {
                                visited_canonical_dirs.insert(target_canon.clone());
                                queue.push_back((
                                    child_vfs,
                                    target_canon,
                                    current_depth.saturating_add(1),
                                ));
                            }
                        }
                        Err(err) => {
                            // Dangling symlinks don't cycle
                            if err.kind() == io::ErrorKind::NotFound {
                                continue;
                            }
                            return Err(VfsError::Io(err));
                        }
                    }
                }
            }

            // Sort deterministically by filename
            dir_entries.sort_by(|a, b| a.name.cmp(&b.name));
            results.extend(dir_entries);
        }

        Ok(results)
    }
}

impl Vfs for LocalFs {
    fn list_dir(&self, path: &VfsPath) -> Result<Vec<VfsEntry>, VfsError> {
        // ODD Phase 6: I/O summary span (entry count only, never names).
        let odd_span = tracing::info_span!(
            "vfs.list_dir",
            component = "rs.vfs/local",
            path = %crate::odd::path_token(path.as_str()),
            entries = tracing::field::Empty,
        );
        let _odd_guard = if crate::odd::enabled() {
            Some(odd_span.enter())
        } else {
            None
        };
        let physical = self.resolve(path)?;
        self.check_traversal_escape(&physical, path)?;

        let meta = fs::metadata(&physical).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(path.clone()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(path.clone()),
            _ => VfsError::Io(err),
        })?;

        if !meta.is_dir() {
            return Err(VfsError::NotADirectory(path.clone()));
        }

        let read_dir = fs::read_dir(&physical).map_err(|err| match err.kind() {
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(path.clone()),
            _ => VfsError::Io(err),
        })?;

        let mut entries = Vec::new();
        for res in read_dir {
            let entry = res.map_err(VfsError::Io)?;
            let file_name = match entry.file_name().into_string() {
                Ok(name) => name,
                Err(_) => continue,
            };

            let child_vfs = path.join(&file_name)?;
            let child_phys = entry.path();
            let sym_meta = fs::symlink_metadata(&child_phys).map_err(VfsError::Io)?;
            let is_symlink = sym_meta.file_type().is_symlink();

            let (is_dir, is_file) = if is_symlink {
                (false, false)
            } else {
                (sym_meta.is_dir(), sym_meta.is_file())
            };

            let vfs_meta = VfsMeta {
                is_dir,
                is_file,
                is_symlink,
                len: if is_dir { 0 } else { sym_meta.len() },
                modified: sym_meta.modified().ok(),
                readonly: sym_meta.permissions().readonly(),
                crc32: None,
            };

            entries.push(VfsEntry {
                path: child_vfs,
                name: file_name,
                meta: vfs_meta,
            });
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        odd_span.record("entries", entries.len() as u64);
        Ok(entries)
    }

    fn stat(&self, path: &VfsPath) -> Result<VfsMeta, VfsError> {
        // ODD Phase 6: I/O summary span (shape only, never names).
        let odd_span = tracing::info_span!(
            "vfs.stat",
            component = "rs.vfs/local",
            path = %crate::odd::path_token(path.as_str()),
            is_dir = tracing::field::Empty,
            len = tracing::field::Empty,
        );
        let _odd_guard = if crate::odd::enabled() {
            Some(odd_span.enter())
        } else {
            None
        };
        let physical = self.resolve(path)?;
        if !path.is_root()
            && let Some(parent) = physical.parent()
            && let Some(ref canon_root) = self.canonical_root
            && let Ok(canon_parent) = fs::canonicalize(parent)
            && !canon_parent.starts_with(canon_root)
        {
            return Err(VfsError::TraversalEscape(path.clone()));
        }

        let sym_meta = fs::symlink_metadata(&physical).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(path.clone()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(path.clone()),
            _ => VfsError::Io(err),
        })?;

        let is_symlink = sym_meta.file_type().is_symlink();
        let (is_dir, is_file) = if is_symlink {
            (false, false)
        } else {
            (sym_meta.is_dir(), sym_meta.is_file())
        };

        let meta = VfsMeta {
            is_dir,
            is_file,
            is_symlink,
            len: if is_dir { 0 } else { sym_meta.len() },
            modified: sym_meta.modified().ok(),
            readonly: sym_meta.permissions().readonly(),
            crc32: None,
        };
        odd_span.record("is_dir", meta.is_dir);
        odd_span.record("len", meta.len);
        Ok(meta)
    }

    fn read_stream(
        &self,
        path: &VfsPath,
        max_bytes: u64,
    ) -> Result<Box<dyn Read + Send>, VfsError> {
        // ODD Phase 6: I/O summary span (byte counts only, never contents).
        let odd_span = tracing::info_span!(
            "vfs.read",
            component = "rs.vfs/local",
            path = %crate::odd::path_token(path.as_str()),
            max_bytes = max_bytes,
            bytes = tracing::field::Empty,
        );
        let _odd_guard = if crate::odd::enabled() {
            Some(odd_span.enter())
        } else {
            None
        };
        let physical = self.resolve(path)?;
        self.check_traversal_escape(&physical, path)?;

        let meta = fs::metadata(&physical).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(path.clone()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(path.clone()),
            _ => VfsError::Io(err),
        })?;

        if meta.is_dir() {
            return Err(VfsError::NotAFile(path.clone()));
        }

        if meta.len() > max_bytes {
            return Err(VfsError::FileTooLarge {
                path: path.clone(),
                max_bytes,
                actual: meta.len(),
            });
        }

        let file = File::open(&physical).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(path.clone()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(path.clone()),
            _ => VfsError::Io(err),
        })?;

        odd_span.record("bytes", meta.len());
        Ok(Box::new(BoundedReader {
            inner: file,
            bytes_read: 0,
            max_bytes,
            path: path.clone(),
        }))
    }

    fn write_atomic(&self, path: &VfsPath, bytes: &[u8]) -> Result<(), VfsError> {
        // ODD Phase 6: I/O summary span (byte count only, never contents).
        let odd_span = tracing::info_span!(
            "vfs.write",
            component = "rs.vfs/local",
            path = %crate::odd::path_token(path.as_str()),
            bytes = bytes.len() as u64,
            ok = tracing::field::Empty,
        );
        let _odd_guard = if crate::odd::enabled() {
            Some(odd_span.enter())
        } else {
            None
        };
        if self.readonly {
            return Err(VfsError::ReadOnly);
        }

        let target_phys = self.resolve(path)?;
        self.check_traversal_escape(&target_phys, path)?;

        let parent = target_phys
            .parent()
            .ok_or_else(|| VfsError::InvalidPath("target has no parent directory".to_string()))?;

        fs::create_dir_all(parent).map_err(VfsError::Io)?;

        let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let now_epoch = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let pid = std::process::id();

        let file_name = path.file_name().unwrap_or("file");
        let temp_name = format!(".{file_name}.tmp.{pid}.{now_epoch}.{counter}");
        let temp_path = parent.join(temp_name);

        let write_result = (|| -> io::Result<()> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path)?;
            file.write_all(bytes)?;
            file.flush()?;
            file.sync_data()?;
            Ok(())
        })();

        if let Err(err) = write_result {
            let _ = fs::remove_file(&temp_path);
            return Err(VfsError::Io(err));
        }

        if let Err(err) = fs::rename(&temp_path, &target_phys) {
            let _ = fs::remove_file(&temp_path);
            return Err(VfsError::Io(err));
        }

        odd_span.record("ok", true);
        Ok(())
    }

    fn delete(&self, path: &VfsPath) -> Result<(), VfsError> {
        if self.readonly {
            return Err(VfsError::ReadOnly);
        }

        let physical = self.resolve(path)?;
        let sym_meta = fs::symlink_metadata(&physical).map_err(|err| match err.kind() {
            io::ErrorKind::NotFound => VfsError::NotFound(path.clone()),
            io::ErrorKind::PermissionDenied => VfsError::PermissionDenied(path.clone()),
            _ => VfsError::Io(err),
        })?;

        if sym_meta.file_type().is_symlink() {
            if let Some(parent) = physical.parent()
                && let Some(ref canon_root) = self.canonical_root
                && let Ok(canon_parent) = fs::canonicalize(parent)
                && !canon_parent.starts_with(canon_root)
            {
                return Err(VfsError::TraversalEscape(path.clone()));
            }
        } else {
            self.check_traversal_escape(&physical, path)?;
        }

        if sym_meta.is_dir() {
            fs::remove_dir_all(&physical).map_err(VfsError::Io)?;
        } else {
            fs::remove_file(&physical).map_err(VfsError::Io)?;
        }

        Ok(())
    }

    fn is_read_only(&self) -> bool {
        self.readonly
    }

    fn walk(&self, root: &VfsPath, options: &WalkOptions) -> Result<Vec<VfsEntry>, VfsError> {
        self.walk_symlink_safe(root, options)
    }
}

/// A reader wrapper that enforces a strict byte cap.
struct BoundedReader<R: Read + Send> {
    inner: R,
    bytes_read: u64,
    max_bytes: u64,
    path: VfsPath,
}

impl<R: Read + Send> Read for BoundedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.inner.read(buf)?;
        if n == 0 {
            return Ok(0);
        }

        self.bytes_read = self.bytes_read.saturating_add(n as u64);
        if self.bytes_read > self.max_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "file size exceeded maximum allowed limit of {} bytes at path {}",
                    self.max_bytes, self.path
                ),
            ));
        }

        Ok(n)
    }
}

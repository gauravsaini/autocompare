//! URI-routed Virtual Filesystem registry.
//!
//! Routes scheme-qualified URIs (`file://`, `zip://`, `tar://`, `bcss://`, or
//! user-registered custom schemes) and plain local paths to concrete [`Vfs`]
//! implementations without hardcoding filesystem types in callers.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::archive_tar::TarGzFs;
use crate::archive_zip::ZipFs;
use crate::error::VfsError;
use crate::local::LocalFs;
use crate::path::VfsPath;
use crate::snapshot::SnapshotFs;
use crate::trait_def::Vfs;

/// Factory closure that opens a [`Vfs`] instance for a scheme-stripped target path/authority.
pub type VfsFactory = Arc<dyn Fn(&str) -> Result<Box<dyn Vfs>, VfsError> + Send + Sync>;

/// Parsed components of a VFS URI or plain local path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VfsUri {
    /// Lowercase scheme identifier (e.g., `"file"`, `"zip"`, `"tar"`, `"bcss"`).
    pub scheme: String,
    /// Scheme-stripped target path or authority (e.g., `"/tmp/archive.zip"`).
    pub target: String,
    /// Optional internal [`VfsPath`] specified after `#` (defaults to root).
    pub subpath: VfsPath,
}

impl VfsUri {
    /// Parses a URI string (`scheme://target[#subpath]`) or auto-detects a plain path.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::InvalidPath`] if the optional `#subpath` fragment is invalid.
    pub fn parse(input: &str) -> Result<Self, VfsError> {
        let trimmed = input.trim();
        let (main_part, fragment) = match trimmed.split_once('#') {
            Some((left, right)) => (left, Some(right)),
            None => (trimmed, None),
        };

        let subpath = match fragment {
            Some(frag) if !frag.trim().is_empty() => VfsPath::new(frag.trim())?,
            _ => VfsPath::root(),
        };

        if let Some((raw_scheme, raw_target)) = main_part.split_once("://") {
            let scheme = raw_scheme.trim().to_ascii_lowercase();
            if scheme.is_empty() {
                return Err(VfsError::InvalidPath("empty URI scheme".to_string()));
            }
            return Ok(Self {
                scheme,
                target: raw_target.to_string(),
                subpath,
            });
        }

        let detected = detect_scheme_from_path(main_part);
        Ok(Self {
            scheme: detected.to_string(),
            target: main_part.to_string(),
            subpath,
        })
    }
}

/// Auto-detects the appropriate built-in VFS scheme from a plain file or directory path.
#[must_use]
pub fn detect_scheme_from_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".zip") {
        "zip"
    } else if lower.ends_with(".tar.gz")
        || lower.ends_with(".tgz")
        || lower.ends_with(".tar")
        || lower.ends_with(".gz")
    {
        "tar"
    } else if lower.ends_with(".bcss") {
        "bcss"
    } else {
        "file"
    }
}

/// Extensible registry mapping URI schemes to [`Vfs`] opener factories.
#[derive(Default)]
pub struct VfsRegistry {
    factories: BTreeMap<String, VfsFactory>,
}

impl std::fmt::Debug for VfsRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VfsRegistry")
            .field("schemes", &self.schemes())
            .finish()
    }
}

impl VfsRegistry {
    /// Creates an empty registry with no schemes registered.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }

    /// Creates a registry pre-populated with all built-in VFS providers:
    /// `file://`, `zip://`, `tar://`, `tgz://`, and `bcss://`.
    #[must_use]
    pub fn with_builtins() -> Self {
        let mut reg = Self::empty();
        reg.register_scheme(
            "file",
            Arc::new(|target: &str| {
                let fs = LocalFs::new(Path::new(target))?;
                Ok(Box::new(fs))
            }),
        );
        reg.register_scheme(
            "zip",
            Arc::new(|target: &str| {
                let fs = ZipFs::open_file(Path::new(target))?;
                Ok(Box::new(fs))
            }),
        );
        let tar_factory: VfsFactory = Arc::new(|target: &str| {
            let fs = TarGzFs::open_file(Path::new(target))?;
            Ok(Box::new(fs))
        });
        reg.register_scheme("tar", Arc::clone(&tar_factory));
        reg.register_scheme("tgz", tar_factory);
        reg.register_scheme(
            "bcss",
            Arc::new(|target: &str| {
                let fs = SnapshotFs::open_file(Path::new(target))?;
                Ok(Box::new(fs))
            }),
        );
        reg
    }

    /// Registers or replaces the factory for `scheme` (normalized to ASCII lowercase).
    pub fn register_scheme(&mut self, scheme: impl Into<String>, factory: VfsFactory) {
        let key = scheme.into().trim().to_ascii_lowercase();
        if !key.is_empty() {
            self.factories.insert(key, factory);
        }
    }

    /// Returns `true` if a factory is registered for `scheme`.
    #[must_use]
    pub fn supports(&self, scheme: &str) -> bool {
        self.factories
            .contains_key(&scheme.trim().to_ascii_lowercase())
    }

    /// Returns all registered scheme names in deterministic sorted order.
    #[must_use]
    pub fn schemes(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }

    /// Opens a [`Vfs`] from a URI or plain path, discarding any `#subpath` fragment.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::UnsupportedFormat`] if the scheme is not registered,
    /// or propagates any underlying VFS open error.
    pub fn open(&self, uri_or_path: &str) -> Result<Box<dyn Vfs>, VfsError> {
        let (vfs, _subpath) = self.open_with_subpath(uri_or_path)?;
        Ok(vfs)
    }

    /// Opens a [`Vfs`] from a URI or plain path and also returns its parsed `#subpath`.
    ///
    /// # Errors
    ///
    /// Returns [`VfsError::UnsupportedFormat`] if the scheme is not registered,
    /// [`VfsError::InvalidPath`] if the URI or fragment is malformed, or propagates
    /// the underlying VFS open error.
    pub fn open_with_subpath(
        &self,
        uri_or_path: &str,
    ) -> Result<(Box<dyn Vfs>, VfsPath), VfsError> {
        // ODD Phase 6: I/O summary span (scheme token only, never raw URI).
        let scheme = crate::registry::detect_scheme_from_path(uri_or_path);
        let odd_span = tracing::info_span!(
            "vfs.registry_open",
            component = "rs.vfs/registry",
            scheme = crate::odd::scheme_token(scheme),
            ok = tracing::field::Empty,
        );
        let _odd_guard = if crate::odd::enabled() {
            Some(odd_span.enter())
        } else {
            None
        };
        let out = self.open_with_subpath_inner(uri_or_path)?;
        odd_span.record("ok", true);
        Ok(out)
    }

    fn open_with_subpath_inner(
        &self,
        uri_or_path: &str,
    ) -> Result<(Box<dyn Vfs>, VfsPath), VfsError> {
        let parsed = VfsUri::parse(uri_or_path)?;
        let factory = self.factories.get(&parsed.scheme).ok_or_else(|| {
            VfsError::UnsupportedFormat(format!(
                "no VFS provider registered for scheme '{}://'",
                parsed.scheme
            ))
        })?;
        let vfs = factory(&parsed.target)?;
        Ok((vfs, parsed.subpath))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]
mod tests {
    use super::*;
    use crate::archive_tar::create_tar_gz_archive;
    use crate::archive_zip::create_zip_archive;
    use crate::snapshot::{SnapshotDocument, SnapshotEntry, serialize_bcss_json};

    #[test]
    fn parses_uris_and_autodetects_plain_paths() {
        let u1 = VfsUri::parse("zip:///tmp/release.zip#src/lib.rs").expect("parse zip uri");
        assert_eq!(u1.scheme, "zip");
        assert_eq!(u1.target, "/tmp/release.zip");
        assert_eq!(u1.subpath.as_str(), "src/lib.rs");

        let u2 = VfsUri::parse("/tmp/backup.tar.gz").expect("parse tar.gz path");
        assert_eq!(u2.scheme, "tar");
        assert_eq!(u2.target, "/tmp/backup.tar.gz");
        assert!(u2.subpath.is_root());

        let u3 = VfsUri::parse("/tmp/snap.bcss").expect("parse bcss path");
        assert_eq!(u3.scheme, "bcss");

        let u4 = VfsUri::parse("/tmp/project").expect("parse local dir");
        assert_eq!(u4.scheme, "file");
    }

    fn make_temp_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "autocompare_vfs_reg_{}_{}",
            label,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir temp");
        dir
    }

    #[test]
    fn opens_local_zip_tar_and_bcss_through_registry() {
        let reg = VfsRegistry::with_builtins();
        assert!(reg.supports("file"));
        assert!(reg.supports("zip"));
        assert!(reg.supports("tar"));
        assert!(reg.supports("bcss"));

        let tmp = make_temp_dir("builtins");
        std::fs::write(tmp.join("hello.txt"), b"world").expect("write hello");

        // 1. LocalFs via file://
        let file_uri = format!("file://{}#hello.txt", tmp.display());
        let (local_vfs, sub) = reg.open_with_subpath(&file_uri).expect("open file://");
        assert_eq!(sub.as_str(), "hello.txt");
        assert_eq!(local_vfs.read_all(&sub, 1024).expect("read"), b"world");

        // 2. ZipFs via zip://
        let zip_path = tmp.join("bundle.zip");
        let entry_path = VfsPath::new("nested/a.txt").expect("vfs path");
        let zip_bytes = create_zip_archive(&[(&entry_path, b"from-zip")]).expect("zip");
        std::fs::write(&zip_path, zip_bytes).expect("write zip");

        let zip_uri = format!("zip://{}#nested/a.txt", zip_path.display());
        let (zip_vfs, zsub) = reg.open_with_subpath(&zip_uri).expect("open zip://");
        assert_eq!(zip_vfs.read_all(&zsub, 1024).expect("read zip"), b"from-zip");

        // 3. TarGzFs via plain .tar.gz auto-detection
        let tar_path = tmp.join("bundle.tar.gz");
        let tar_bytes = create_tar_gz_archive(&[(&entry_path, b"from-tar")]).expect("tar");
        std::fs::write(&tar_path, tar_bytes).expect("write tar");

        let tar_vfs = reg
            .open(&tar_path.display().to_string())
            .expect("open tar.gz");
        assert_eq!(
            tar_vfs.read_all(&entry_path, 1024).expect("read tar"),
            b"from-tar"
        );

        // 4. SnapshotFs via bcss://
        let bcss_path = tmp.join("tree.bcss");
        let mut doc = SnapshotDocument::new("demo");
        doc.add_entry(SnapshotEntry {
            path: "README.md".to_string(),
            is_dir: false,
            size: 42,
            modified_epoch_secs: Some(1_700_000_000),
            crc32: Some(1234),
            readonly: false,
        });
        std::fs::write(&bcss_path, serialize_bcss_json(&doc).expect("bcss json"))
            .expect("write bcss");

        let bcss_uri = format!("bcss://{}", bcss_path.display());
        let bcss_vfs = reg.open(&bcss_uri).expect("open bcss://");
        let readme = VfsPath::new("README.md").expect("readme path");
        assert_eq!(bcss_vfs.stat(&readme).expect("stat").len, 42);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn rejects_unknown_scheme_and_allows_custom_registration() {
        let mut reg = VfsRegistry::with_builtins();
        let err = match reg.open("s3://my-bucket/prefix") {
            Err(e) => e,
            Ok(_) => panic!("expected UnsupportedFormat for s3://"),
        };
        assert!(matches!(err, VfsError::UnsupportedFormat(_)));

        let root = make_temp_dir("custom");
        let root_clone = root.clone();
        reg.register_scheme(
            "mem-alias",
            Arc::new(move |_target: &str| Ok(Box::new(LocalFs::new(&root_clone)?))),
        );
        assert!(reg.supports("mem-alias"));
        let vfs = reg.open("mem-alias://anything").expect("open custom");
        assert!(!vfs.is_read_only());
        let _ = std::fs::remove_dir_all(&root);
    }
}

//! Virtual filesystem abstraction layer for Beyond Compare replica.
//!
//! Provides clean, symlink-safe abstractions over:
//! - Local physical filesystems ([`LocalFs`]) with symlink cycle prevention and atomic writes
//! - Pure-Rust Zip archives ([`ZipFs`])
//! - Pure-Rust Tar and Tar.gz archives ([`TarGzFs`])
//! - Versioned Beyond Compare Session Snapshot (.bcss) persistence and virtual browsing ([`SnapshotFs`])
//!
//! Layer 1: Domain & VFS abstractions.

#![forbid(unsafe_code)]

pub mod archive_tar;
pub mod archive_zip;
pub mod error;
pub mod local;
pub mod path;
pub mod registry;
pub mod snapshot;
pub mod trait_def;

pub use archive_tar::{
    TarGzFs, create_tar_archive, create_tar_from_vfs, create_tar_gz_archive, create_tar_gz_from_vfs,
};
pub use archive_zip::{ArchiveFs, ZipFs, create_zip_archive, create_zip_from_vfs};
pub use error::VfsError;
pub use local::LocalFs;
pub use path::{VfsPath, VfsPathAncestors};
pub use registry::{VfsFactory, VfsRegistry, VfsUri, detect_scheme_from_path};
pub use snapshot::{
    BCSS_SCHEMA_VERSION, DetailedSnapshotDiff, SnapshotDiffSummary, SnapshotDocument,
    SnapshotEntry, SnapshotFs, create_deep_snapshot, create_shallow_snapshot, create_snapshot,
    create_snapshot_with_options, deserialize_bcss, serialize_bcss_compressed, serialize_bcss_json,
};
pub use trait_def::{Vfs, VfsEntry, VfsMeta, WalkOptions};

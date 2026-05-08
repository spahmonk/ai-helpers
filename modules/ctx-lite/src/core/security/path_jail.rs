use std::ffi::{OsStr, OsString};
use std::fs::{self, File, Metadata, OpenOptions};
use std::path::{Path, PathBuf};

use crate::core::config::AppConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathJail {
    project_root: PathBuf,
    allowed_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct ResolvedPath {
    canonical_path: PathBuf,
    allowed_roots: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct ResolvedDirEntry {
    file_name: OsString,
    path: ResolvedPath,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PathJailError {
    pub kind: PathJailErrorKind,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PathJailErrorKind {
    InvalidRoot,
    OutsideAllowedRoot,
    NotFound,
    SymlinkEscape,
    HardLinkDenied,
    Io,
}

impl PathJail {
    pub fn from_config(config: &AppConfig) -> Result<Self, PathJailError> {
        let project_root = canonicalize_root(&config.project_root)?;
        let allowed_roots = if config.allowed_roots.is_empty() {
            vec![project_root.clone()]
        } else {
            config
                .allowed_roots
                .iter()
                .map(|root| {
                    let candidate = if root.is_absolute() {
                        root.clone()
                    } else {
                        project_root.join(root)
                    };
                    canonicalize_root(&candidate)
                })
                .collect::<Result<Vec<_>, _>>()?
        };

        if allowed_roots
            .iter()
            .any(|root| root != &project_root && !root.starts_with(&project_root))
        {
            return Err(PathJailError {
                kind: PathJailErrorKind::InvalidRoot,
                message: format!(
                    "allowed roots must stay inside project root {}",
                    project_root.display()
                ),
            });
        }

        Ok(Self {
            project_root,
            allowed_roots,
        })
    }

    pub fn resolve(&self, requested: &Path) -> Result<ResolvedPath, PathJailError> {
        let candidate = if requested.is_absolute() {
            strip_extended_path_prefix(requested.to_path_buf())
        } else {
            self.project_root.join(requested)
        };
        let normalized_candidate = lexically_normalize(&candidate);

        if !is_within_allowed_roots(&self.allowed_roots, &normalized_candidate) {
            return Err(PathJailError {
                kind: PathJailErrorKind::OutsideAllowedRoot,
                message: format!(
                    "path {} escapes the configured allowed root",
                    requested.display()
                ),
            });
        }

        let canonical = canonicalize_existing_path(&normalized_candidate)?;
        validate_allowed_path(&self.allowed_roots, requested, canonical)
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }
}

impl ResolvedPath {
    pub fn path(&self) -> &Path {
        &self.canonical_path
    }

    pub fn open_file(&self) -> Result<File, PathJailError> {
        self.revalidate_current_path()?;

        if self.canonical_path.is_dir() {
            return Err(PathJailError {
                kind: PathJailErrorKind::Io,
                message: format!(
                    "'{}' is a directory, not a file; use `ctx-lite tree` to list its contents",
                    self.canonical_path.display()
                ),
            });
        }

        let file = open_file_for_read(&self.canonical_path).map_err(|error| {
            map_open_error(&self.canonical_path, error, "failed to open validated file")
        })?;

        let current = self.revalidate_current_path()?;

        #[cfg(unix)]
        {
            let file_metadata = file.metadata().map_err(|error| PathJailError {
                kind: PathJailErrorKind::Io,
                message: format!(
                    "failed to inspect opened file {}: {}",
                    self.canonical_path.display(),
                    error
                ),
            })?;
            let path_metadata = fs::metadata(&current).map_err(|error| PathJailError {
                kind: PathJailErrorKind::Io,
                message: format!(
                    "failed to re-check {} after opening it: {}",
                    current.display(),
                    error
                ),
            })?;

            if !same_file(&file_metadata, &path_metadata) {
                return Err(PathJailError {
                    kind: PathJailErrorKind::SymlinkEscape,
                    message: format!(
                        "path {} changed after validation and was rejected",
                        self.canonical_path.display()
                    ),
                });
            }
        }

        Ok(file)
    }

    pub fn metadata(&self) -> Result<Metadata, PathJailError> {
        let current = self.revalidate_current_path()?;
        fs::metadata(&current).map_err(|error| PathJailError {
            kind: PathJailErrorKind::Io,
            message: format!("failed to inspect {}: {}", current.display(), error),
        })
    }

    pub fn read_dir(&self) -> Result<Vec<ResolvedDirEntry>, PathJailError> {
        let current = self.revalidate_current_path()?;
        let metadata = fs::metadata(&current).map_err(|error| PathJailError {
            kind: PathJailErrorKind::Io,
            message: format!("failed to inspect {}: {}", current.display(), error),
        })?;

        if !metadata.is_dir() {
            return Err(PathJailError {
                kind: PathJailErrorKind::Io,
                message: format!("path {} is not a directory", current.display()),
            });
        }

        let mut children = fs::read_dir(&current)
            .map_err(|error| PathJailError {
                kind: PathJailErrorKind::Io,
                message: format!("failed to list directory {}: {}", current.display(), error),
            })?
            .map(|entry| {
                let entry = entry.map_err(|error| PathJailError {
                    kind: PathJailErrorKind::Io,
                    message: format!(
                        "failed to read directory entry in {}: {}",
                        current.display(),
                        error
                    ),
                })?;
                let file_name = entry.file_name();
                let path = validate_allowed_path(
                    &self.allowed_roots,
                    entry.path().as_path(),
                    canonicalize_existing_path(&entry.path())?,
                )?;

                Ok(ResolvedDirEntry { file_name, path })
            })
            .collect::<Result<Vec<_>, _>>()?;

        children.sort_by(|left, right| left.path.canonical_path.cmp(&right.path.canonical_path));
        self.revalidate_current_path()?;
        Ok(children)
    }

    fn revalidate_current_path(&self) -> Result<PathBuf, PathJailError> {
        let current = canonicalize_existing_path(&self.canonical_path)?;
        enforce_file_link_policy(&current)?;

        if current == self.canonical_path && is_within_allowed_roots(&self.allowed_roots, &current)
        {
            Ok(current)
        } else {
            Err(PathJailError {
                kind: PathJailErrorKind::SymlinkEscape,
                message: format!(
                    "path {} changed after validation and now escapes the configured allowed root",
                    self.canonical_path.display()
                ),
            })
        }
    }
}

impl ResolvedDirEntry {
    pub fn file_name(&self) -> &OsStr {
        &self.file_name
    }

    pub fn path(&self) -> &ResolvedPath {
        &self.path
    }
}

fn validate_allowed_path(
    allowed_roots: &[PathBuf],
    requested: &Path,
    canonical: PathBuf,
) -> Result<ResolvedPath, PathJailError> {
    if is_within_allowed_roots(allowed_roots, &canonical) {
        enforce_file_link_policy_for_request(requested, &canonical)?;
        Ok(ResolvedPath {
            canonical_path: canonical,
            allowed_roots: allowed_roots.to_vec(),
        })
    } else {
        Err(PathJailError {
            kind: PathJailErrorKind::SymlinkEscape,
            message: format!(
                "path {} resolves through a symlink outside the configured allowed root",
                requested.display()
            ),
        })
    }
}

fn is_within_allowed_roots(allowed_roots: &[PathBuf], path: &Path) -> bool {
    allowed_roots
        .iter()
        .any(|root| path_is_within_root(path, root))
}

#[cfg(not(windows))]
fn path_is_within_root(path: &Path, root: &Path) -> bool {
    path == root || path.starts_with(root)
}

#[cfg(windows)]
fn path_is_within_root(path: &Path, root: &Path) -> bool {
    let mut path_components = path.components();

    for root_component in root.components() {
        let Some(path_component) = path_components.next() else {
            return false;
        };

        if !windows_path_component_eq(path_component, root_component) {
            return false;
        }
    }

    true
}

#[cfg(windows)]
fn windows_path_component_eq(
    left: std::path::Component<'_>,
    right: std::path::Component<'_>,
) -> bool {
    match (left, right) {
        (std::path::Component::Prefix(left), std::path::Component::Prefix(right)) => {
            left.as_os_str().to_string_lossy().to_lowercase()
                == right.as_os_str().to_string_lossy().to_lowercase()
        }
        (std::path::Component::RootDir, std::path::Component::RootDir) => true,
        (std::path::Component::Normal(left), std::path::Component::Normal(right)) => {
            left.to_string_lossy().to_lowercase() == right.to_string_lossy().to_lowercase()
        }
        _ => false,
    }
}

fn canonicalize_root(path: &Path) -> Result<PathBuf, PathJailError> {
    let canonical = fs::canonicalize(path).map_err(|error| PathJailError {
        kind: PathJailErrorKind::InvalidRoot,
        message: format!("failed to canonicalize root {}: {}", path.display(), error),
    })?;
    Ok(strip_extended_path_prefix(canonical))
}

fn canonicalize_existing_path(path: &Path) -> Result<PathBuf, PathJailError> {
    let canonical = fs::canonicalize(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => PathJailError {
            kind: PathJailErrorKind::NotFound,
            message: format!("path {} does not exist", path.display()),
        },
        _ => PathJailError {
            kind: PathJailErrorKind::Io,
            message: format!("failed to canonicalize {}: {}", path.display(), error),
        },
    })?;
    Ok(strip_extended_path_prefix(canonical))
}

/// On Windows, `fs::canonicalize` returns paths with the `\\?\` extended-length prefix.
/// Strip it so that canonical paths compare correctly against lexically-normalised paths
/// (which never have this prefix). Paths longer than MAX_PATH are not a practical concern
/// for a developer tool.
#[cfg(windows)]
fn strip_extended_path_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        PathBuf::from(rest)
    } else {
        path
    }
}

#[cfg(not(windows))]
#[inline]
fn strip_extended_path_prefix(path: PathBuf) -> PathBuf {
    path
}

fn enforce_file_link_policy(path: &Path) -> Result<(), PathJailError> {
    enforce_file_link_policy_for_request(path, path)
}

fn enforce_file_link_policy_for_request(
    requested: &Path,
    path: &Path,
) -> Result<(), PathJailError> {
    let metadata = fs::metadata(path).map_err(|error| PathJailError {
        kind: PathJailErrorKind::Io,
        message: format!("failed to inspect {}: {}", path.display(), error),
    })?;

    if metadata.is_file() && is_multiply_linked_file(&metadata) {
        return Err(PathJailError {
            kind: PathJailErrorKind::HardLinkDenied,
            message: format!(
                "path {} resolves to a multiply-linked file and hard links are not allowed",
                requested.display()
            ),
        });
    }

    Ok(())
}

#[cfg(unix)]
fn open_file_for_read(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options.read(true).custom_flags(libc::O_NOFOLLOW);
    options.open(path)
}

#[cfg(not(unix))]
fn open_file_for_read(path: &Path) -> std::io::Result<File> {
    File::open(path)
}

fn map_open_error(path: &Path, error: std::io::Error, prefix: &str) -> PathJailError {
    #[cfg(unix)]
    if error.raw_os_error() == Some(libc::ELOOP) {
        return PathJailError {
            kind: PathJailErrorKind::SymlinkEscape,
            message: format!(
                "path {} changed into a symlink after validation and was rejected",
                path.display()
            ),
        };
    }

    match error.kind() {
        std::io::ErrorKind::NotFound => PathJailError {
            kind: PathJailErrorKind::NotFound,
            message: format!("path {} does not exist", path.display()),
        },
        _ => PathJailError {
            kind: PathJailErrorKind::Io,
            message: format!("{} {}: {}", prefix, path.display(), error),
        },
    }
}

#[cfg(unix)]
fn same_file(left: &Metadata, right: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(unix)]
fn is_multiply_linked_file(metadata: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    metadata.nlink() > 1
}

#[cfg(not(unix))]
fn is_multiply_linked_file(_metadata: &Metadata) -> bool {
    false
}

fn lexically_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                normalized.push(Path::new(&std::path::MAIN_SEPARATOR.to_string()))
            }
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => normalized.push(part),
            std::path::Component::ParentDir => {
                if !normalized.pop() && !path.is_absolute() {
                    normalized.push("..");
                }
            }
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// Regression test: canonicalize_root must NOT return a path with the \\?\ extended-length
    /// prefix on Windows. If it does, comparisons against lexically-normalised paths (which
    /// never carry this prefix) would always fail and every absolute-path request would be
    /// rejected, even when the path is legitimately inside the allowed root.
    #[test]
    fn canonicalize_root_does_not_return_extended_path_prefix() {
        let cwd = env::current_dir().expect("must have a current directory");
        let canonical =
            canonicalize_root(&cwd).expect("current directory should canonicalize without error");
        let s = canonical.to_string_lossy();
        assert!(
            !s.starts_with(r"\\?\"),
            "canonicalized root must not contain the Windows \\?\\  extended-length prefix; got: {s}"
        );
    }

    /// Regression test: a PathJail built from the default AppConfig must accept an absolute
    /// path that is inside the current working directory.
    #[test]
    fn absolute_path_within_cwd_is_accepted() {
        use crate::core::config::AppConfig;

        let config = AppConfig::default();
        let jail = PathJail::from_config(&config)
            .expect("default AppConfig should produce a valid PathJail");

        // Build an absolute path that is definitely inside the allowed root
        // (the project root itself is always within allowed_roots).
        let inner = config.project_root.join("Cargo.toml");
        if !inner.exists() {
            // Skip in environments without Cargo.toml (e.g. installed binary tests)
            return;
        }

        jail.resolve(&inner)
            .expect("absolute path inside allowed root should be accepted");
    }
}

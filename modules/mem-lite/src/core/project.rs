use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::core::config::default_mem_lite_home;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectScope {
    pub workspace_root: PathBuf,
    pub project_id: String,
    pub database_path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectError {
    RelativeWorkspaceRoot(PathBuf),
    Io(String),
}

impl ProjectError {
    pub fn io(error: std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::RelativeWorkspaceRoot(path) => {
                write!(f, "workspace root must be absolute: {}", path.display())
            }
            ProjectError::Io(message) => f.write_str(message),
        }
    }
}

impl Error for ProjectError {}

impl ProjectScope {
    pub fn from_workspace_root(root: &Path) -> Result<Self, ProjectError> {
        if !root.is_absolute() {
            return Err(ProjectError::RelativeWorkspaceRoot(root.to_path_buf()));
        }

        let canonical = root.canonicalize().map_err(ProjectError::io)?;
        let digest = hash_path_identity(&canonical);
        let project_id = digest[..16].to_string();
        let database_path = default_mem_lite_home()
            .join("projects")
            .join(&project_id)
            .join("memory.sqlite");

        Ok(Self {
            workspace_root: canonical,
            project_id,
            database_path,
        })
    }
}

fn hash_path_identity(path: &Path) -> String {
    let mut hasher = Sha256::new();

    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        hasher.update(path.as_os_str().as_bytes());
    }

    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        for unit in path.as_os_str().encode_wide() {
            hasher.update(unit.to_le_bytes());
        }
    }

    hex::encode(hasher.finalize())
}

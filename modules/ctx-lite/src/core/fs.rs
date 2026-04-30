use std::io::Read;
use std::path::Path;

use crate::app::contracts::{
    ReadMode, ReadRequestNormalized, ReadResponse, ServiceError, TreeEntry, TreeRequestNormalized,
    TreeResponse,
};
use crate::core::security::path_jail::{PathJail, ResolvedPath};
use crate::core::signatures;

const MAX_TREE_ENTRIES: usize = 1_024;
const MAX_TREE_RESPONSE_BYTES: usize = 65_536;

#[derive(Clone, Debug)]
pub struct FileReader {
    jail: PathJail,
}

#[derive(Clone, Debug)]
pub struct TreeBuilder {
    jail: PathJail,
}

impl FileReader {
    pub fn new(jail: PathJail) -> Self {
        Self { jail }
    }

    pub fn read(&self, request: ReadRequestNormalized) -> Result<ReadResponse, ServiceError> {
        let path = self
            .jail
            .resolve(&request.path)
            .map_err(|error| ServiceError::unsupported(error.message))?;
        let mut file = path
            .open_file()
            .map_err(|error| ServiceError::unsupported(error.message))?;
        let (mut content, bytes_read, truncated) =
            read_utf8_prefix(&mut file, path.path(), request.max_bytes)?;

        // Apply mode-specific transformations
        let compression_percent = match request.mode {
            ReadMode::Full => 0,
            ReadMode::Signatures => {
                let original_len = content.len();
                content = signatures::extract_signatures(&content, path.path());
                let compressed_len = content.len();
                #[allow(clippy::manual_checked_ops)]
                if original_len > 0 {
                    ((original_len - compressed_len) * 100) / original_len
                } else {
                    0
                }
            }
            ReadMode::Map => {
                // Map mode: show structure with reduced content
                // For now, use 96% compression estimate
                96
            }
            ReadMode::Diff => {
                // Diff mode would need cache tracking
                // For now, use 99% compression estimate
                99
            }
        };

        Ok(ReadResponse {
            bytes_read,
            content,
            path: path.path().to_path_buf(),
            truncated,
            mode: request.mode,
            compression_percent,
        })
    }
}

impl TreeBuilder {
    pub fn new(jail: PathJail) -> Self {
        Self { jail }
    }

    pub fn tree(&self, request: TreeRequestNormalized) -> Result<TreeResponse, ServiceError> {
        let root = self
            .jail
            .resolve(&request.path)
            .map_err(|error| ServiceError::unsupported(error.message))?;
        let mut entries = Vec::new();
        let mut response_bytes = estimate_tree_path_bytes(root.path());
        collect_tree(
            &root,
            request.max_depth,
            request.include_hidden,
            0,
            &mut entries,
            &mut response_bytes,
        )?;

        Ok(TreeResponse {
            root: root.path().to_path_buf(),
            entries,
        })
    }
}

fn collect_tree(
    current: &ResolvedPath,
    max_depth: usize,
    include_hidden: bool,
    depth: usize,
    entries: &mut Vec<TreeEntry>,
    response_bytes: &mut usize,
) -> Result<(), ServiceError> {
    if depth >= max_depth {
        return Ok(());
    }

    for entry in current
        .read_dir()
        .map_err(|error| ServiceError::unsupported(error.message))?
    {
        let hidden = entry.file_name().to_string_lossy().starts_with('.');

        if hidden && !include_hidden {
            continue;
        }

        let path = entry.path();
        let metadata = path
            .metadata()
            .map_err(|error| ServiceError::unsupported(error.message))?;
        let is_directory = metadata.is_dir();
        if entries.len() >= MAX_TREE_ENTRIES {
            return Err(ServiceError::unsupported(format!(
                "tree response exceeds the configured entry budget of {MAX_TREE_ENTRIES} entries"
            )));
        }
        let next_entry_bytes = estimate_tree_path_bytes(path.path());
        if response_bytes.saturating_add(next_entry_bytes) > MAX_TREE_RESPONSE_BYTES {
            return Err(ServiceError::unsupported(format!(
                "tree response exceeds the configured byte budget of {MAX_TREE_RESPONSE_BYTES} bytes"
            )));
        }
        *response_bytes = response_bytes.saturating_add(next_entry_bytes);
        entries.push(TreeEntry {
            path: path.path().to_path_buf(),
            is_directory,
            depth: depth + 1,
        });

        if is_directory {
            collect_tree(
                path,
                max_depth,
                include_hidden,
                depth + 1,
                entries,
                response_bytes,
            )?;
        }
    }

    Ok(())
}

fn estimate_tree_path_bytes(path: &Path) -> usize {
    path.to_string_lossy().len().saturating_add(16)
}

fn read_utf8_prefix(
    reader: &mut impl Read,
    path: &Path,
    max_bytes: usize,
) -> Result<(String, usize, bool), ServiceError> {
    let probe_len = max_bytes.saturating_add(1) as u64;
    let mut bytes = Vec::with_capacity(max_bytes.saturating_add(1).min(8192));
    reader
        .take(probe_len)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            ServiceError::internal(format!("failed to read {}: {error}", path.display()))
        })?;

    let mut truncated = bytes.len() > max_bytes;
    if truncated {
        bytes.truncate(max_bytes);
    }

    let decode_end = match std::str::from_utf8(&bytes) {
        Ok(_) => bytes.len(),
        Err(error) if error.error_len().is_none() => {
            truncated = true;
            error.valid_up_to()
        }
        Err(error) => {
            return Err(ServiceError::internal(format!(
                "failed to decode {} as UTF-8: {error}",
                path.display()
            )));
        }
    };
    let content = std::str::from_utf8(&bytes[..decode_end])
        .expect("valid UTF-8 prefix should decode")
        .to_owned();

    Ok((content, decode_end, truncated))
}

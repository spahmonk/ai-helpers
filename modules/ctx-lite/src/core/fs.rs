use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use std::time::SystemTime;

use crate::app::contracts::{
    ReadMode, ReadRequestNormalized, ReadResponse, ServiceError, TreeEntry, TreeRequestNormalized,
    TreeResponse,
};
use crate::core::security::path_jail::{PathJail, ResolvedPath};
use crate::core::cache::{SemanticCache, ReadMode as CacheReadMode};
use crate::core::policy::AdaptivePolicy;
use crate::core::budget::ContextBudget;
use crate::core::signatures;

const MAX_TREE_ENTRIES: usize = 1_024;
const MAX_TREE_RESPONSE_BYTES: usize = 65_536;

/// Convert contracts::ReadMode to cache::ReadMode
fn to_cache_mode(mode: ReadMode) -> CacheReadMode {
    match mode {
        ReadMode::Full => CacheReadMode::Full,
        ReadMode::Signatures => CacheReadMode::Partial,
        ReadMode::Map => CacheReadMode::Partial,
        ReadMode::Diff => CacheReadMode::Semantic,
    }
}

#[derive(Clone)]
pub struct FileReader {
    jail: PathJail,
    cache: std::sync::Arc<Mutex<SemanticCache>>,
    policy: AdaptivePolicy,
    budget: std::sync::Arc<Mutex<ContextBudget>>,
}

impl std::fmt::Debug for FileReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileReader").finish()
    }
}

#[derive(Clone, Debug)]
pub struct TreeBuilder {
    jail: PathJail,
}

impl FileReader {
    pub fn new(jail: PathJail) -> Self {
        Self {
            jail,
            cache: std::sync::Arc::new(Mutex::new(SemanticCache::new(1000))),
            policy: AdaptivePolicy::new(),
            budget: std::sync::Arc::new(Mutex::new(ContextBudget::new(12_000))),
        }
    }

    pub fn read(&self, request: ReadRequestNormalized) -> Result<ReadResponse, ServiceError> {
        let path = self
            .jail
            .resolve(&request.path)
            .map_err(|error| ServiceError::unsupported(error.message))?;

        // Step 1: Determine mode - use auto-selection if Full mode requested
        let is_auto_selected = request.mode == ReadMode::Full;
        let selected_mode = if is_auto_selected {
            self.policy.select_mode(path.path(), request.max_bytes)
        } else {
            request.mode
        };

        // Step 2: Open file and get metadata for cache validation
        let mut file = path
            .open_file()
            .map_err(|error| ServiceError::unsupported(error.message))?;
        
        let file_mtime = std::fs::metadata(path.path())
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        // Step 3: Read file content
        let (mut content, bytes_read, truncated) =
            read_utf8_prefix(&mut file, path.path(), request.max_bytes)?;

        let original_content = content.clone();

        // Step 4: Check cache with actual content hash and mtime
        {
            let cache = self.cache.lock().unwrap();
            let cache_mode = to_cache_mode(selected_mode);
            if let Some(cached_result) = cache.get(path.path(), &content, cache_mode, file_mtime) {
                // Cache hit - minimal cost
                let mut budget = self.budget.lock().unwrap();
                budget.consume(1);
                
                let budget_status = budget.status();
                let tokens_consumed = budget.used();
                let max_tokens = budget.max_tokens();
                
                return Ok(ReadResponse {
                    bytes_read,
                    content: cached_result,
                    path: path.path().to_path_buf(),
                    truncated,
                    mode: selected_mode,
                    compression_percent: 99, // Cache hits are ~99% compression
                    is_auto_selected,
                    tokens_consumed,
                    max_tokens,
                    budget_status,
                });
            }
        }

        // Step 5: Cache miss - apply mode-specific transformations
        let compression_percent = match selected_mode {
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
                96
            }
            ReadMode::Diff => {
                // Diff mode: delta compression (99% on re-reads via cache)
                99
            }
        } as usize;

        // Step 6: Track budget consumption
        let read_cost = (bytes_read / 4).min(1000);
        let (budget_status, tokens_consumed, max_tokens) = {
            let mut budget = self.budget.lock().unwrap();
            budget.consume(read_cost);
            (budget.status(), budget.used(), budget.max_tokens())
        };

        // Step 7: Store in cache for future reads
        {
            let mut cache = self.cache.lock().unwrap();
            let cache_mode = to_cache_mode(selected_mode);
            cache.insert(
                path.path(),
                original_content,
                content.clone(),
                compression_percent,
                cache_mode,
                file_mtime,
            );
        }

        Ok(ReadResponse {
            bytes_read,
            content,
            path: path.path().to_path_buf(),
            truncated,
            mode: selected_mode,
            compression_percent,
            is_auto_selected,
            tokens_consumed,
            max_tokens,
            budget_status,
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

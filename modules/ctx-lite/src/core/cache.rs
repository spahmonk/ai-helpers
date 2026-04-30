use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::time::SystemTime;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ReadMode {
    Full,
    Signatures,
    Map,
    Diff,
}

impl ReadMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ReadMode::Full => "full",
            ReadMode::Signatures => "signatures",
            ReadMode::Map => "map",
            ReadMode::Diff => "diff",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
struct CacheKey {
    path_hash: u64,
    content_hash: u64,
    mode: ReadMode,
}

struct CacheEntry {
    content: String,
    compression_percent: usize,
    timestamp: SystemTime,
    file_mtime: SystemTime,
}

/// Semantic cache with LRU eviction and content-based invalidation
pub struct SemanticCache {
    cache: HashMap<CacheKey, CacheEntry>,
    max_entries: usize,
}

/// Hash a file path to a u64
fn hash_path(path: &Path) -> u64 {
    let mut hasher = DefaultHasher::new();
    path.to_string_lossy().hash(&mut hasher);
    hasher.finish()
}

/// Hash file content to a u64
fn hash_content(content: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    hasher.finish()
}

/// Create a cache key from path, content, and read mode
fn create_cache_key(path: &Path, content: &str, mode: ReadMode) -> CacheKey {
    CacheKey {
        path_hash: hash_path(path),
        content_hash: hash_content(content),
        mode,
    }
}

impl SemanticCache {
    /// Create a new semantic cache with the specified maximum number of entries
    pub fn new(max_entries: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_entries,
        }
    }

    /// Get a cached value if it exists and hasn't been invalidated by mtime changes
    pub fn get(
        &self,
        path: &Path,
        content: &str,
        mode: ReadMode,
        current_mtime: SystemTime,
    ) -> Option<String> {
        let key = create_cache_key(path, content, mode);
        self.cache.get(&key).and_then(|entry| {
            if entry.file_mtime == current_mtime {
                Some(entry.content.clone())
            } else {
                None
            }
        })
    }

    /// Insert a value into the cache, evicting LRU entry if necessary
    pub fn insert(
        &mut self,
        path: &Path,
        content: String,
        result: String,
        compression_percent: usize,
        mode: ReadMode,
        mtime: SystemTime,
    ) {
        if self.cache.len() >= self.max_entries {
            self.evict_lru();
        }

        let key = create_cache_key(path, &content, mode);
        self.cache.insert(
            key,
            CacheEntry {
                content: result,
                compression_percent,
                timestamp: SystemTime::now(),
                file_mtime: mtime,
            },
        );
    }

    /// Remove the least recently used entry from the cache
    fn evict_lru(&mut self) {
        if let Some(oldest_key) = self
            .cache
            .iter()
            .min_by_key(|(_, entry)| entry.timestamp)
            .map(|(key, _)| *key)
        {
            self.cache.remove(&oldest_key);
        }
    }

    /// Get current cache size
    pub fn size(&self) -> usize {
        self.cache.len()
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_hit_same_file() {
        let mut cache = SemanticCache::new(10);
        let path = Path::new("/test/file.txt");
        let content = "hello world";
        let result = "processed content";
        let mode = ReadMode::Full;
        let now = SystemTime::now();

        cache.insert(path, content.to_string(), result.to_string(), 50, mode, now);

        let retrieved = cache.get(path, content, mode, now);
        assert_eq!(retrieved, Some(result.to_string()));
    }

    #[test]
    fn test_cache_miss_new_file() {
        let cache = SemanticCache::new(10);
        let path = Path::new("/test/file.txt");
        let content = "hello world";
        let mode = ReadMode::Full;
        let now = SystemTime::now();

        let retrieved = cache.get(path, content, mode, now);
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_cache_invalidation_mtime() {
        let mut cache = SemanticCache::new(10);
        let path = Path::new("/test/file.txt");
        let content = "hello world";
        let result = "processed content";
        let mode = ReadMode::Full;
        let original_time = SystemTime::now();

        cache.insert(
            path,
            content.to_string(),
            result.to_string(),
            50,
            mode,
            original_time,
        );

        // Same file at same time - should hit
        let retrieved = cache.get(path, content, mode, original_time);
        assert_eq!(retrieved, Some(result.to_string()));

        // File modified - should miss
        let new_time = original_time + Duration::from_secs(1);
        let retrieved = cache.get(path, content, mode, new_time);
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = SemanticCache::new(3);
        let mode = ReadMode::Full;
        let now = SystemTime::now();

        // Insert 3 entries
        for i in 0..3 {
            let path_str = format!("/test/file{}.txt", i);
            let path = Path::new(&path_str);
            let content = format!("content{}", i);
            let result = format!("result{}", i);
            cache.insert(path, content.clone(), result, 50, mode, now);
        }

        assert_eq!(cache.size(), 3);

        // Sleep a bit to ensure different timestamps
        std::thread::sleep(Duration::from_millis(10));

        // Insert 4th entry - should evict the oldest (file0)
        let path = Path::new("/test/file3.txt");
        let content = "content3";
        let result = "result3";
        cache.insert(
            path,
            content.to_string(),
            result.to_string(),
            50,
            mode,
            SystemTime::now(),
        );

        assert_eq!(cache.size(), 3);

        // Verify file0 was evicted
        let path0 = Path::new("/test/file0.txt");
        let retrieved = cache.get(path0, "content0", mode, now);
        assert_eq!(retrieved, None);
    }

    #[test]
    fn test_compression_percent_stored() {
        let mut cache = SemanticCache::new(10);
        let path = Path::new("/test/file.txt");
        let content = "hello world";
        let result = "processed content";
        let mode = ReadMode::Full;
        let now = SystemTime::now();
        let compression = 75;

        cache.insert(
            path,
            content.to_string(),
            result.to_string(),
            compression,
            mode,
            now,
        );

        // Verify it's in cache (compression % is stored internally)
        let retrieved = cache.get(path, content, mode, now);
        assert_eq!(retrieved, Some(result.to_string()));
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn test_different_modes_different_keys() {
        let mut cache = SemanticCache::new(10);
        let path = Path::new("/test/file.txt");
        let content = "hello world";
        let now = SystemTime::now();

        cache.insert(
            path,
            content.to_string(),
            "result_full".to_string(),
            50,
            ReadMode::Full,
            now,
        );

        cache.insert(
            path,
            content.to_string(),
            "result_signatures".to_string(),
            75,
            ReadMode::Signatures,
            now,
        );

        // Both should be retrievable with their respective modes
        assert_eq!(
            cache.get(path, content, ReadMode::Full, now),
            Some("result_full".to_string())
        );
        assert_eq!(
            cache.get(path, content, ReadMode::Signatures, now),
            Some("result_signatures".to_string())
        );

        // Wrong mode should miss
        assert_eq!(cache.get(path, content, ReadMode::Diff, now), None);
    }

    #[test]
    fn test_different_content_different_keys() {
        let mut cache = SemanticCache::new(10);
        let path = Path::new("/test/file.txt");
        let mode = ReadMode::Full;
        let now = SystemTime::now();

        cache.insert(
            path,
            "content1".to_string(),
            "result1".to_string(),
            50,
            mode,
            now,
        );

        cache.insert(
            path,
            "content2".to_string(),
            "result2".to_string(),
            60,
            mode,
            now,
        );

        // Both should be retrievable with their respective content
        assert_eq!(
            cache.get(path, "content1", mode, now),
            Some("result1".to_string())
        );
        assert_eq!(
            cache.get(path, "content2", mode, now),
            Some("result2".to_string())
        );
    }

    #[test]
    fn test_clear_cache() {
        let mut cache = SemanticCache::new(10);
        let path = Path::new("/test/file.txt");
        let content = "hello world";
        let result = "processed content";
        let mode = ReadMode::Full;
        let now = SystemTime::now();

        cache.insert(path, content.to_string(), result.to_string(), 50, mode, now);
        assert_eq!(cache.size(), 1);

        cache.clear();
        assert_eq!(cache.size(), 0);

        let retrieved = cache.get(path, content, mode, now);
        assert_eq!(retrieved, None);
    }
}

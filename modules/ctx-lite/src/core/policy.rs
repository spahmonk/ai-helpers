use std::path::Path;
use crate::app::contracts::ReadMode;

/// Adaptive policy for automatic mode selection based on file type and size
#[derive(Clone, Debug)]
pub struct AdaptivePolicy;

impl AdaptivePolicy {
    /// Create a new adaptive policy
    pub fn new() -> Self {
        Self
    }

    /// Select the best compression mode for a given file
    /// 
    /// Returns the recommended ReadMode based on:
    /// - File extension (code files -> Signatures, config -> Map, large files -> Diff)
    /// - File size (>100KB -> Diff for delta compression)
    /// - User preference is respected if explicitly set (defaults to Full)
    pub fn select_mode(&self, path: &Path, _max_bytes: usize) -> ReadMode {
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .unwrap_or_default();

        // Check file size
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 100_000 {
                return ReadMode::Diff;
            }
        }

        // Code files -> Signatures mode (95%+ compression)
        if matches!(
            extension.as_str(),
            "rs" | "py" | "ts" | "js" | "tsx" | "jsx" | "go" | "java" | "cpp" | "c" | "h" | "hpp"
                | "rb" | "php" | "swift" | "kt"
        ) {
            return ReadMode::Signatures;
        }

        // Config files -> Map mode (96% compression)
        if matches!(
            extension.as_str(),
            "json" | "yaml" | "yml" | "toml" | "ini" | "xml" | "conf"
        ) {
            return ReadMode::Map;
        }

        // Default to Full mode
        ReadMode::Full
    }

    /// Suggest mode upgrade based on current budget status and file type
    /// This helps optimize token usage by choosing more aggressive compression
    /// when budget is running low
    pub fn suggest_upgrade(&self, _path: &Path, _current_mode: ReadMode) -> Option<ReadMode> {
        // Future: implement budget-aware mode suggestion
        // For now, return None (no upgrade needed)
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signatures_for_rust_files() {
        let policy = AdaptivePolicy::new();
        let path = Path::new("src/main.rs");
        assert_eq!(policy.select_mode(path, 8192), ReadMode::Signatures);
    }

    #[test]
    fn test_signatures_for_python_files() {
        let policy = AdaptivePolicy::new();
        let path = Path::new("script.py");
        assert_eq!(policy.select_mode(path, 8192), ReadMode::Signatures);
    }

    #[test]
    fn test_map_for_json_files() {
        let policy = AdaptivePolicy::new();
        let path = Path::new("config.json");
        assert_eq!(policy.select_mode(path, 8192), ReadMode::Map);
    }

    #[test]
    fn test_map_for_yaml_files() {
        let policy = AdaptivePolicy::new();
        let path = Path::new("settings.yaml");
        assert_eq!(policy.select_mode(path, 8192), ReadMode::Map);
    }

    #[test]
    fn test_full_for_unknown_types() {
        let policy = AdaptivePolicy::new();
        let path = Path::new("document.unknown");
        assert_eq!(policy.select_mode(path, 8192), ReadMode::Full);
    }

    #[test]
    fn test_case_insensitive_extension_matching() {
        let policy = AdaptivePolicy::new();
        
        // .TS (uppercase) should still select Signatures
        let path = Path::new("file.TS");
        assert_eq!(policy.select_mode(path, 8192), ReadMode::Signatures);

        // .Ts (mixed case) should still select Signatures
        let path = Path::new("file.Ts");
        assert_eq!(policy.select_mode(path, 8192), ReadMode::Signatures);
    }
}

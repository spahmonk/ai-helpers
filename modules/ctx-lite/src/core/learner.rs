/// ML-Based Mode Selection: Learns from compression results to optimize mode selection
///
/// Tracks compression effectiveness per file pattern and learns which modes work best
/// for different file types. Stores learned data persistently in ~/.ctx-lite/mode-learning.json
use crate::core::cache::ReadMode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Pattern for matching files (e.g., "*.rs", "*.json", "Makefile")
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct FilePattern {
    pub pattern: String,
}

impl FilePattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        FilePattern {
            pattern: pattern.into(),
        }
    }

    /// Match a filename against the pattern
    pub fn matches(&self, filename: &str) -> bool {
        let filename_lower = filename.to_lowercase();

        // Exact match
        if filename_lower == self.pattern.to_lowercase() {
            return true;
        }

        // Extension match (*.ext)
        if self.pattern.starts_with("*.") {
            let ext = self.pattern.strip_prefix("*.").unwrap_or("");
            return filename_lower.ends_with(&format!(".{}", ext));
        }

        // Wildcard at end (*pattern)
        if self.pattern.starts_with("*") && !self.pattern.contains("*") {
            let suffix = self.pattern.strip_prefix("*").unwrap_or("");
            return filename_lower.ends_with(suffix);
        }

        // Wildcard at start (pattern*)
        if self.pattern.ends_with("*") && !self.pattern.starts_with("*") {
            let prefix = self.pattern.strip_suffix("*").unwrap_or("");
            return filename_lower.starts_with(prefix);
        }

        false
    }
}

/// Learning record for a specific mode
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModeRecord {
    pub mode: String,
    pub successes: usize,        // Count of reads with compression >= threshold
    pub failures: usize,         // Count of reads with compression < threshold
    pub avg_compression: f32,    // Average compression percent across all reads
    pub best_compression: usize, // Best compression achieved
}

impl ModeRecord {
    pub fn success_rate(&self) -> f32 {
        if self.successes + self.failures == 0 {
            0.0
        } else {
            (self.successes as f32) / ((self.successes + self.failures) as f32)
        }
    }

    pub fn total_attempts(&self) -> usize {
        self.successes + self.failures
    }
}

/// Pattern learning data: tracks all modes tried for a pattern
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternLearning {
    pub pattern: String,
    pub modes: HashMap<String, ModeRecord>,
    pub best_mode: Option<String>, // Best mode so far
}

impl PatternLearning {
    pub fn new(pattern: String) -> Self {
        PatternLearning {
            pattern,
            modes: HashMap::new(),
            best_mode: None,
        }
    }

    /// Get the recommended mode based on learning history
    pub fn get_recommended_mode(&self) -> Option<ReadMode> {
        let best_mode_str = self.best_mode.as_ref()?;
        match best_mode_str.as_str() {
            "full" => Some(ReadMode::Full),
            "signatures" => Some(ReadMode::Signatures),
            "map" => Some(ReadMode::Map),
            "diff" => Some(ReadMode::Diff),
            _ => None,
        }
    }
}

/// ML-Based learner that adapts mode selection based on actual compression results
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModeLearner {
    pub patterns: HashMap<String, PatternLearning>,
    pub compression_threshold: usize, // e.g., 60% - below this, try next mode
}

impl ModeLearner {
    pub fn new(compression_threshold: usize) -> Self {
        ModeLearner {
            patterns: HashMap::new(),
            compression_threshold,
        }
    }

    /// Load learner from persistent storage
    pub fn load() -> Self {
        let path = Self::learner_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(learner) = serde_json::from_str(&content) {
                    return learner;
                }
            }
        }
        ModeLearner::new(60)
    }

    /// Save learner to persistent storage
    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::learner_path();

        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(path, json)
    }

    /// Get path to learner storage file
    fn learner_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home)
            .join(".ctx-lite")
            .join("mode-learning.json")
    }

    /// Learn from a compression result
    ///
    /// # Arguments
    /// * `filename` - The filename to learn from
    /// * `mode` - The mode that was used
    /// * `compression_percent` - The actual compression achieved
    pub fn learn_mode(&mut self, filename: &str, mode: ReadMode, compression_percent: usize) {
        let pattern = Self::extract_pattern(filename);
        let pattern_str = pattern.pattern.clone();
        let mode_str = mode.as_str().to_string();

        // Ensure pattern exists
        self.patterns
            .entry(pattern_str.clone())
            .or_insert_with(|| PatternLearning::new(pattern_str.clone()));

        let pattern_learning = self.patterns.get_mut(&pattern_str).unwrap();

        // Ensure mode record exists
        pattern_learning
            .modes
            .entry(mode_str.clone())
            .or_insert_with(|| ModeRecord {
                mode: mode_str.clone(),
                successes: 0,
                failures: 0,
                avg_compression: 0.0,
                best_compression: 0,
            });

        let mode_record = pattern_learning.modes.get_mut(&mode_str).unwrap();

        // Update compression stats
        let old_total = (mode_record.successes + mode_record.failures) as f32;
        let old_avg = mode_record.avg_compression;

        if compression_percent >= self.compression_threshold {
            mode_record.successes += 1;
        } else {
            mode_record.failures += 1;
        }

        // Calculate new average
        let new_total = (mode_record.successes + mode_record.failures) as f32;
        mode_record.avg_compression =
            (old_avg * old_total + compression_percent as f32) / new_total;

        // Track best compression
        mode_record.best_compression = mode_record.best_compression.max(compression_percent);

        // Update best mode for pattern (highest success rate, then highest avg compression)
        let new_best = pattern_learning
            .modes
            .values()
            .max_by(|a, b| {
                let cmp = a.success_rate().partial_cmp(&b.success_rate()).unwrap();
                if cmp == std::cmp::Ordering::Equal {
                    a.avg_compression.partial_cmp(&b.avg_compression).unwrap()
                } else {
                    cmp
                }
            })
            .map(|r| r.mode.clone());

        pattern_learning.best_mode = new_best;
    }

    /// Get the recommended mode for a filename based on learning
    pub fn get_recommended_mode(&self, filename: &str) -> Option<ReadMode> {
        let ext = Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Try exact extension pattern first
        let ext_pattern = format!("*.{}", ext);
        if let Some(pattern_learning) = self.patterns.get(&ext_pattern) {
            if let Some(mode) = pattern_learning.get_recommended_mode() {
                // Only return if we have enough learning (at least 3 attempts)
                if pattern_learning
                    .modes
                    .values()
                    .map(|m| m.total_attempts())
                    .sum::<usize>()
                    >= 3
                {
                    return Some(mode);
                }
            }
        }

        None
    }

    /// Extract a pattern from a filename
    fn extract_pattern(filename: &str) -> FilePattern {
        let path = Path::new(filename);

        // Get extension
        if let Some(ext) = path.extension() {
            if let Some(ext_str) = ext.to_str() {
                return FilePattern::new(format!("*.{}", ext_str.to_lowercase()));
            }
        }

        // Fallback to filename
        FilePattern::new(filename.to_lowercase())
    }

    /// Clear all learning data (useful for testing)
    pub fn clear(&mut self) {
        self.patterns.clear();
    }

    /// Get statistics about learning
    pub fn stats(&self) -> LearnStats {
        let total_patterns = self.patterns.len();
        let total_attempts: usize = self
            .patterns
            .values()
            .flat_map(|p| p.modes.values())
            .map(|m| m.total_attempts())
            .sum();
        let patterns_with_best: usize = self
            .patterns
            .values()
            .filter(|p| p.best_mode.is_some())
            .count();

        LearnStats {
            total_patterns,
            total_attempts,
            patterns_with_best_mode: patterns_with_best,
        }
    }
}

#[derive(Clone, Debug)]
pub struct LearnStats {
    pub total_patterns: usize,
    pub total_attempts: usize,
    pub patterns_with_best_mode: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_pattern_matches_extension() {
        let pattern = FilePattern::new("*.rs");
        assert!(pattern.matches("main.rs"));
        assert!(pattern.matches("lib.rs"));
        assert!(pattern.matches("test.RS")); // Case insensitive
        assert!(!pattern.matches("main.py"));
        assert!(!pattern.matches("rs")); // No extension
    }

    #[test]
    fn test_file_pattern_exact_match() {
        let pattern = FilePattern::new("Makefile");
        assert!(pattern.matches("Makefile"));
        assert!(pattern.matches("makefile")); // Case insensitive
        assert!(!pattern.matches("Makefile.bak"));
    }

    #[test]
    fn test_mode_record_success_rate() {
        let record = ModeRecord {
            mode: "signatures".to_string(),
            successes: 7,
            failures: 3,
            avg_compression: 75.0,
            best_compression: 85,
        };

        assert_eq!(record.success_rate(), 0.7);
        assert_eq!(record.total_attempts(), 10);
    }

    #[test]
    fn test_mode_record_success_rate_empty() {
        let record = ModeRecord {
            mode: "full".to_string(),
            successes: 0,
            failures: 0,
            avg_compression: 0.0,
            best_compression: 0,
        };

        assert_eq!(record.success_rate(), 0.0);
        assert_eq!(record.total_attempts(), 0);
    }

    #[test]
    fn test_mode_learner_learn_mode_success() {
        let mut learner = ModeLearner::new(60);

        // Learn successful compressions
        learner.learn_mode("main.rs", ReadMode::Signatures, 75);
        learner.learn_mode("main.rs", ReadMode::Signatures, 80);

        let pattern_learning = learner.patterns.get("*.rs").unwrap();
        let sig_record = pattern_learning.modes.get("signatures").unwrap();

        assert_eq!(sig_record.successes, 2);
        assert_eq!(sig_record.failures, 0);
        assert!(sig_record.avg_compression > 74.0 && sig_record.avg_compression < 80.1);
        assert_eq!(sig_record.best_compression, 80);
    }

    #[test]
    fn test_mode_learner_learn_mode_failure() {
        let mut learner = ModeLearner::new(60);

        // Learn failed compressions (below threshold)
        learner.learn_mode("data.bin", ReadMode::Full, 30);
        learner.learn_mode("data.bin", ReadMode::Full, 40);

        let pattern_learning = learner.patterns.get("*.bin").unwrap();
        let full_record = pattern_learning.modes.get("full").unwrap();

        assert_eq!(full_record.successes, 0);
        assert_eq!(full_record.failures, 2);
    }

    #[test]
    fn test_mode_learner_best_mode_selection() {
        let mut learner = ModeLearner::new(60);

        // Signatures performs well
        learner.learn_mode("main.rs", ReadMode::Signatures, 85);
        learner.learn_mode("main.rs", ReadMode::Signatures, 80);
        learner.learn_mode("main.rs", ReadMode::Signatures, 75);

        // Map performs poorly
        learner.learn_mode("main.rs", ReadMode::Map, 30);
        learner.learn_mode("main.rs", ReadMode::Map, 40);

        let pattern_learning = learner.patterns.get("*.rs").unwrap();
        assert_eq!(pattern_learning.best_mode, Some("signatures".to_string()));
    }

    #[test]
    fn test_mode_learner_get_recommended_mode() {
        let mut learner = ModeLearner::new(60);

        // Not enough learning yet
        assert_eq!(learner.get_recommended_mode("main.rs"), None);

        // After 3+ attempts, should return best mode
        learner.learn_mode("main.rs", ReadMode::Signatures, 85);
        learner.learn_mode("app.rs", ReadMode::Signatures, 80);
        learner.learn_mode("lib.rs", ReadMode::Signatures, 75);

        let recommended = learner.get_recommended_mode("test.rs");
        assert_eq!(recommended, Some(ReadMode::Signatures));
    }

    #[test]
    fn test_mode_learner_multiple_patterns() {
        let mut learner = ModeLearner::new(60);

        // Learn different patterns
        learner.learn_mode("config.json", ReadMode::Map, 90);
        learner.learn_mode("config.json", ReadMode::Map, 85);
        learner.learn_mode("config.json", ReadMode::Map, 80);

        learner.learn_mode("script.sh", ReadMode::Signatures, 88);
        learner.learn_mode("script.sh", ReadMode::Signatures, 82);
        learner.learn_mode("script.sh", ReadMode::Signatures, 78);

        assert_eq!(
            learner.get_recommended_mode("settings.json"),
            Some(ReadMode::Map)
        );
        assert_eq!(
            learner.get_recommended_mode("deploy.sh"),
            Some(ReadMode::Signatures)
        );
    }

    #[test]
    fn test_mode_learner_learning_improves_over_time() {
        let mut learner = ModeLearner::new(60);

        // Initial learning - Diff performs better than Signatures for large file
        learner.learn_mode("large_file.bin", ReadMode::Signatures, 50); // Below threshold
        learner.learn_mode("large_file.bin", ReadMode::Diff, 95); // Above threshold
        learner.learn_mode("large_file.bin", ReadMode::Diff, 94);
        learner.learn_mode("large_file.bin", ReadMode::Diff, 93);

        // After learning, should recommend Diff (highest success rate)
        let recommended = learner.get_recommended_mode("other_large_file.bin");
        assert_eq!(recommended, Some(ReadMode::Diff));
    }

    #[test]
    fn test_mode_learner_stats() {
        let mut learner = ModeLearner::new(60);

        learner.learn_mode("main.rs", ReadMode::Signatures, 80);
        learner.learn_mode("main.rs", ReadMode::Signatures, 75);
        learner.learn_mode("config.json", ReadMode::Map, 85);

        let stats = learner.stats();
        assert_eq!(stats.total_patterns, 2); // *.rs and *.json
        assert_eq!(stats.total_attempts, 3);
        assert_eq!(stats.patterns_with_best_mode, 2);
    }
}

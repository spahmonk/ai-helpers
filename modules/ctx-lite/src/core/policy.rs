use std::path::Path;
use crate::core::cache::ReadMode;
use crate::core::learner::ModeLearner;

/// Adaptive policy for selecting the best compression mode per file type.
///
/// The policy considers:
/// - User preference (always wins if specified)
/// - Learned patterns (if enough data from ModeLearner)
/// - File extension (code/config/other)
/// - File size (large files benefit from diff mode)
/// - Budget constraints (suggests upgrading to more aggressive compression when needed)
pub struct AdaptivePolicy;

impl AdaptivePolicy {
    /// Select the optimal read mode for a given file.
    ///
    /// # Arguments
    /// * `path` - The file path to analyze
    /// * `file_size` - The size of the file in bytes
    /// * `user_preference` - Optional user-specified mode override
    ///
    /// # Returns
    /// The recommended ReadMode
    ///
    /// # Mode Selection Logic
    /// - Code files (.rs, .py, .ts, etc.): Signatures (95%+ compression with function/class signatures)
    /// - Config files (.json, .yaml, .toml, etc.): Map (96% compression with key-value structure)
    /// - Large files (>100KB): Diff (99% compression on re-reads)
    /// - All others: Full (uncompressed)
    pub fn select_mode(
        path: &Path,
        file_size: usize,
        user_preference: Option<ReadMode>,
    ) -> ReadMode {
        // User preference always takes priority
        if let Some(mode) = user_preference {
            return mode;
        }

        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        // Code files: use signatures for consistent compression
        if matches!(
            ext.as_str(),
            "rs" | "py" | "ts" | "js" | "go" | "java" | "cpp" | "c" | "h" | "hpp" | "cc"
                | "cxx" | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bash"
        ) {
            return ReadMode::Signatures;
        }

        // Config and data files: use map for structure preservation
        if matches!(
            ext.as_str(),
            "json" | "yaml" | "yml" | "toml" | "xml" | "csv" | "ini" | "conf" | "config"
        ) {
            return ReadMode::Map;
        }

        // Large files: use diff for best compression on re-reads
        if file_size > 100_000 {
            return ReadMode::Diff;
        }

        // Default: full content
        ReadMode::Full
    }

    /// Suggest a higher compression level when budget is running low.
    ///
    /// # Arguments
    /// * `budget_remaining_percent` - Percentage of budget remaining (0.0 to 1.0)
    ///
    /// # Returns
    /// Some(ReadMode) if an upgrade is suggested, None otherwise
    ///
    /// # Upgrade Strategy
    /// - <20% budget: suggest Diff (most aggressive)
    /// - <40% budget: suggest Map (moderate compression)
    /// - >=40% budget: no upgrade needed
    pub fn suggest_upgrade(budget_remaining_percent: f32) -> Option<ReadMode> {
        match budget_remaining_percent {
            p if p < 0.2 => Some(ReadMode::Diff),
            p if p < 0.4 => Some(ReadMode::Map),
            _ => None,
        }
    }

    /// Select mode with ML-based learning integration.
    ///
    /// This method combines the static heuristics with learned patterns from ModeLearner.
    /// First checks if ModeLearner has recommendations, then falls back to static heuristics.
    ///
    /// # Arguments
    /// * `path` - The file path to analyze
    /// * `file_size` - The size of the file in bytes
    /// * `user_preference` - Optional user-specified mode override
    /// * `learner` - Reference to the ModeLearner for getting learned recommendations
    ///
    /// # Returns
    /// The recommended ReadMode
    pub fn select_mode_with_learning(
        path: &Path,
        file_size: usize,
        user_preference: Option<ReadMode>,
        learner: &ModeLearner,
    ) -> ReadMode {
        // User preference always takes priority
        if let Some(mode) = user_preference {
            return mode;
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Check if learner has a recommendation (at least 3 samples)
        if let Some(learned_mode) = learner.get_recommended_mode(filename) {
            return learned_mode;
        }

        // Fall back to static heuristics
        Self::select_mode(path, file_size, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_code_files_use_signatures() {
        let code_files = vec!["main.rs", "app.py", "index.ts", "app.js", "main.go"];

        for filename in code_files {
            let path = PathBuf::from(filename);
            let mode = AdaptivePolicy::select_mode(&path, 1000, None);
            assert_eq!(
                mode.as_str(),
                "signatures",
                "File {} should use Signatures mode",
                filename
            );
        }
    }

    #[test]
    fn test_config_files_use_map() {
        let config_files = vec!["config.json", "settings.yaml", "app.yml", "Cargo.toml"];

        for filename in config_files {
            let path = PathBuf::from(filename);
            let mode = AdaptivePolicy::select_mode(&path, 1000, None);
            assert_eq!(
                mode.as_str(),
                "map",
                "File {} should use Map mode",
                filename
            );
        }
    }

    #[test]
    fn test_large_files_use_diff() {
        let path = PathBuf::from("large_data.bin");
        let large_size = 150_000; // 150KB > 100KB threshold

        let mode = AdaptivePolicy::select_mode(&path, large_size, None);
        assert_eq!(
            mode.as_str(),
            "diff",
            "Large files (>100KB) should use Diff mode"
        );
    }

    #[test]
    fn test_unknown_files_full_mode() {
        let path = PathBuf::from("document.txt");
        let mode = AdaptivePolicy::select_mode(&path, 5000, None);
        assert_eq!(
            mode.as_str(),
            "full",
            "Unknown file types should default to Full mode"
        );
    }

    #[test]
    fn test_user_preference_override() {
        let path = PathBuf::from("main.rs"); // Would normally be Signatures
        let mode = AdaptivePolicy::select_mode(&path, 1000, Some(ReadMode::Full));

        assert_eq!(
            mode.as_str(),
            "full",
            "User preference should override adaptive selection"
        );
    }

    #[test]
    fn test_user_preference_overrides_large_file() {
        let path = PathBuf::from("data.bin");
        let mode = AdaptivePolicy::select_mode(&path, 500_000, Some(ReadMode::Map));

        assert_eq!(
            mode.as_str(),
            "map",
            "User preference should override large file heuristic"
        );
    }

    #[test]
    fn test_budget_suggestion_low_budget() {
        let suggestion = AdaptivePolicy::suggest_upgrade(0.15); // 15% budget
        assert_eq!(
            suggestion,
            Some(ReadMode::Diff),
            "Should suggest Diff mode when budget is very low"
        );
    }

    #[test]
    fn test_budget_suggestion_moderate_budget() {
        let suggestion = AdaptivePolicy::suggest_upgrade(0.30); // 30% budget
        assert_eq!(
            suggestion,
            Some(ReadMode::Map),
            "Should suggest Map mode when budget is moderate"
        );
    }

    #[test]
    fn test_budget_suggestion_plenty_budget() {
        let suggestion = AdaptivePolicy::suggest_upgrade(0.50); // 50% budget
        assert_eq!(
            suggestion, None,
            "Should not suggest upgrade when budget is sufficient"
        );
    }

    #[test]
    fn test_budget_suggestion_edge_case_20_percent() {
        let suggestion = AdaptivePolicy::suggest_upgrade(0.20); // Exactly 20%
        assert_eq!(
            suggestion,
            Some(ReadMode::Map),
            "At 20% boundary, should suggest Map"
        );
    }

    #[test]
    fn test_budget_suggestion_edge_case_40_percent() {
        let suggestion = AdaptivePolicy::suggest_upgrade(0.40); // Exactly 40%
        assert_eq!(
            suggestion, None,
            "At 40% boundary, should not suggest upgrade"
        );
    }

    #[test]
    fn test_extension_case_insensitive() {
        let path_upper = PathBuf::from("MAIN.RS");
        let path_lower = PathBuf::from("main.rs");
        let path_mixed = PathBuf::from("Main.Rs");

        let mode_upper = AdaptivePolicy::select_mode(&path_upper, 1000, None);
        let mode_lower = AdaptivePolicy::select_mode(&path_lower, 1000, None);
        let mode_mixed = AdaptivePolicy::select_mode(&path_mixed, 1000, None);

        assert_eq!(mode_upper.as_str(), "signatures");
        assert_eq!(mode_lower.as_str(), "signatures");
        assert_eq!(mode_mixed.as_str(), "signatures");
    }

    #[test]
    fn test_no_extension() {
        let path = PathBuf::from("Makefile");
        let mode = AdaptivePolicy::select_mode(&path, 1000, None);
        assert_eq!(
            mode.as_str(),
            "full",
            "Files without extensions should use Full mode"
        );
    }

    #[test]
    fn test_file_size_boundary() {
        let path = PathBuf::from("data.bin");

        // Just below threshold
        let mode_below = AdaptivePolicy::select_mode(&path, 99_999, None);
        assert_eq!(mode_below.as_str(), "full");

        // Just at threshold
        let mode_at = AdaptivePolicy::select_mode(&path, 100_000, None);
        assert_eq!(mode_at.as_str(), "full");

        // Just above threshold
        let mode_above = AdaptivePolicy::select_mode(&path, 100_001, None);
        assert_eq!(mode_above.as_str(), "diff");
    }

    #[test]
    fn test_java_cpp_extensions() {
        let java_path = PathBuf::from("Main.java");
        let cpp_path = PathBuf::from("app.cpp");

        let java_mode = AdaptivePolicy::select_mode(&java_path, 1000, None);
        let cpp_mode = AdaptivePolicy::select_mode(&cpp_path, 1000, None);

        assert_eq!(java_mode.as_str(), "signatures");
        assert_eq!(cpp_mode.as_str(), "signatures");
    }

    #[test]
    fn test_shell_scripts() {
        let bash_path = PathBuf::from("deploy.bash");
        let sh_path = PathBuf::from("setup.sh");

        let bash_mode = AdaptivePolicy::select_mode(&bash_path, 5000, None);
        let sh_mode = AdaptivePolicy::select_mode(&sh_path, 3000, None);

        assert_eq!(bash_mode.as_str(), "signatures");
        assert_eq!(sh_mode.as_str(), "signatures");
    }

    #[test]
    fn test_select_mode_with_learning_no_learning_data() {
        let learner = ModeLearner::new(60);
        let path = PathBuf::from("main.rs");

        let mode = AdaptivePolicy::select_mode_with_learning(&path, 1000, None, &learner);

        // Should fall back to static heuristics
        assert_eq!(mode.as_str(), "signatures");
    }

    #[test]
    fn test_select_mode_with_learning_uses_learned_mode() {
        let mut learner = ModeLearner::new(60);

        // Learn that large files should use Diff
        learner.learn_mode("large_file_1.bin", ReadMode::Diff, 92);
        learner.learn_mode("large_file_2.bin", ReadMode::Diff, 90);
        learner.learn_mode("large_file_3.bin", ReadMode::Diff, 91);

        let path = PathBuf::from("other_large_file.bin");

        // Should use learned Diff mode instead of Full mode
        let mode = AdaptivePolicy::select_mode_with_learning(&path, 50_000, None, &learner);
        assert_eq!(mode.as_str(), "diff");
    }

    #[test]
    fn test_select_mode_with_learning_user_preference_priority() {
        let mut learner = ModeLearner::new(60);

        // Learn that .rs files should use Signatures
        learner.learn_mode("main.rs", ReadMode::Signatures, 80);
        learner.learn_mode("app.rs", ReadMode::Signatures, 75);
        learner.learn_mode("lib.rs", ReadMode::Signatures, 78);

        let path = PathBuf::from("test.rs");

        // Even with learning, user preference should win
        let mode = AdaptivePolicy::select_mode_with_learning(&path, 1000, Some(ReadMode::Full), &learner);
        assert_eq!(mode.as_str(), "full");
    }

    #[test]
    fn test_select_mode_with_learning_insufficient_samples() {
        let mut learner = ModeLearner::new(60);

        // Only 2 samples - not enough for recommendation
        learner.learn_mode("main.rs", ReadMode::Signatures, 80);
        learner.learn_mode("app.rs", ReadMode::Signatures, 75);

        let path = PathBuf::from("test.rs");

        // Should fall back to static heuristics (not enough learning data)
        let mode = AdaptivePolicy::select_mode_with_learning(&path, 1000, None, &learner);
        assert_eq!(mode.as_str(), "signatures");
    }
}

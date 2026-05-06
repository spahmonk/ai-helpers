use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use regex::Regex;

use crate::app::contracts::{SearchHit, SearchRequestNormalized, SearchResponse, ServiceError};

use crate::core::security::path_jail::PathJail;

/// Maximum bytes to read per file during search
const MAX_SEARCH_FILE_SIZE: usize = 1_048_576; // 1 MB

#[derive(Clone, Debug)]
pub struct SearchService {
    jail: PathJail,
}

impl SearchService {
    pub fn new(jail: PathJail) -> Self {
        Self { jail }
    }

    pub fn search(&self, request: SearchRequestNormalized) -> Result<SearchResponse, ServiceError> {
        // Validate the query is not empty
        if request.query.is_empty() {
            return Err(ServiceError::unsupported("search query cannot be empty"));
        }

        // Compile the search pattern (regex or literal)
        let regex = self.compile_search_pattern(&request.query)?;

        let mut hits = Vec::new();

        // Search all files rooted at request.path (jail-validated)
        for entry in self.list_files(&request.path)? {
            if hits.len() >= request.limit {
                break;
            }

            match self.search_file(&entry, &regex) {
                Ok((file_hits, _file_bytes)) => {
                    for hit in file_hits {
                        if hits.len() < request.limit {
                            hits.push(hit);
                        } else {
                            break;
                        }
                    }
                }
                Err(_) => {
                    // Skip files that can't be read
                    continue;
                }
            }
        }

        Ok(SearchResponse {
            query: request.query,
            hits,
        })
    }

    /// Compile search pattern as regex, with fallback to literal search
    fn compile_search_pattern(&self, query: &str) -> Result<Regex, ServiceError> {
        match Regex::new(query) {
            Ok(re) => Ok(re),
            Err(_) => {
                // Fall back to literal search by escaping the query
                let escaped = regex::escape(query);
                Regex::new(&escaped).map_err(|e| {
                    ServiceError::unsupported(format!("failed to compile search pattern: {}", e))
                })
            }
        }
    }

    /// List all files within a subtree, validated against the jail
    fn list_files(&self, root: &std::path::Path) -> Result<Vec<PathBuf>, ServiceError> {
        let mut files = Vec::new();
        self.walk_directory(root, &mut files)?;
        Ok(files)
    }

    /// Recursively walk directory to find files
    fn walk_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), ServiceError> {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        let path = entry.path();
                        if metadata.is_file() {
                            // Verify the file is within the jail before adding
                            if self.jail.resolve(&path).is_ok() {
                                files.push(path);
                            }
                        } else if metadata.is_dir() {
                            // Recursively walk subdirectories
                            let _ = self.walk_directory(&path, files);
                        }
                    }
                }
            }
            Err(_) => {
                // Skip directories we can't read
            }
        }
        Ok(())
    }

    /// Search a single file for matches
    fn search_file(
        &self,
        file_path: &Path,
        regex: &Regex,
    ) -> Result<(Vec<SearchHit>, usize), ServiceError> {
        let resolved = self
            .jail
            .resolve(file_path)
            .map_err(|e| ServiceError::unsupported(e.message))?;

        let mut file = resolved
            .open_file()
            .map_err(|e| ServiceError::unsupported(e.message))?;

        let (content, bytes_read) = read_utf8_prefix(&mut file, MAX_SEARCH_FILE_SIZE)?;

        let mut hits = Vec::new();
        for (line_number, line) in content.lines().enumerate() {
            if regex.is_match(line) {
                hits.push(SearchHit {
                    path: resolved.path().to_path_buf(),
                    line_number: line_number + 1,
                    line: line.to_string(),
                });
            }
        }

        Ok((hits, bytes_read))
    }
}

/// Read file content up to a size limit, ensuring UTF-8 validity
fn read_utf8_prefix(
    file: &mut dyn Read,
    max_bytes: usize,
) -> Result<(String, usize), ServiceError> {
    let mut buf = vec![0u8; max_bytes];
    let bytes_read = file
        .read(&mut buf)
        .map_err(|_| ServiceError::unsupported("failed to read file".to_string()))?;

    buf.truncate(bytes_read);

    // Convert to string, attempting to truncate at valid UTF-8 boundaries
    match String::from_utf8(buf) {
        Ok(content) => Ok((content, bytes_read)),
        Err(e) => {
            // Try to recover by truncating at the last valid UTF-8 boundary
            let valid_up_to = e.utf8_error().valid_up_to();
            let valid_bytes = e.into_bytes();
            let truncated = String::from_utf8_lossy(&valid_bytes[..valid_up_to]).into_owned();
            Ok((truncated, valid_up_to))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::config::AppConfig;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn setup_test_dir() -> (TempDir, PathBuf) {
        let temp = TempDir::new().expect("failed to create temp dir");
        // Canonicalize to handle symlinks on macOS (e.g., /tmp -> /private/tmp)
        let root = temp
            .path()
            .canonicalize()
            .expect("failed to canonicalize temp dir");

        // Create test files
        let file1 = root.join("test1.txt");
        let mut f = File::create(&file1).expect("failed to create file1");
        writeln!(f, "hello world").expect("failed to write to file1");
        writeln!(f, "foo bar").expect("failed to write to file1");
        writeln!(f, "hello again").expect("failed to write to file1");

        let file2 = root.join("test2.txt");
        let mut f = File::create(&file2).expect("failed to create file2");
        writeln!(f, "another line").expect("failed to write to file2");
        writeln!(f, "hello there").expect("failed to write to file2");

        (temp, root)
    }

    #[test]
    fn search_finds_regex_matches_in_files() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "hello".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let response = service.search(request).expect("search failed");

        assert_eq!(response.query, "hello");
        assert!(!response.hits.is_empty());
        assert!(response.hits.iter().any(|h| h.line.contains("hello")));
    }

    #[test]
    fn search_respects_result_count_limit() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "hello".to_string(),
            limit: 2,
            path: root.clone(),
        };

        let response = service.search(request).expect("search failed");

        assert!(response.hits.len() <= 2);
    }

    #[test]
    fn search_returns_correct_line_numbers() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "foo bar".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let response = service.search(request).expect("search failed");

        assert!(!response.hits.is_empty());
        // "foo bar" is on line 2 of test1.txt
        assert!(response
            .hits
            .iter()
            .any(|h| h.line_number == 2 && h.line == "foo bar"));
    }

    #[test]
    fn search_handles_empty_results_gracefully() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "nonexistent_pattern_xyz".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let response = service.search(request).expect("search failed");

        assert_eq!(response.query, "nonexistent_pattern_xyz");
        assert!(response.hits.is_empty());
    }

    #[test]
    fn search_rejects_empty_query() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let result = service.search(request);
        assert!(result.is_err());
    }

    #[test]
    fn search_supports_literal_text_mode() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        // Search for "hello world" as literal text
        let request = SearchRequestNormalized {
            query: "hello world".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let response = service.search(request).expect("search failed");

        assert!(!response.hits.is_empty());
        assert!(response.hits.iter().any(|h| h.line == "hello world"));
    }

    #[test]
    fn search_falls_back_to_literal_matching_for_invalid_regex() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let literal_path = root.join("literal.txt");
        let mut file = File::create(&literal_path).expect("failed to create literal file");
        writeln!(file, "literal [ bracket").expect("failed to write literal file");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "[".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let response = service
            .search(request)
            .expect("invalid regex should fall back to literal search");

        assert!(response.hits.iter().any(|hit| hit.line.contains('[')));
    }

    #[test]
    fn search_returns_stable_response_format() {
        let (_temp, root) = setup_test_dir();
        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "hello".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let response = service.search(request).expect("search failed");

        // Verify response structure
        assert!(!response.hits.is_empty());
        for hit in &response.hits {
            assert!(!hit.path.as_os_str().is_empty());
            assert!(hit.line_number > 0);
            assert!(!hit.line.is_empty());
        }
    }

    #[test]
    fn search_respects_path_jail_containment() {
        let temp = TempDir::new().expect("failed to create temp dir");
        // Canonicalize to handle symlinks on macOS (e.g., /tmp -> /private/tmp)
        let root = temp
            .path()
            .canonicalize()
            .expect("failed to canonicalize root");

        // Create a file in the allowed root
        let allowed_file = root.join("allowed.txt");
        let mut f = File::create(&allowed_file).expect("failed to create file");
        writeln!(f, "this should be found").expect("failed to write");

        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);
        let request = SearchRequestNormalized {
            query: "should be found".to_string(),
            limit: 100,
            path: root.clone(),
        };

        let response = service.search(request).expect("search failed");

        // Should find the file
        assert!(!response.hits.is_empty());
        // All results should be within the allowed root
        for hit in &response.hits {
            assert!(hit.path.starts_with(&root));
        }
    }

    #[test]
    fn search_scopes_results_to_provided_path() {
        let temp = TempDir::new().expect("failed to create temp dir");
        let root = temp
            .path()
            .canonicalize()
            .expect("failed to canonicalize root");

        // Content only in root (NOT in subdir)
        let root_file = root.join("root_only.txt");
        let mut f = File::create(&root_file).expect("failed to create root file");
        writeln!(f, "ROOT_UNIQUE_CONTENT").expect("failed to write");

        // Create subdir with its own content
        let subdir = root.join("subdir");
        std::fs::create_dir(&subdir).expect("failed to create subdir");
        let sub_file = subdir.join("sub.txt");
        let mut f = File::create(&sub_file).expect("failed to create sub file");
        writeln!(f, "SUB_CONTENT").expect("failed to write");

        let jail = PathJail::from_config(&AppConfig {
            project_root: root.clone(),
            allowed_roots: vec![root.clone()],
            ..Default::default()
        })
        .expect("failed to create jail");

        let service = SearchService::new(jail);

        // Search for ROOT_UNIQUE_CONTENT but scope to subdir — should find nothing
        let request = SearchRequestNormalized {
            query: "ROOT_UNIQUE_CONTENT".to_string(),
            limit: 100,
            path: subdir.clone(),
        };
        let response = service.search(request).expect("search failed");
        assert!(
            response.hits.is_empty(),
            "search scoped to subdir must not return hits from root: {:?}",
            response.hits
        );

        // Search in root — should find it
        let request_root = SearchRequestNormalized {
            query: "ROOT_UNIQUE_CONTENT".to_string(),
            limit: 100,
            path: root.clone(),
        };
        let response_root = service.search(request_root).expect("search failed");
        assert!(!response_root.hits.is_empty(), "search from root should find ROOT_UNIQUE_CONTENT");
    }
}

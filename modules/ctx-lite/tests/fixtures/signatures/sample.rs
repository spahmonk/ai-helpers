/// A comprehensive sample Rust module for signature extraction testing
use std::fs::File;
use std::io::{Read, Write, Result};
use serde::{Serialize, Deserialize};

/// Configuration structure for application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    name: String,
    version: String,
    debug: bool,
}

impl Config {
    /// Create a new configuration with default values
    pub fn new(name: String, version: String) -> Self {
        Config {
            name,
            version,
            debug: false,
        }
    }

    /// Enable debug mode for verbose logging
    pub fn set_debug(&mut self, enabled: bool) {
        self.debug = enabled;
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Name cannot be empty"
            ));
        }
        Ok(())
    }
}

/// File processor trait for handling different file types
pub trait FileProcessor {
    fn process(&self, path: &str) -> Result<String>;
    fn supports(&self, extension: &str) -> bool;
}

/// Concrete implementation for text files
pub struct TextFileProcessor;

impl FileProcessor for TextFileProcessor {
    fn process(&self, path: &str) -> Result<String> {
        let mut file = File::open(path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        Ok(contents)
    }

    fn supports(&self, extension: &str) -> bool {
        matches!(extension, "txt" | "md" | "log")
    }
}

/// Main application handler
pub struct AppHandler {
    config: Config,
    processors: Vec<Box<dyn FileProcessor>>,
}

impl AppHandler {
    /// Initialize the application handler
    pub fn new(config: Config) -> Self {
        AppHandler {
            config,
            processors: Vec::new(),
        }
    }

    /// Register a new file processor
    pub fn register_processor(&mut self, processor: Box<dyn FileProcessor>) {
        self.processors.push(processor);
    }

    /// Process a file with appropriate handler
    pub fn handle_file(&self, path: &str) -> Result<String> {
        let extension = path.split('.').last().unwrap_or("");
        for processor in &self.processors {
            if processor.supports(extension) {
                return processor.process(path);
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "No processor for this file type"
        ))
    }

    /// Get configuration reference
    pub fn get_config(&self) -> &Config {
        &self.config
    }
}

/// Utility function for encoding strings
pub fn encode_string(input: &str) -> String {
    input
        .chars()
        .map(|c| format!("{:x}", c as u32))
        .collect::<Vec<_>>()
        .join("-")
}

/// Async utility for processing large files
pub async fn process_large_file_async(path: &str) -> Result<usize> {
    let mut file = File::open(path)?;
    let mut buffer = vec![0; 8192];
    let mut total = 0;
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 { break; }
        total += n;
    }
    Ok(total)
}

/// Module for error handling
pub mod errors {
    use std::fmt;

    #[derive(Debug)]
    pub struct CustomError {
        message: String,
    }

    impl fmt::Display for CustomError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.message)
        }
    }

    impl std::error::Error for CustomError {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = Config::new("test".to_string(), "1.0".to_string());
        assert_eq!(config.name, "test");
        assert!(!config.debug);
    }

    #[test]
    fn test_encode_string() {
        let result = encode_string("A");
        assert_eq!(result, "41");
    }
}

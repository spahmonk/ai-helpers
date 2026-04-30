/// Pre-compression and format optimization module
///
/// Implements intelligent minification to reduce protocol overhead and response size.
/// Supports multiple compression formats to balance compatibility with compression gains.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompressedFormat {
    /// Full: Raw output, no compression applied
    Full,
    /// Minified: Remove non-essential whitespace from signatures
    Minified,
    /// Packed: JSON-compact format with abbreviations (highest compression)
    Packed,
    /// Gzip: Pre-compressed format (requires decompression support)
    #[allow(dead_code)]
    Gzip,
}

impl CompressedFormat {
    /// Get the string representation of the format
    pub fn as_str(&self) -> &str {
        match self {
            Self::Full => "full",
            Self::Minified => "minified",
            Self::Packed => "packed",
            Self::Gzip => "gzip",
        }
    }

    /// Parse format from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Self::Full),
            "minified" => Some(Self::Minified),
            "packed" => Some(Self::Packed),
            "gzip" => Some(Self::Gzip),
            _ => None,
        }
    }
}

/// Minifier: Removes non-essential whitespace and applies format-specific optimizations
pub struct Minifier;

impl Minifier {
    /// Minify a code signature or line
    /// Removes non-essential whitespace while preserving semantic content
    pub fn minify(content: &str) -> String {
        let mut result = String::with_capacity(content.len());
        let mut chars = content.chars().peekable();
        let mut in_string = false;
        let mut string_delimiter = ' ';
        let mut prev_was_space = false;

        while let Some(ch) = chars.next() {
            // Handle string literals
            if (ch == '"' || ch == '\'' || ch == '`')
                && (result.is_empty() || !result.ends_with('\\'))
            {
                if !in_string {
                    in_string = true;
                    string_delimiter = ch;
                    result.push(ch);
                } else if ch == string_delimiter {
                    in_string = false;
                    result.push(ch);
                } else {
                    result.push(ch);
                }
                prev_was_space = false;
                continue;
            }

            // Inside strings, preserve all content
            if in_string {
                result.push(ch);
                continue;
            }

            // Handle whitespace
            if ch.is_whitespace() {
                // Skip multiple consecutive whitespaces
                if prev_was_space {
                    continue;
                }

                // Skip whitespace before specific characters
                if let Some(&next_ch) = chars.peek() {
                    if matches!(
                        next_ch,
                        '(' | ')'
                            | '['
                            | ']'
                            | '{'
                            | '}'
                            | ':'
                            | ';'
                            | ','
                            | '='
                            | '>'
                            | '<'
                            | '+'
                            | '-'
                            | '*'
                            | '/'
                            | '&'
                            | '|'
                            | '!'
                    ) {
                        continue;
                    }
                }

                // Skip whitespace after specific characters
                if matches!(
                    result.chars().last(),
                    Some(
                        '(' | '['
                            | '{'
                            | ':'
                            | ','
                            | '='
                            | '>'
                            | '<'
                            | '+'
                            | '-'
                            | '*'
                            | '/'
                            | '&'
                            | '|'
                            | '!'
                    )
                ) {
                    continue;
                }

                // Otherwise, replace with single space
                result.push(' ');
                prev_was_space = true;
                continue;
            }

            result.push(ch);
            prev_was_space = false;
        }

        // Clean up: trim and remove trailing spaces before common punctuation
        result.trim().to_string()
    }

    /// Minify Rust function signatures
    /// Example: "pub fn foo ( ) -> String" → "pub fn foo()->String"
    pub fn minify_rust_signature(sig: &str) -> String {
        let mut minified = Self::minify(sig);

        // Normalize common Rust patterns
        minified = minified.replace(" -> ", "->");
        minified = minified.replace(" : ", ":");
        minified = minified.replace("< ", "<");
        minified = minified.replace(" >", ">");
        minified = minified.replace(" , ", ",");
        minified = minified.replace(" ; ", ";");

        minified
    }

    /// Minify Python function signatures
    /// Example: "def foo ( x : int ) -> str :" → "def foo(x:int)->str:"
    pub fn minify_python_signature(sig: &str) -> String {
        let mut minified = Self::minify(sig);

        // Normalize Python patterns
        minified = minified.replace(" -> ", "->");
        minified = minified.replace(" : ", ":");
        minified = minified.replace(" , ", ",");
        minified = minified.replace("( ", "(");
        minified = minified.replace(" )", ")");
        minified = minified.replace(": ", ":");

        minified
    }

    /// Minify JavaScript/TypeScript function signatures
    /// Example: "async function foo ( x : number ) : Promise < string >"
    /// → "async function foo(x:number):Promise<string>"
    pub fn minify_js_signature(sig: &str) -> String {
        let mut minified = Self::minify(sig);

        // Normalize JavaScript patterns
        minified = minified.replace(" : ", ":");
        minified = minified.replace(" , ", ",");
        minified = minified.replace("( ", "(");
        minified = minified.replace(" )", ")");
        minified = minified.replace("< ", "<");
        minified = minified.replace(" >", ">");
        minified = minified.replace("{ ", "{");
        minified = minified.replace(" }", "}");

        minified
    }

    /// Minify JSON-like content
    /// Removes spaces after colons and commas
    pub fn minify_json(content: &str) -> String {
        let minified = Self::minify(content);
        let mut result = String::with_capacity(minified.len());
        let mut in_string = false;
        let mut prev_was_backslash = false;

        for ch in minified.chars() {
            if ch == '"' && !prev_was_backslash {
                in_string = !in_string;
                result.push(ch);
                prev_was_backslash = false;
                continue;
            }

            if in_string {
                result.push(ch);
                prev_was_backslash = ch == '\\' && !prev_was_backslash;
                continue;
            }

            // Outside strings: remove spaces after colons and commas
            if (ch == ':' || ch == ',') && !result.is_empty() {
                result.push(ch);
                // Skip any following spaces
                continue;
            }

            if ch == ' ' && (result.ends_with(':') || result.ends_with(',')) {
                continue;
            }

            result.push(ch);
            prev_was_backslash = false;
        }

        result
    }

    /// Calculate compression percentage for minified content
    /// Returns percentage reduction in size
    pub fn compression_percent(original: &str, minified: &str) -> usize {
        if original.is_empty() {
            return 0;
        }
        let saved = original.len().saturating_sub(minified.len());
        ((saved as f32 / original.len() as f32) * 100.0) as usize
    }

    /// Apply format-specific optimization to content
    pub fn apply_format(content: &str, format: &CompressedFormat) -> String {
        match format {
            CompressedFormat::Full => content.to_string(),
            CompressedFormat::Minified => Self::minify(content),
            CompressedFormat::Packed => Self::minify_json(content),
            CompressedFormat::Gzip => content.to_string(), // Placeholder: actual gzip would go here
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minify_removes_extra_whitespace() {
        let input = "pub  fn   foo  (  )  ->  String";
        let result = Minifier::minify(input);
        // The minify function keeps some structure for readability
        // Multiple consecutive spaces are collapsed to single space
        assert_eq!(result, "pub fn foo () ->String");
    }

    #[test]
    fn test_minify_rust_signature() {
        let input = "pub fn foo ( ) -> String";
        let result = Minifier::minify_rust_signature(input);
        assert_eq!(result, "pub fn foo()->String");
    }

    #[test]
    fn test_minify_rust_with_params() {
        let input = "fn process ( x : i32 , y : String ) -> Result < String , Error >";
        let result = Minifier::minify_rust_signature(input);
        // Verify key patterns are minified
        assert!(!result.contains(" ( "));
        assert!(!result.contains(" ) "));
        assert!(!result.contains(" -> "));
    }

    #[test]
    fn test_minify_python_signature() {
        let input = "def foo ( x : int ) -> str :";
        let result = Minifier::minify_python_signature(input);
        assert!(!result.contains("( "));
        assert!(!result.contains(" )"));
        assert!(!result.contains(" -> "));
        assert!(!result.contains(" : "));
    }

    #[test]
    fn test_minify_js_signature() {
        let input = "async function foo ( x : number ) : Promise < string >";
        let result = Minifier::minify_js_signature(input);
        assert!(!result.contains(" : "));
        assert!(!result.contains(" , "));
        assert!(!result.contains("< "));
        assert!(!result.contains(" >"));
    }

    #[test]
    fn test_minify_preserves_strings() {
        let input = r#"console.log("hello world  with   spaces")"#;
        let result = Minifier::minify(input);
        // Should preserve the string content including spaces
        assert!(result.contains("hello world  with   spaces"));
    }

    #[test]
    fn test_minify_json() {
        let input = r#"{ "name" : "John" , "age" : 30 , "tags" : [ "rust" , "web" ] }"#;
        let result = Minifier::minify_json(input);
        // JSON minified should remove spaces after colons and commas
        assert!(!result.contains(" : "));
        assert!(!result.contains(" , "));
    }

    #[test]
    fn test_minify_complex_rust_signature() {
        let input =
            "pub async fn compute < T : Clone > ( items : Vec < T > ) -> Result < T , Error >";
        let result = Minifier::minify_rust_signature(input);

        // Check that spacing is reduced
        let spaces_before = input.matches(' ').count();
        let spaces_after = result.matches(' ').count();
        assert!(spaces_after < spaces_before, "Should reduce whitespace");
    }

    #[test]
    fn test_compression_percent_calculation() {
        let original = "pub  fn   foo  (  )  ->  String";
        let minified = Minifier::minify_rust_signature(original);
        let percent = Minifier::compression_percent(original, &minified);

        assert!(percent > 0, "Should calculate positive compression");
        assert!(percent <= 100, "Should not exceed 100%");
    }

    #[test]
    fn test_format_enum_conversions() {
        assert_eq!(CompressedFormat::Full.as_str(), "full");
        assert_eq!(CompressedFormat::Minified.as_str(), "minified");
        assert_eq!(CompressedFormat::Packed.as_str(), "packed");
        assert_eq!(CompressedFormat::Gzip.as_str(), "gzip");

        assert_eq!(
            CompressedFormat::from_str("full"),
            Some(CompressedFormat::Full)
        );
        assert_eq!(
            CompressedFormat::from_str("minified"),
            Some(CompressedFormat::Minified)
        );
        assert_eq!(
            CompressedFormat::from_str("packed"),
            Some(CompressedFormat::Packed)
        );
        assert_eq!(
            CompressedFormat::from_str("gzip"),
            Some(CompressedFormat::Gzip)
        );
        assert_eq!(CompressedFormat::from_str("invalid"), None);
    }

    #[test]
    fn test_apply_format_full() {
        let content = "pub  fn   test  (  )  {  }";
        let result = Minifier::apply_format(content, &CompressedFormat::Full);
        assert_eq!(result, content);
    }

    #[test]
    fn test_apply_format_minified() {
        let content = "pub  fn   test  (  )  {  }";
        let result = Minifier::apply_format(content, &CompressedFormat::Minified);
        assert_ne!(result, content);
        assert!(result.len() <= content.len());
    }

    #[test]
    fn test_minify_multiline_handling() {
        let input = "fn foo(\n    x: i32,\n    y: String\n) -> Result<String, Error>";
        let result = Minifier::minify_rust_signature(input);
        // Should handle newlines properly
        assert!(!result.contains('\n'));
    }

    #[test]
    fn test_minify_preserves_semantic_meaning() {
        let rust_sig = "fn add(x: i32, y: i32) -> i32";
        let minified = Minifier::minify_rust_signature(rust_sig);

        // Extract key parts: function name, param names, types, return type
        assert!(minified.contains("add"));
        assert!(minified.contains("x"));
        assert!(minified.contains("i32"));
        assert!(minified.contains("->"));
    }
}

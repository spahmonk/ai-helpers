#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactionRuleSet {
    sensitive_keys: Vec<&'static str>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RedactionPipeline {
    rule_set: RedactionRuleSet,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RedactedText {
    pub text: String,
    pub redactions_applied: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SensitiveValueMatch {
    value_start: usize,
    value_end: usize,
}

impl RedactionRuleSet {
    pub fn default_rules() -> Self {
        Self {
            sensitive_keys: vec![
                "token",
                "access_token",
                "api_key",
                "apikey",
                "password",
                "passwd",
                "secret",
                "bearer",
            ],
        }
    }
}

impl Default for RedactionRuleSet {
    fn default() -> Self {
        Self::default_rules()
    }
}

impl RedactionPipeline {
    pub fn new(rule_set: RedactionRuleSet) -> Self {
        Self { rule_set }
    }

    pub fn redact_text(&self, input: &str) -> String {
        self.redact(input).text
    }

    pub fn redact(&self, input: &str) -> RedactedText {
        let mut keys = self.rule_set.sensitive_keys.clone();
        keys.sort_unstable_by_key(|key| std::cmp::Reverse(key.len()));

        let lower_input = input.to_ascii_lowercase();
        let lower_bytes = lower_input.as_bytes();
        let mut text = String::with_capacity(input.len());
        let mut cursor = 0usize;
        let mut redactions_applied = 0usize;

        while let Some(found) = find_next_sensitive_value(input, lower_bytes, &keys, cursor) {
            text.push_str(&input[cursor..found.value_start]);
            text.push_str("[REDACTED]");
            cursor = found.value_end;
            redactions_applied += 1;
        }

        text.push_str(&input[cursor..]);

        RedactedText {
            text,
            redactions_applied,
        }
    }
}

fn find_next_sensitive_value(
    input: &str,
    lower_bytes: &[u8],
    keys: &[&str],
    cursor: usize,
) -> Option<SensitiveValueMatch> {
    let bytes = input.as_bytes();
    let mut index = cursor;

    while index < bytes.len() {
        for key in keys {
            if let Some(found) = match_sensitive_value_at(bytes, lower_bytes, index, key) {
                return Some(found);
            }
        }

        index += 1;
    }

    None
}

fn match_sensitive_value_at(
    bytes: &[u8],
    lower_bytes: &[u8],
    index: usize,
    key: &str,
) -> Option<SensitiveValueMatch> {
    let key_bytes = key.as_bytes();

    if !lower_bytes.get(index..)?.starts_with(key_bytes) || !has_key_boundary(bytes, index) {
        return None;
    }

    let mut cursor = index + key_bytes.len();

    if let Some(quote) = bytes.get(index.wrapping_sub(1)) {
        if matches!(quote, b'"' | b'\'') && bytes.get(cursor) == Some(quote) {
            cursor += 1;
        }
    }

    let separator_start = cursor;
    while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }

    if matches!(bytes.get(cursor), Some(b'=' | b':')) {
        cursor += 1;
    } else if !key.eq_ignore_ascii_case("bearer") || cursor == separator_start {
        return None;
    }

    parse_sensitive_value(bytes, cursor).map(|(value_start, value_end)| SensitiveValueMatch {
        value_start,
        value_end,
    })
}

fn has_key_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0 || !is_identifier_byte(bytes[index - 1])
}

fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-')
}

fn parse_sensitive_value(bytes: &[u8], mut cursor: usize) -> Option<(usize, usize)> {
    while matches!(bytes.get(cursor), Some(b' ' | b'\t')) {
        cursor += 1;
    }

    let quote = match bytes.get(cursor) {
        Some(b'"' | b'\'') => {
            let quote = bytes[cursor];
            cursor += 1;
            Some(quote)
        }
        Some(_) => None,
        None => return None,
    };

    let value_start = cursor;
    while let Some(byte) = bytes.get(cursor) {
        if let Some(quote) = quote {
            if *byte == b'\\' {
                cursor += usize::from(cursor + 1 < bytes.len()) + 1;
                continue;
            }

            if *byte == quote {
                break;
            }
        } else if is_value_terminator(*byte) {
            break;
        }

        cursor += 1;
    }

    (cursor > value_start).then_some((value_start, cursor))
}

fn is_value_terminator(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'\t'
            | b'\n'
            | b'\r'
            | b'&'
            | b','
            | b';'
            | b')'
            | b']'
            | b'}'
            | b'"'
            | b'\''
            | b'#'
    )
}

#[cfg(test)]
mod tests {
    use super::RedactionPipeline;

    #[test]
    fn masks_common_secret_markers() {
        let pipeline = RedactionPipeline::default();

        let redacted = pipeline.redact_text("token=abc123 password: super-secret");

        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("super-secret"));
    }

    #[test]
    fn preserves_structured_and_url_like_inputs() {
        let pipeline = RedactionPipeline::default();

        let redacted = pipeline.redact_text(
            r#"{"path":"/repo/src/tokenizer.rs","access_token":"abc123","url":"https://example.test/download?file=src/lib.rs&token=xyz789"}"#,
        );

        assert!(redacted.contains(r#""path":"/repo/src/tokenizer.rs""#));
        assert!(redacted.contains(r#""access_token":"[REDACTED]""#));
        assert!(redacted.contains("file=src/lib.rs&token=[REDACTED]"));
        assert!(!redacted.contains("abc123"));
        assert!(!redacted.contains("xyz789"));
    }

    #[test]
    fn overlapping_keys_are_counted_once() {
        let pipeline = RedactionPipeline::default();

        let redacted = pipeline.redact("access_token=abc123");

        assert_eq!(redacted.text, "access_token=[REDACTED]");
        assert_eq!(redacted.redactions_applied, 1);
    }

    #[test]
    fn redacts_slash_containing_secret_values_fully() {
        let pipeline = RedactionPipeline::default();

        let redacted = pipeline.redact_text("access_token=abc/def api_key=ghi\\jkl");

        assert_eq!(redacted, "access_token=[REDACTED] api_key=[REDACTED]");
    }

    #[test]
    fn redacts_bearer_tokens_with_slashes_without_touching_urls() {
        let pipeline = RedactionPipeline::default();

        let redacted = pipeline.redact_text(
            "Authorization: Bearer abc/def\\ghi https://example.test/download/abc/def",
        );

        assert_eq!(
            redacted,
            "Authorization: Bearer [REDACTED] https://example.test/download/abc/def"
        );
    }

    #[test]
    fn redacts_json_values_with_escaped_quotes_without_corrupting_structure() {
        let pipeline = RedactionPipeline::default();

        let redacted =
            pipeline.redact_text(r#"{"access_token":"ab\\\"cd","path":"/repo/src/tokenizer.rs"}"#);

        assert_eq!(
            redacted,
            r#"{"access_token":"[REDACTED]","path":"/repo/src/tokenizer.rs"}"#
        );
    }

    #[test]
    fn redacts_shell_quoted_values_with_escaped_quotes_without_corrupting_structure() {
        let pipeline = RedactionPipeline::default();

        let redacted =
            pipeline.redact_text(r#"command access_token="ab\\\"cd" path=/repo/src/lib.rs"#);

        assert_eq!(
            redacted,
            r#"command access_token="[REDACTED]" path=/repo/src/lib.rs"#
        );
    }
}

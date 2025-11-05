use regex::Regex;
use std::collections::HashMap;

/// Text processor that applies word overrides and punctuation commands
///
/// This handles transforming transcribed text according to user preferences:
/// - Word overrides: Replace specific words/phrases (case-insensitive)
/// - Punctuation commands: Convert spoken commands to punctuation
pub struct TextProcessor {
    word_overrides: Vec<(Regex, String)>,
    punctuation: Vec<(Regex, &'static str)>,
}

impl TextProcessor {
    pub fn new(overrides: HashMap<String, String>) -> Self {
        // Compile word overrides into regexes
        let word_overrides = overrides
            .into_iter()
            .filter_map(|(k, v)| {
                // Case-insensitive word boundary match
                Regex::new(&format!(r"(?i)\b{}\b", regex::escape(&k)))
                    .ok()
                    .map(|re| (re, v))
            })
            .collect();

        let punctuation = vec![
            (Regex::new(r"\bperiod\b").unwrap(), "."),
            (Regex::new(r"\bcomma\b").unwrap(), ","),
            (Regex::new(r"\bquestion mark\b").unwrap(), "?"),
            (Regex::new(r"\bexclamation mark\b").unwrap(), "!"),
            (Regex::new(r"\bcolon\b").unwrap(), ":"),
            (Regex::new(r"\bsemicolon\b").unwrap(), ";"),
            (Regex::new(r"\bnew line\b").unwrap(), "\n"),
            (Regex::new(r"\btab\b").unwrap(), "\t"),
            (Regex::new(r"\bdash\b").unwrap(), "-"),
            (Regex::new(r"\bunderscore\b").unwrap(), "_"),
            (Regex::new(r"\bopen paren\b").unwrap(), "("),
            (Regex::new(r"\bclose paren\b").unwrap(), ")"),
            (Regex::new(r"\bopen bracket\b").unwrap(), "["),
            (Regex::new(r"\bclose bracket\b").unwrap(), "]"),
            (Regex::new(r"\bopen brace\b").unwrap(), "{"),
            (Regex::new(r"\bclose brace\b").unwrap(), "}"),
            (Regex::new(r"\bat symbol\b").unwrap(), "@"),
            (Regex::new(r"\bhash\b").unwrap(), "#"),
            (Regex::new(r"\bplus\b").unwrap(), "+"),
            (Regex::new(r"\bequals\b").unwrap(), "="),
            (Regex::new(r"\basterisk\b").unwrap(), "*"),
            (Regex::new(r"\bampersand\b").unwrap(), "&"),
            (Regex::new(r"\bpercent\b").unwrap(), "%"),
            (Regex::new(r"\bdollar sign\b").unwrap(), "$"),
            (Regex::new(r"\bbackslash\b").unwrap(), "\\"),
            (Regex::new(r"\bslash\b").unwrap(), "/"),
            (Regex::new(r"\bpipe\b").unwrap(), "|"),
            (Regex::new(r"\bcaret\b").unwrap(), "^"),
            (Regex::new(r"\btilde\b").unwrap(), "~"),
            (Regex::new(r"\bbacktick\b").unwrap(), "`"),
            (Regex::new(r"\bquote\b").unwrap(), "\""),
            (Regex::new(r"\bsingle quote\b").unwrap(), "'"),
            (Regex::new(r"\bless than\b").unwrap(), "<"),
            (Regex::new(r"\bgreater than\b").unwrap(), ">"),
        ];

        Self {
            word_overrides,
            punctuation,
        }
    }

    /// Process text by applying all transformations
    pub fn process(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Apply word overrides first
        for (re, replacement) in &self.word_overrides {
            result = re.replace_all(&result, replacement).to_string();
        }

        // Then apply punctuation commands
        for (re, replacement) in &self.punctuation {
            result = re.replace_all(&result, *replacement).to_string();
        }

        // Normalize whitespace and trim
        result = result.trim().to_string();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punctuation_commands() {
        let processor = TextProcessor::new(HashMap::new());

        assert_eq!(processor.process("hello period"), "hello .");
        assert_eq!(processor.process("hello comma world"), "hello , world");
        assert_eq!(
            processor.process("question mark at end question mark"),
            "? at end ?"
        );
    }

    #[test]
    fn test_word_overrides() {
        let mut overrides = HashMap::new();
        overrides.insert("dictator".to_string(), "dctr".to_string());

        let processor = TextProcessor::new(overrides);

        assert_eq!(
            processor.process("dictator is cool"),
            "dctr is cool"
        );
        assert_eq!(
            processor.process("Dictator is cool"),
            "dctr is cool"
        ); // Case insensitive
    }

    #[test]
    fn test_combined() {
        let mut overrides = HashMap::new();
        overrides.insert("dictator".to_string(), "Dictator".to_string());

        let processor = TextProcessor::new(overrides);

        assert_eq!(
            processor.process("dictator is great period"),
            "Dictator is great ."
        );
    }
}

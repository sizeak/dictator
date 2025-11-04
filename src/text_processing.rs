use anyhow::{Context, Result};
use regex::Regex;
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::task;

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

        // Punctuation commands from Python implementation
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

/// Inject text by processing it and simulating keyboard paste
///
/// This function:
/// - Processes the text through the TextProcessor
/// - Copies it to clipboard via wl-copy
/// - Triggers paste via ydotool
pub async fn inject_text(
    text: String,
    processor: &TextProcessor,
    paste_mode: &str,
) -> Result<()> {
    tracing::info!("Injecting text: {} chars", text.len());

    // Process text (word overrides + punctuation commands)
    let processed = processor.process(&text);
    let paste_mode = paste_mode.to_string();

    // Use spawn_blocking for external commands
    task::spawn_blocking(move || {
        // Copy to clipboard via wl-copy
        let mut child = Command::new("wl-copy")
            .stdin(Stdio::piped())
            .spawn()
            .context("Failed to spawn wl-copy")?;

        child
            .stdin
            .as_mut()
            .context("Failed to get wl-copy stdin")?
            .write_all(processed.as_bytes())
            .context("Failed to write to wl-copy")?;

        child.wait().context("wl-copy failed")?;

        // Wait for clipboard to settle
        std::thread::sleep(Duration::from_millis(120));

        // Trigger paste via ydotool
        let keycodes = match paste_mode.as_str() {
            "super" => "125:1 47:1 47:0 125:0",                     // Super+V
            "ctrl_shift" => "29:1 42:1 47:1 47:0 42:0 29:0",        // Ctrl+Shift+V
            _ => "29:1 47:1 47:0 29:0",                             // Ctrl+V
        };

        Command::new("ydotool")
            .args(["key", keycodes])
            .output()
            .context("Failed to execute ydotool")?;

        tracing::info!("Text injected successfully");
        Ok::<(), anyhow::Error>(())
    })
    .await
    .context("spawn_blocking failed")??;

    Ok(())
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
        overrides.insert("hyperwhisper".to_string(), "hyprwhspr".to_string());

        let processor = TextProcessor::new(overrides);

        assert_eq!(processor.process("hyperwhisper is cool"), "hyprwhspr is cool");
        assert_eq!(
            processor.process("HyperWhisper is cool"),
            "hyprwhspr is cool"
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

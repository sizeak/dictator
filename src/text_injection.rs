use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::task;

/// Inject processed text into the system via clipboard and keyboard simulation
///
/// This function:
/// - Copies the processed text to clipboard via wl-copy
/// - Waits for clipboard to settle
/// - Triggers paste via ydotool with the specified keyboard shortcut
pub async fn inject_text(processed_text: String, paste_mode: &str) -> Result<()> {
    tracing::info!("Injecting text: {} chars", processed_text.len());

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
            .write_all(processed_text.as_bytes())
            .context("Failed to write to wl-copy")?;

        child.wait().context("wl-copy failed")?;

        // Wait for clipboard to settle
        std::thread::sleep(Duration::from_millis(120));

        // Trigger paste via ydotool
        let keycodes = match paste_mode.as_str() {
            "super" => "125:1 47:1 47:0 125:0",              // Super+V
            "ctrl_shift" => "29:1 42:1 47:1 47:0 42:0 29:0", // Ctrl+Shift+V
            _ => "29:1 47:1 47:0 29:0",                      // Ctrl+V
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

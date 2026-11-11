use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::task;

use crate::config::PasteMode;

/// Inject processed text into the system via clipboard and keyboard simulation
///
/// This function:
/// - Copies the processed text to the clipboard and the primary selection (middle-click
///   paste) via wl-copy
/// - Waits for clipboard to settle (if paste_mode is not None)
/// - Triggers paste via ydotool with the specified keyboard shortcut (unless paste_mode is None)
pub async fn inject_text(processed_text: String, paste_mode: &PasteMode) -> Result<()> {
    tracing::info!("Processing text: {} chars", processed_text.len());

    let paste_mode = *paste_mode;

    // Use spawn_blocking for external commands
    task::spawn_blocking(move || {
        wl_copy(&processed_text, false)?;
        // Not every compositor supports the primary selection, so it's best effort
        if let Err(e) = wl_copy(&processed_text, true) {
            tracing::warn!("Failed to copy to primary selection: {:#}", e);
        }

        // Only trigger paste if not in "none" mode
        match paste_mode {
            PasteMode::None => {
                tracing::info!("Text copied to clipboard (paste_mode: none)");
            }
            _ => {
                // Wait for clipboard to settle
                std::thread::sleep(Duration::from_millis(120));

                // Trigger paste via ydotool
                let keycodes = match paste_mode {
                    PasteMode::Super => "125:1 47:1 47:0 125:0",              // Super+V
                    PasteMode::CtrlShift => "29:1 42:1 47:1 47:0 42:0 29:0", // Ctrl+Shift+V
                    PasteMode::Ctrl => "29:1 47:1 47:0 29:0",                // Ctrl+V
                    PasteMode::None => unreachable!(),
                };

                Command::new("ydotool")
                    .args(["key", keycodes])
                    .output()
                    .context("Failed to execute ydotool")?;

                tracing::info!("Text injected successfully");
            }
        }
        Ok::<(), anyhow::Error>(())
    })
    .await
    .context("spawn_blocking failed")??;

    Ok(())
}

/// Copy text to the clipboard, or to the primary selection if `primary` is set
fn wl_copy(text: &str, primary: bool) -> Result<()> {
    let mut command = Command::new("wl-copy");
    if primary {
        command.arg("--primary");
    }
    let mut child = command
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to spawn wl-copy")?;

    child
        .stdin
        .take()
        .context("Failed to get wl-copy stdin")?
        .write_all(text.as_bytes())
        .context("Failed to write to wl-copy")?;

    let status = child.wait().context("wl-copy failed")?;
    anyhow::ensure!(status.success(), "wl-copy exited with {}", status);
    Ok(())
}

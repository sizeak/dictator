use anyhow::{Context, Result};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures::StreamExt;
use tokio::sync::mpsc;

const SHORTCUT_ID: &str = "toggle-recording";
const DEFAULT_TRIGGER: &str = "LOGO+ALT+d";

/// Monitor for the global shortcut via XDG Desktop Portal.
///
/// Registers a "toggle-recording" shortcut with the compositor (KDE/GNOME/etc)
/// and sends `()` on the channel each time it's activated.
/// The user can reconfigure the binding through their desktop's shortcut settings.
pub async fn monitor_shortcut(tx: mpsc::Sender<()>) -> Result<()> {
    let shortcuts = GlobalShortcuts::new()
        .await
        .context("Failed to connect to GlobalShortcuts portal")?;

    let session = shortcuts
        .create_session()
        .await
        .context("Failed to create GlobalShortcuts session")?;

    let shortcut = NewShortcut::new(SHORTCUT_ID, "Toggle voice recording")
        .preferred_trigger(Some(DEFAULT_TRIGGER));

    shortcuts
        .bind_shortcuts(&session, &[shortcut], None)
        .await
        .context("Failed to bind shortcuts")?
        .response()
        .context("Shortcut binding was rejected")?;

    tracing::info!(
        "Global shortcut registered (default: {}). Reconfigure in System Settings > Shortcuts.",
        DEFAULT_TRIGGER
    );

    let mut stream = shortcuts
        .receive_activated()
        .await
        .context("Failed to listen for shortcut activations")?;

    while let Some(activated) = stream.next().await {
        if activated.shortcut_id() == SHORTCUT_ID {
            tracing::debug!("Shortcut activated: {}", SHORTCUT_ID);
            if tx.send(()).await.is_err() {
                break;
            }
        }
    }

    Ok(())
}

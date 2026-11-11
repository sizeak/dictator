use anyhow::{Context, Result};
use ashpd::desktop::global_shortcuts::{GlobalShortcuts, NewShortcut};
use futures::StreamExt;
use tokio::sync::mpsc;

const APP_ID: &str = "io.github.sizeak.Dictator";
const SHORTCUT_ID: &str = "toggle-recording";
const DEFAULT_TRIGGER: &str = "LOGO+ALT+d";

/// Monitor for the global shortcut via XDG Desktop Portal.
///
/// Registers a "toggle-recording" shortcut with the compositor (KDE/GNOME/etc)
/// and sends `()` on the channel each time it's activated.
/// The user can reconfigure the binding through their desktop's shortcut settings.
pub async fn monitor_shortcut(tx: mpsc::Sender<()>) -> Result<()> {
    // Portals identify unsandboxed apps by the app id of their systemd scope or .desktop
    // launcher. Run from a terminal or `cargo run`, there is none, and KDE then refuses the
    // session ("An app id is required"), so register one explicitly. The portal only accepts
    // an app id that has an installed .desktop file whose Exec program exists.
    if let Err(e) = install_desktop_entry() {
        tracing::warn!("Failed to install desktop entry for {}: {:#}", APP_ID, e);
    }
    let app_id = APP_ID.parse().context("Invalid app id")?;
    if let Err(e) = ashpd::register_host_app(app_id).await {
        tracing::warn!(
            "Failed to register app id {} with the portal: {}",
            APP_ID,
            e
        );
    }

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

/// Write `~/.local/share/applications/<APP_ID>.desktop` pointing at this executable, unless it
/// is already up to date. Hidden from menus; it only exists so the portal accepts the app id.
fn install_desktop_entry() -> Result<()> {
    let data_dir = match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) => std::path::PathBuf::from(dir),
        None => std::path::PathBuf::from(std::env::var_os("HOME").context("HOME not set")?)
            .join(".local/share"),
    };
    let path = data_dir
        .join("applications")
        .join(format!("{APP_ID}.desktop"));
    let exe = std::env::current_exe().context("Failed to locate executable")?;
    let contents = desktop_entry(&exe.to_string_lossy());

    if std::fs::read_to_string(&path).is_ok_and(|existing| existing == contents) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, contents).with_context(|| format!("Failed to write {:?}", path))?;
    tracing::info!("Installed desktop entry {:?}", path);
    Ok(())
}

fn desktop_entry(exe: &str) -> String {
    // Quote the path per the desktop entry spec so spaces and quotes survive
    let exe = exe
        .replace('\\', "\\\\\\\\")
        .replace('"', "\\\\\"")
        .replace('`', "\\\\`")
        .replace('$', "\\\\$");
    format!(
        "[Desktop Entry]\nType=Application\nName=Dictator\nComment=Voice transcription daemon\nExec=\"{exe}\"\nNoDisplay=true\nStartupNotify=false\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_desktop_entry_quotes_exec() {
        let entry = desktop_entry("/home/me/my apps/dictator");
        assert!(entry.contains("Exec=\"/home/me/my apps/dictator\"\n"));
        assert!(entry.contains("NoDisplay=true"));
    }
}

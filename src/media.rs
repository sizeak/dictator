//! Best-effort pausing of media players while recording, via `playerctl` (MPRIS).
//!
//! Every failure (playerctl missing, no players, a player vanishing) is a quiet no-op:
//! this must never get in the way of recording.

use tokio::process::Command;

/// Pause every player that is currently playing, returning the ones that were paused
pub async fn pause_playing() -> Vec<String> {
    let Some(list) = playerctl(&["--list-all"]).await else {
        return Vec::new();
    };
    let mut paused = Vec::new();
    for player in parse_player_list(&list) {
        if is_status(&player, "Playing").await
            && playerctl(&["-p", &player, "pause"]).await.is_some()
        {
            paused.push(player);
        }
    }
    if !paused.is_empty() {
        tracing::info!("Paused media: {}", paused.join(", "));
    }
    paused
}

/// Resume the given players, skipping any the user has since resumed, stopped or closed
pub async fn resume(players: &[String]) {
    for player in players {
        if is_status(player, "Paused").await {
            tracing::info!("Resuming media: {}", player);
            playerctl(&["-p", player, "play"]).await;
        }
    }
}

async fn is_status(player: &str, status: &str) -> bool {
    playerctl(&["-p", player, "status"])
        .await
        .is_some_and(|s| s.trim() == status)
}

/// Run playerctl, returning stdout on success
async fn playerctl(args: &[&str]) -> Option<String> {
    match Command::new("playerctl").args(args).output().await {
        Ok(out) if out.status.success() => Some(String::from_utf8_lossy(&out.stdout).into_owned()),
        Ok(_) => None,
        Err(e) => {
            tracing::debug!("playerctl unavailable: {}", e);
            None
        }
    }
}

fn parse_player_list(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && *l != "No players found")
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_player_list() {
        assert_eq!(
            parse_player_list("Plexamp\nchromium.instance42\n\n"),
            vec!["Plexamp", "chromium.instance42"]
        );
        assert!(parse_player_list("No players found\n").is_empty());
    }
}

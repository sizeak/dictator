use rodio::{Decoder, OutputStream, Sink};
use std::fs::File;
use std::io::BufReader;

/// Play recording start sound
pub async fn play_start_sound(path: &str) {
    tokio::spawn(play_sound(path.to_string()));
}

/// Play recording stop sound
pub async fn play_stop_sound(path: &str) {
    tokio::spawn(play_sound(path.to_string()));
}

async fn play_sound(path: String) {
    tokio::task::spawn_blocking(move || {
        if let Err(e) = play_sound_blocking(&path) {
            tracing::warn!("Failed to play sound {}: {}", path, e);
        }
    })
    .await
    .ok();
}

fn play_sound_blocking(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try both absolute path and relative to assets/
    let file = File::open(path).or_else(|_| {
        File::open(format!("assets/{}", path))
            .or_else(|_| File::open(format!("/usr/share/dictator/assets/{}", path)))
    })?;

    let source = Decoder::new(BufReader::new(file))?;

    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

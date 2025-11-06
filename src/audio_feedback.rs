use rodio::OutputStreamBuilder;
use std::fs::File;
use std::io::BufReader;

pub async fn play_sound(path: &str) {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || {
        if let Err(e) = play_sound_blocking(&path) {
            tracing::warn!("Failed to play sound {}: {}", path, e);
        }
    })
    .await
    .ok();
}

fn play_sound_blocking(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)
        .or_else(|_| File::open(format!("assets/{}", path)))
        .or_else(|_| File::open(format!("/usr/share/dictator/assets/{}", path)))?;

    let stream_handle = OutputStreamBuilder::open_default_stream()?;
    let sink = rodio::play(stream_handle.mixer(), BufReader::new(file))?;
    sink.sleep_until_end();

    Ok(())
}

use rodio::OutputStreamBuilder;
use std::fs::File;
use std::io::BufReader;

pub async fn play_sound(path: &str) {
    let path = path.to_string();
    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let file = File::open(&path)
                .or_else(|_| File::open(format!("assets/{}", path)))
                .or_else(|_| File::open(format!("/usr/share/dictator/assets/{}", path)));

            match file {
                Ok(file) => {
                    let stream_handle = OutputStreamBuilder::open_default_stream();
                    if let Ok(stream_handle) = stream_handle {
                        if let Ok(sink) = rodio::play(stream_handle.mixer(), BufReader::new(file)) {
                            sink.sleep_until_end();
                        } else {
                            tracing::warn!("Failed to play sound {}", path);
                        }
                    } else {
                        tracing::warn!("Failed to open audio stream for {}", path);
                    }
                }
                Err(e) => tracing::warn!("Failed to open sound file {}: {}", path, e),
            }
        })
        .await
        .ok();
    });
}

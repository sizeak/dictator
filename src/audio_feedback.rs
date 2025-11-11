use rodio::OutputStreamBuilder;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedbackSoundType {
    Start,
    Stop,
    Complete,
}

pub struct AudioFeedback {
    paths: HashMap<FeedbackSoundType, PathBuf>,
}

impl AudioFeedback {
    pub fn new(paths: HashMap<FeedbackSoundType, PathBuf>) -> Self {
        Self { paths }
    }

    pub async fn play(&self, sound_type: FeedbackSoundType) {
        if let Some(path) = self.paths.get(&sound_type) {
            play_sound(path.clone()).await;
        }
    }
}

async fn play_sound(path: PathBuf) {
    tokio::task::spawn_blocking(move || {
        if let Err(e) = play_sound_blocking(&path) {
            tracing::warn!("Failed to play sound {}: {}", path.display(), e);
        }
    })
    .await
    .ok();
}

fn play_sound_blocking(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(path)
        .or_else(|_| File::open(PathBuf::from("assets").join(path)))
        .or_else(|_| File::open(PathBuf::from("/usr/share/dictator/assets").join(path)))?;

    let stream_handle = OutputStreamBuilder::open_default_stream()?;
    let sink = rodio::play(stream_handle.mixer(), BufReader::new(file))?;
    sink.sleep_until_end();

    Ok(())
}

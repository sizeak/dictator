# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Test

```bash
cargo build --release        # Release build → target/release/dictator
cargo test                   # Run tests (text_processing module has unit tests)
cargo clippy                 # Lint
RUST_LOG=info cargo run      # Run with logging
```

No CI is configured. No rustfmt.toml or clippy.toml — use defaults.

## Architecture

Dictator is a voice transcription daemon for Linux/Wayland (~1100 lines of Rust). It listens for a global keyboard shortcut, records audio, sends it to an OpenAI-compatible API, and injects the transcribed text.

### State Machine (app.rs)

`Idle → Recording → Processing → Idle`

- **Idle**: Waiting for shortcut activation
- **Recording**: cpal captures audio into a streaming WAV file via lock-free ring buffer
- **Processing**: Audio sent to transcription API, text processed and injected; toggle press ignored during this state

### Audio Pipeline

```
cpal callback (f32, 16kHz mono) → HeapRb (lock-free ring buffer, 60s)
  → bridge_task (Notify-driven) → mpsc channel
  → WavSink (f32→i16, WAV encode on blocking thread) → NamedTempFile
```

The `Recorder` is `!Send` (holds `cpal::Stream`), which is why `main.rs` uses `tokio::task::LocalSet`. Everything else is `Send` and spawned normally.

### Module Roles

- **shortcuts.rs**: Registers global shortcut via XDG Desktop Portal (`ashpd` crate). Hardcoded default `LOGO+ALT+d`, user reconfigures via desktop settings (not config file).
- **audio/capture.rs**: cpal input stream → ring buffer producer
- **audio/recorder.rs**: Orchestrates capture start/stop, owns the cpal stream and task handles
- **audio/wav_sink.rs**: Streaming WAV encoding on a dedicated blocking thread
- **audio/feedback.rs**: Plays OGG sound effects via rodio (`spawn_blocking`)
- **transcription.rs**: `async_openai` client wrapper
- **text_processing.rs**: Regex-based voice command expansion (40+ patterns) and word overrides
- **text_injection.rs**: `wl-copy` for clipboard, `ydotool` for auto-paste (both via `spawn_blocking`)
- **config.rs**: JSON config at `~/.config/dictator/config.json`, auto-created with defaults if missing

### External Tool Dependencies

Runtime: `wl-copy` (clipboard, required), `ydotool` (auto-paste, optional), `xdg-desktop-portal` + compositor backend (shortcuts).

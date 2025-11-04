# Dictator

A voice transcription daemon for Linux/Wayland that enables system-wide voice-to-text input using remote transcription services.

## Features

- **Remote Transcription**: Uses OpenAI-compatible APIs (OpenAI, Groq, local Whisper servers)
- **System-wide Hotkey**: Global keyboard shortcut to start/stop recording
- **Auto Text Injection**: Automatically types transcribed text into any application
- **Audio Feedback**: Optional sound effects for recording start/stop
- **Word Overrides**: Custom replacements for commonly misheard words
- **Punctuation Commands**: Voice commands like "period", "comma", "question mark"
- **Streaming Architecture**: Efficient async Rust implementation with minimal latency

## Requirements

- Linux with Wayland compositor
- Rust toolchain (for building)
- `wl-copy` and `ydotool` (for text injection)
- Audio input device (microphone)
- OpenAI-compatible transcription API

## Installation

### Build from source

```bash
git clone https://github.com/sizeak/dictator
cd dictator
cargo build --release
sudo cp target/release/dictator /usr/local/bin/
```

### Configure

Create the configuration directory and copy the example config:

```bash
mkdir -p ~/.config/dictator
cp assets/config.example.json ~/.config/dictator/config.json
```

Edit `~/.config/dictator/config.json` with your API details:

```json
{
  "api_url": "http://localhost:8000/v1",
  "api_key": "your-api-key-here",
  "model": "Systran/faster-distil-whisper-large-v3",
  "primary_shortcut": "SUPER+ALT+D"
}
```

### Install systemd service (optional)

```bash
mkdir -p ~/.config/systemd/user
cp systemd/dictator.service ~/.config/systemd/user/
systemctl --user enable dictator.service
systemctl --user start dictator.service
```

## Usage

### Running manually

```bash
dictator
```

The daemon will start and listen for the configured keyboard shortcut (default: `SUPER+ALT+D`).

### Using the daemon

1. Press the shortcut to start recording (you'll hear a beep if audio feedback is enabled)
2. Speak your text
3. Press the shortcut again to stop recording
4. The transcribed text will be automatically typed into the focused application

### Configuration Options

- `api_url`: Base URL for the OpenAI-compatible API
- `api_key`: API authentication key
- `model`: Model name for transcription (e.g., "whisper-1" for OpenAI, model path for local servers)
- `primary_shortcut`: Keyboard shortcut (format: "SUPER+ALT+D")
- `paste_mode`: Paste shortcut ("ctrl_shift" for Ctrl+Shift+V, "super" for Super+V, "ctrl" for Ctrl+V)
- `audio_feedback`: Enable/disable sound effects (true/false)
- `start_sound_path`: Path to recording start sound
- `stop_sound_path`: Path to recording stop sound
- `language`: Transcription language code (e.g., "en")
- `whisper_prompt`: Optional prompt to guide transcription style
- `word_overrides`: Dictionary of word replacements

## Architecture

Dictator uses a service-based architecture with the following components:

- **Recorder**: Manages audio capture and WAV encoding
- **Transcriber**: Handles API communication for transcription
- **TextInjector**: Processes and injects transcribed text
- **AudioFeedback**: Plays sound effects for user feedback

The audio capture uses lock-free ring buffers for real-time performance, streaming audio data directly to WAV files as recording happens.

## Troubleshooting

### Keyboard shortcut not working

Make sure your user has access to input devices:

```bash
sudo usermod -aG input $USER
```

Log out and back in for changes to take effect.

### Audio not recording

Check that your microphone is working:

```bash
arecord -l
```

### Text not injecting

Ensure ydotool is installed and running:

```bash
sudo pacman -S ydotool  # Arch
sudo apt install ydotool  # Debian/Ubuntu
```

## License

MIT

## Credits

Inspired by [hyprwhspr](https://github.com/sizeak/hyprwhspr), reimplemented in Rust for better performance and reliability.

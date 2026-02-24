# Dictator

A voice transcription daemon for Linux/Wayland that enables system-wide voice-to-text input using OpenAI-compatible transcription APIs.

## Features

- **Remote Transcription**: Uses OpenAI-compatible APIs (OpenAI, Groq, local Whisper servers)
- **System-wide Hotkey**: Global keyboard shortcut via XDG Desktop Portal to start/stop recording
- **Flexible Text Injection**: Multiple paste modes including clipboard-only, Ctrl+V, Ctrl+Shift+V, and Super+V
- **Audio Feedback**: Optional sound effects for recording start/stop
- **Word Overrides**: Custom case-insensitive replacements for commonly misheard words
- **Voice Commands**: Extensive punctuation and symbol commands (period, comma, new line, etc.)
- **High-Performance Audio**: Async Rust implementation with lock-free ring buffers for real-time capture
- **Configurable Retries**: Built-in timeout and retry logic for API requests

## Requirements

- Linux with Wayland compositor supporting [XDG Desktop Portal](https://flatpak.github.io/xdg-desktop-portal/) (GNOME, KDE Plasma, COSMIC, etc.)
- `xdg-desktop-portal` and a compositor-specific backend (e.g., `xdg-desktop-portal-gnome`, `xdg-desktop-portal-kde`)
- Rust toolchain (for building)
- `wl-copy` (for clipboard operations)
- `ydotool` (for auto-paste modes - not needed if using `paste_mode: "none"`)
- Audio input device (microphone)
- OpenAI-compatible transcription API (local or remote)

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

Edit `~/.config/dictator/config.json` with your settings:

```json
{
  "api_url": "http://localhost:8000/v1",
  "api_key": "your-api-key-here",
  "model": "Systran/faster-distil-whisper-large-v3",
  "paste_mode": "ctrl_shift",
  "audio_feedback": true,
  "language": "en"
}
```

### Install systemd service (optional)

For automatic startup:

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

The daemon will start and register a global shortcut (default: `Logo+Alt+D`) via XDG Desktop Portal. You can reconfigure the binding in your desktop's System Settings > Shortcuts.

### Using the daemon

1. Press the shortcut to start recording (you'll hear a beep if audio feedback is enabled)
2. Speak your text
3. Press the shortcut again to stop recording
4. The text will be transcribed, processed, and either auto-pasted or copied to clipboard depending on your `paste_mode` setting

## Configuration Options

All configuration is stored in `~/.config/dictator/config.json`.

### Required Settings

- **`api_url`**: Base URL for the OpenAI-compatible API (e.g., `"http://localhost:8000/v1"`)
- **`api_key`**: API authentication key
- **`model`**: Model name for transcription
  - For local servers: model path (e.g., `"Systran/faster-distil-whisper-large-v3"`)
  - For OpenAI: `"whisper-1"`

### Optional Settings

- **`paste_mode`**: How to handle transcribed text (default: `"ctrl_shift"`)
  - `"none"`: Copy to clipboard only, no auto-paste
  - `"ctrl"`: Auto-paste using Ctrl+V
  - `"ctrl_shift"`: Auto-paste using Ctrl+Shift+V
  - `"super"`: Auto-paste using Super+V

- **`audio_feedback`**: Enable/disable sound effects (default: `true`)

- **`start_sound_path`**: Path to recording start sound (default: `"ping-up.ogg"`)
  - Relative paths are resolved from executable location or use absolute paths

- **`stop_sound_path`**: Path to recording stop sound (default: `"ping-down.ogg"`)

- **`complete_sound_path`**: Path to completion notification sound (default: `"ping-complete.ogg"`)
  - Plays when transcription completes and text is injected/copied to clipboard

- **`language`**: Two-letter language code for transcription (e.g., `"en"`, `"es"`, `"fr"`)
  - If not specified, API will auto-detect language

- **`whisper_prompt`**: Optional prompt to guide transcription style/context
  - Can improve accuracy for domain-specific vocabulary

- **`word_overrides`**: Dictionary of case-insensitive word/phrase replacements
  ```json
  "word_overrides": {
    "open ai": "OpenAI",
    "rust": "Rust",
    "dictator": "Dictator"
  }
  ```

- **`timeout`**: API request timeout in seconds (default: `30`)

- **`max_retries`**: Number of retry attempts for failed API requests (default: `2`)

## Voice Commands

Dictator supports voice commands for punctuation and symbols. Say the command word to insert the corresponding character:

**Punctuation:**
- period → `.`
- comma → `,`
- question mark → `?`
- exclamation mark → `!`
- colon → `:`
- semicolon → `;`

**Whitespace:**
- new line → `\n`
- tab → `\t`

**Symbols:**
- dash → `-`
- underscore → `_`
- slash → `/`
- backslash → `\`
- pipe → `|`
- at symbol → `@`
- hash → `#`
- dollar sign → `$`
- percent → `%`
- caret → `^`
- ampersand → `&`
- asterisk → `*`
- plus → `+`
- equals → `=`
- tilde → `~`

**Brackets:**
- open paren / close paren → `(` / `)`
- open bracket / close bracket → `[` / `]`
- open brace / close brace → `{` / `}`
- less than / greater than → `<` / `>`

**Quotes:**
- quote → `"`
- single quote → `'`
- backtick → `` ` ``

## Architecture

Dictator uses a modular service-based architecture:

- **App**: Main application loop handling state transitions
- **Recorder**: Audio capture service using cpal with lock-free ring buffers
- **Transcriber**: Handles OpenAI-compatible API communication
- **TextProcessor**: Applies word overrides and voice command transformations
- **AudioFeedback**: Plays sound effects using rodio
- **TextInjector**: Manages clipboard and keyboard simulation via wl-copy and ydotool

Audio is captured in 16-bit signed PCM format at 16kHz mono, streamed to temporary WAV files as recording happens, then sent to the transcription API.

The application uses Tokio's async runtime with a LocalSet to handle `!Send` futures from the audio capture library.

## Troubleshooting

### Keyboard shortcut not working

The shortcut is registered via XDG Desktop Portal's GlobalShortcuts interface. Ensure:

- `xdg-desktop-portal` is installed and running
- Your compositor's portal backend is installed (e.g., `xdg-desktop-portal-gnome`, `xdg-desktop-portal-kde`)
- Check if the shortcut was registered: look for "Global shortcut registered" in the logs
- You can reconfigure the binding in System Settings > Shortcuts

### Audio not recording

Check that your microphone is working and is the default input device:

```bash
arecord -l
```

View logs for more details:

```bash
journalctl --user -u dictator.service -f
```

Or run manually with logging:

```bash
RUST_LOG=info dictator
```

### Text not injecting

Ensure `ydotool` is installed (not needed if using `paste_mode: "none"`):

```bash
# Arch Linux
sudo pacman -S ydotool

# Debian/Ubuntu
sudo apt install ydotool
```

Ensure `wl-copy` is installed (required for all paste modes):

```bash
# Arch Linux
sudo pacman -S wl-clipboard

# Debian/Ubuntu
sudo apt install wl-clipboard
```

### API connection issues

- Check that your API server is running and accessible
- Verify the `api_url` in your config matches your server's address
- Check network connectivity and firewall rules
- Review logs for specific error messages

## License

MIT License - Copyright (c) 2025 Simon Jackson

See [LICENSE](LICENSE) for full details.

## Credits

Inspired by [hyprwhspr](https://github.com/sizeak/hyprwhspr), reimplemented in Rust for better performance and reliability.

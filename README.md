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
- `playerctl` (optional, to pause media while recording)
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

- **`paste_mode`**: How to handle transcribed text (default: `"ctrl_shift"`). The text is always copied to both the clipboard and the primary selection, so middle-click also pastes it
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

- **`voice_commands`**: Turn spoken commands such as "period" or "new line" into punctuation and symbols (default: `true`). Set to `false` if you speak naturally: with it on, those words are replaced wherever they occur, so "the trial period ends" becomes "the trial . ends"

- **`pause_media`**: Pause any media that is playing while you record, and resume it when you stop (default: `true`). Uses `playerctl`, so it works with any MPRIS player (Spotify, browsers, Plexamp, ...). Players you resume, stop or close yourself mid-recording are left alone. Does nothing if `playerctl` isn't installed

- **`timeout`**: API request timeout in seconds (default: `30`)

- **`max_retries`**: Number of retry attempts for failed API requests (default: `2`)

- **`cleanup`**: Optional LLM post-processing of the transcription. After `word_overrides` and voice commands have been applied, the text goes to a model with a system prompt that rewrites it as a Slack message in the speaker's voice: it fixes misheard words, spelling and punctuation, cuts filler, false starts, hedging and repetition, and condenses long rambles while keeping every substantive point. The replacement spellings from `word_overrides` are listed in the prompt as terms to keep exactly, so the model doesn't "correct" them. If cleanup fails, times out, or stops early (e.g. at a token limit), the uncleaned text is used instead.

  The model is reached through an OpenAI-compatible chat completions API. A small local model via [Ollama](https://ollama.com), such as `qwen2.5:3b`, `qwen2.5:1.5b` or `llama3.2:1b`, keeps cleanup fast and private:
  ```json
  "cleanup": {
    "enabled": true,
    "model": "qwen2.5:3b",
    "api_url": "http://localhost:11434/v1",
    "api_key": "ollama"
  }
  ```

  Settings:
  - `enabled`: default `false`
  - `model`: the model to request (required)
  - `api_url` / `api_key`: default to the transcription `api_url` / `api_key`; set them to use a different provider (e.g. a local Ollama, or `https://api.openai.com/v1`). If `api_url` points at a different host (scheme, hostname and port) and `api_key` is not set, no key is sent: the transcription key is never sent to a different host
  - `system_prompt`: instructions sent with every transcription. Leave it out to use the built-in cleanup prompt. The transcript is wrapped in `<transcript>...</transcript>` tags, so a custom prompt should say so
  - `temperature`: default `0`. Set to `null` to leave it out, which models that reject it (e.g. OpenAI o-series / gpt-5) require
  - `timeout`: seconds to wait before falling back to the uncleaned text (default: `20`). A local model that has to load from cold may need longer
  - Reasoning models' `<think>...</think>` blocks are stripped from the response

## Voice Commands

Dictator supports voice commands for punctuation and symbols (disable with `"voice_commands": false`). Say the command word to insert the corresponding character:

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
- Dictator writes a hidden `~/.local/share/applications/io.github.sizeak.Dictator.desktop` on startup so the portal can identify it. Without it, KDE refuses with "An app id is required"
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

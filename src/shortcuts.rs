use anyhow::{Context, Result};
use evdev::{Device, EventType, KeyCode};
use std::collections::HashSet;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Map friendly key names to their evdev KEY_* string representations
fn get_key_alias(name: &str) -> Option<&'static str> {
    match name {
        // Left-side modifiers
        "SUPER" | "META" | "WIN" => Some("KEY_LEFTMETA"),
        "ALT" => Some("KEY_LEFTALT"),
        "CTRL" | "CONTROL" => Some("KEY_LEFTCTRL"),
        "SHIFT" => Some("KEY_LEFTSHIFT"),

        // Right-side modifiers
        "RSUPER" | "RMETA" | "RWIN" => Some("KEY_RIGHTMETA"),
        "RALT" => Some("KEY_RIGHTALT"),
        "RCTRL" | "RCONTROL" => Some("KEY_RIGHTCTRL"),
        "RSHIFT" => Some("KEY_RIGHTSHIFT"),

        // Function keys
        "F1" => Some("KEY_F1"),
        "F2" => Some("KEY_F2"),
        "F3" => Some("KEY_F3"),
        "F4" => Some("KEY_F4"),
        "F5" => Some("KEY_F5"),
        "F6" => Some("KEY_F6"),
        "F7" => Some("KEY_F7"),
        "F8" => Some("KEY_F8"),
        "F9" => Some("KEY_F9"),
        "F10" => Some("KEY_F10"),
        "F11" => Some("KEY_F11"),
        "F12" => Some("KEY_F12"),
        "F13" => Some("KEY_F13"),
        "F14" => Some("KEY_F14"),
        "F15" => Some("KEY_F15"),
        "F16" => Some("KEY_F16"),
        "F17" => Some("KEY_F17"),
        "F18" => Some("KEY_F18"),
        "F19" => Some("KEY_F19"),
        "F20" => Some("KEY_F20"),
        "F21" => Some("KEY_F21"),
        "F22" => Some("KEY_F22"),
        "F23" => Some("KEY_F23"),
        "F24" => Some("KEY_F24"),

        // Numbers
        "0" => Some("KEY_0"),
        "1" => Some("KEY_1"),
        "2" => Some("KEY_2"),
        "3" => Some("KEY_3"),
        "4" => Some("KEY_4"),
        "5" => Some("KEY_5"),
        "6" => Some("KEY_6"),
        "7" => Some("KEY_7"),
        "8" => Some("KEY_8"),
        "9" => Some("KEY_9"),

        // Navigation
        "UP" => Some("KEY_UP"),
        "DOWN" => Some("KEY_DOWN"),
        "LEFT" => Some("KEY_LEFT"),
        "RIGHT" => Some("KEY_RIGHT"),
        "HOME" => Some("KEY_HOME"),
        "END" => Some("KEY_END"),
        "PAGEUP" | "PGUP" => Some("KEY_PAGEUP"),
        "PAGEDOWN" | "PGDOWN" => Some("KEY_PAGEDOWN"),

        // Editing keys
        "ENTER" | "RETURN" => Some("KEY_ENTER"),
        "SPACE" => Some("KEY_SPACE"),
        "BACKSPACE" | "BKSP" => Some("KEY_BACKSPACE"),
        "TAB" => Some("KEY_TAB"),
        "ESC" | "ESCAPE" => Some("KEY_ESC"),
        "DELETE" | "DEL" => Some("KEY_DELETE"),
        "INSERT" | "INS" => Some("KEY_INSERT"),

        // Punctuation
        "COMMA" => Some("KEY_COMMA"),
        "PERIOD" | "DOT" => Some("KEY_DOT"),
        "SLASH" => Some("KEY_SLASH"),
        "BACKSLASH" => Some("KEY_BACKSLASH"),
        "SEMICOLON" => Some("KEY_SEMICOLON"),
        "APOSTROPHE" | "QUOTE" => Some("KEY_APOSTROPHE"),
        "GRAVE" | "BACKTICK" => Some("KEY_GRAVE"),
        "LEFTBRACE" | "LBRACKET" => Some("KEY_LEFTBRACE"),
        "RIGHTBRACE" | "RBRACKET" => Some("KEY_RIGHTBRACE"),
        "MINUS" | "DASH" => Some("KEY_MINUS"),
        "EQUAL" | "EQUALS" => Some("KEY_EQUAL"),

        // Lock keys
        "CAPSLOCK" | "CAPS" => Some("KEY_CAPSLOCK"),
        "NUMLOCK" => Some("KEY_NUMLOCK"),
        "SCROLLLOCK" => Some("KEY_SCROLLLOCK"),

        // Media keys
        "VOLUMEUP" | "VOLUP" => Some("KEY_VOLUMEUP"),
        "VOLUMEDOWN" | "VOLDOWN" => Some("KEY_VOLUMEDOWN"),
        "MUTE" => Some("KEY_MUTE"),
        "PLAYPAUSE" | "PLAY" => Some("KEY_PLAYPAUSE"),
        "STOP" => Some("KEY_STOPCD"),
        "PREVIOUSSONG" | "PREV" => Some("KEY_PREVIOUSSONG"),
        "NEXTSONG" | "NEXT" => Some("KEY_NEXTSONG"),

        // Numpad
        "KP0" => Some("KEY_KP0"),
        "KP1" => Some("KEY_KP1"),
        "KP2" => Some("KEY_KP2"),
        "KP3" => Some("KEY_KP3"),
        "KP4" => Some("KEY_KP4"),
        "KP5" => Some("KEY_KP5"),
        "KP6" => Some("KEY_KP6"),
        "KP7" => Some("KEY_KP7"),
        "KP8" => Some("KEY_KP8"),
        "KP9" => Some("KEY_KP9"),
        "KPENTER" => Some("KEY_KPENTER"),
        "KPPLUS" => Some("KEY_KPPLUS"),
        "KPMINUS" => Some("KEY_KPMINUS"),
        "KPASTERISK" | "KPMULTIPLY" => Some("KEY_KPASTERISK"),
        "KPSLASH" | "KPDIVIDE" => Some("KEY_KPSLASH"),
        "KPDOT" => Some("KEY_KPDOT"),

        // Browser keys
        "BACK" => Some("KEY_BACK"),
        "FORWARD" => Some("KEY_FORWARD"),
        "REFRESH" => Some("KEY_REFRESH"),
        "HOMEPAGE" => Some("KEY_HOMEPAGE"),

        // Letter keys (A-Z)
        "A" => Some("KEY_A"),
        "B" => Some("KEY_B"),
        "C" => Some("KEY_C"),
        "D" => Some("KEY_D"),
        "E" => Some("KEY_E"),
        "F" => Some("KEY_F"),
        "G" => Some("KEY_G"),
        "H" => Some("KEY_H"),
        "I" => Some("KEY_I"),
        "J" => Some("KEY_J"),
        "K" => Some("KEY_K"),
        "L" => Some("KEY_L"),
        "M" => Some("KEY_M"),
        "N" => Some("KEY_N"),
        "O" => Some("KEY_O"),
        "P" => Some("KEY_P"),
        "Q" => Some("KEY_Q"),
        "R" => Some("KEY_R"),
        "S" => Some("KEY_S"),
        "T" => Some("KEY_T"),
        "U" => Some("KEY_U"),
        "V" => Some("KEY_V"),
        "W" => Some("KEY_W"),
        "X" => Some("KEY_X"),
        "Y" => Some("KEY_Y"),
        "Z" => Some("KEY_Z"),

        _ => None,
    }
}

/// Parse a shortcut string like "SUPER+ALT+D" into a set of KeyCode codes.
///
/// Supports three resolution methods:
/// 1. Friendly aliases (e.g., "SUPER", "F12", "ENTER")
/// 2. Direct evdev names (e.g., "KEY_LEFTMETA", "KEY_COMMA")
/// 3. Automatic KEY_* prefix (e.g., "COMMA" -> "KEY_COMMA")
pub fn parse_shortcut(shortcut: &str) -> Result<HashSet<KeyCode>> {
    let mut keys = HashSet::new();

    for part in shortcut.split('+') {
        let part_upper = part.trim().to_uppercase();

        // Tier 1: Check friendly aliases
        let evdev_name = if let Some(alias) = get_key_alias(&part_upper) {
            alias.to_string()
        } else if part_upper.starts_with("KEY_") {
            // Tier 2: Already in KEY_* format
            part_upper
        } else {
            // Tier 3: Try adding KEY_ prefix
            format!("KEY_{}", part_upper)
        };

        // Parse using evdev's FromStr implementation
        let keycode = KeyCode::from_str(&evdev_name)
            .map_err(|_| anyhow::anyhow!("Unknown key: {} (tried parsing as '{}')", part, evdev_name))?;

        keys.insert(keycode);
    }

    Ok(keys)
}

/// Monitor keyboards for the target key combination
///
/// Spawns a task for each keyboard device found, and sends a message
/// to the channel whenever the target combination is pressed
pub async fn monitor_keyboards(target_keys: HashSet<KeyCode>, tx: mpsc::Sender<()>) -> Result<()> {
    let keyboards = discover_keyboards()?;

    if keyboards.is_empty() {
        return Err(anyhow::anyhow!("No keyboard devices found"));
    }

    tracing::info!("Monitoring {} keyboard devices", keyboards.len());

    for device in keyboards {
        let keys = target_keys.clone();
        let tx = tx.clone();

        tokio::spawn(async move {
            if let Err(e) = monitor_device(device, keys, tx).await {
                tracing::error!("Device monitoring error: {}", e);
            }
        });
    }

    Ok(())
}

async fn monitor_device(
    device: Device,
    target_keys: HashSet<KeyCode>,
    tx: mpsc::Sender<()>,
) -> Result<()> {
    let device_name = device
        .name()
        .unwrap_or("unknown")
        .to_string();

    tracing::debug!("Monitoring device: {}", device_name);

    let mut stream = device
        .into_event_stream()
        .context("Failed to create event stream")?;

    let mut pressed = HashSet::new();
    let mut last_trigger = Instant::now();
    let debounce_duration = Duration::from_millis(500);

    loop {
        let event = stream
            .next_event()
            .await
            .context("Failed to read event")?;

        if event.event_type() == EventType::KEY {
            let key = KeyCode(event.code());

            match event.value() {
                1 => {
                    // Key down
                    pressed.insert(key);

                    // Check if target combination is pressed
                    if target_keys.is_subset(&pressed) {
                        let now = Instant::now();
                        if now.duration_since(last_trigger) > debounce_duration {
                            tracing::debug!("Shortcut triggered on device: {}", device_name);
                            if tx.send(()).await.is_err() {
                                // Receiver dropped, exit
                                break;
                            }
                            last_trigger = now;
                        }
                    }
                }
                0 => {
                    // Key up
                    pressed.remove(&key);
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn discover_keyboards() -> Result<Vec<Device>> {
    let mut keyboards = Vec::new();

    for (path, device) in evdev::enumerate() {
        // Check if device has keyboard capabilities
        if is_keyboard(&device) {
            keyboards.push(device);
        } else {
            tracing::debug!("Skipping non-keyboard device: {}", path.display());
        }
    }

    Ok(keyboards)
}

fn is_keyboard(device: &Device) -> bool {
    if let Some(keys) = device.supported_keys() {
        // Check for common keyboard keys
        keys.contains(KeyCode::KEY_A)
            && keys.contains(KeyCode::KEY_S)
            && keys.contains(KeyCode::KEY_ENTER)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_shortcut() {
        let keys = parse_shortcut("SUPER+ALT+D").unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&KeyCode::KEY_LEFTMETA));
        assert!(keys.contains(&KeyCode::KEY_LEFTALT));
        assert!(keys.contains(&KeyCode::KEY_D));
    }

    #[test]
    fn test_parse_shortcut_function_key() {
        let keys = parse_shortcut("F12").unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&KeyCode::KEY_F12));
    }

    #[test]
    fn test_parse_shortcut_right_modifiers() {
        let keys = parse_shortcut("RCTRL+RSHIFT+A").unwrap();
        assert_eq!(keys.len(), 3);
        assert!(keys.contains(&KeyCode::KEY_RIGHTCTRL));
        assert!(keys.contains(&KeyCode::KEY_RIGHTSHIFT));
        assert!(keys.contains(&KeyCode::KEY_A));
    }

    #[test]
    fn test_parse_shortcut_numbers_and_special() {
        let keys = parse_shortcut("CTRL+1").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::KEY_LEFTCTRL));
        assert!(keys.contains(&KeyCode::KEY_1));

        let keys = parse_shortcut("SUPER+ENTER").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::KEY_LEFTMETA));
        assert!(keys.contains(&KeyCode::KEY_ENTER));
    }

    #[test]
    fn test_parse_shortcut_direct_evdev_names() {
        let keys = parse_shortcut("KEY_COMMA").unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&KeyCode::KEY_COMMA));

        let keys = parse_shortcut("SUPER+KEY_COMMA").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::KEY_LEFTMETA));
        assert!(keys.contains(&KeyCode::KEY_COMMA));
    }

    #[test]
    fn test_parse_shortcut_automatic_prefix() {
        // Test automatic KEY_ prefix for keys without aliases
        let keys = parse_shortcut("COMMA").unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&KeyCode::KEY_COMMA));
    }

    #[test]
    fn test_parse_shortcut_media_keys() {
        let keys = parse_shortcut("VOLUMEUP").unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&KeyCode::KEY_VOLUMEUP));

        let keys = parse_shortcut("CTRL+PLAYPAUSE").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::KEY_LEFTCTRL));
        assert!(keys.contains(&KeyCode::KEY_PLAYPAUSE));
    }

    #[test]
    fn test_parse_shortcut_navigation() {
        let keys = parse_shortcut("CTRL+UP").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::KEY_LEFTCTRL));
        assert!(keys.contains(&KeyCode::KEY_UP));

        let keys = parse_shortcut("ALT+PAGEUP").unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&KeyCode::KEY_LEFTALT));
        assert!(keys.contains(&KeyCode::KEY_PAGEUP));
    }

    #[test]
    fn test_parse_shortcut_aliases() {
        // Test that aliases work correctly
        let keys1 = parse_shortcut("SUPER").unwrap();
        let keys2 = parse_shortcut("META").unwrap();
        let keys3 = parse_shortcut("WIN").unwrap();
        assert_eq!(keys1, keys2);
        assert_eq!(keys2, keys3);
        assert!(keys1.contains(&KeyCode::KEY_LEFTMETA));
    }

    #[test]
    fn test_parse_shortcut_invalid_key() {
        let result = parse_shortcut("INVALIDKEY123");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown key"));
    }
}

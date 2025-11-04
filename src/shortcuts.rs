use anyhow::{Context, Result};
use evdev::{Device, EventType, Key};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Parse a shortcut string like "SUPER+ALT+D" into a set of Key codes
pub fn parse_shortcut(shortcut: &str) -> Result<HashSet<Key>> {
    let mut keys = HashSet::new();

    for part in shortcut.split('+') {
        let key = match part.trim().to_uppercase().as_str() {
            "SUPER" | "META" | "WIN" => Key::KEY_LEFTMETA,
            "ALT" => Key::KEY_LEFTALT,
            "CTRL" | "CONTROL" => Key::KEY_LEFTCTRL,
            "SHIFT" => Key::KEY_LEFTSHIFT,
            // Letter keys
            "A" => Key::KEY_A,
            "B" => Key::KEY_B,
            "C" => Key::KEY_C,
            "D" => Key::KEY_D,
            "E" => Key::KEY_E,
            "F" => Key::KEY_F,
            "G" => Key::KEY_G,
            "H" => Key::KEY_H,
            "I" => Key::KEY_I,
            "J" => Key::KEY_J,
            "K" => Key::KEY_K,
            "L" => Key::KEY_L,
            "M" => Key::KEY_M,
            "N" => Key::KEY_N,
            "O" => Key::KEY_O,
            "P" => Key::KEY_P,
            "Q" => Key::KEY_Q,
            "R" => Key::KEY_R,
            "S" => Key::KEY_S,
            "T" => Key::KEY_T,
            "U" => Key::KEY_U,
            "V" => Key::KEY_V,
            "W" => Key::KEY_W,
            "X" => Key::KEY_X,
            "Y" => Key::KEY_Y,
            "Z" => Key::KEY_Z,
            _ => {
                return Err(anyhow::anyhow!("Unknown key: {}", part));
            }
        };
        keys.insert(key);
    }

    Ok(keys)
}

/// Monitor keyboards for the target key combination
///
/// Spawns a task for each keyboard device found, and sends a message
/// to the channel whenever the target combination is pressed
pub async fn monitor_keyboards(target_keys: HashSet<Key>, tx: mpsc::Sender<()>) -> Result<()> {
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
    target_keys: HashSet<Key>,
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
            let key = Key(event.code());

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
        keys.contains(Key::KEY_A)
            && keys.contains(Key::KEY_S)
            && keys.contains(Key::KEY_ENTER)
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
        assert!(keys.contains(&Key::KEY_LEFTMETA));
        assert!(keys.contains(&Key::KEY_LEFTALT));
        assert!(keys.contains(&Key::KEY_D));
    }

    #[test]
    fn test_parse_shortcut_single_key() {
        let keys = parse_shortcut("F12").unwrap();
        assert_eq!(keys.len(), 1);
    }
}

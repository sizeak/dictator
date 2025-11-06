use anyhow::{Context, Result};
use evdev::{Device, EventType, KeyCode};
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Parse a shortcut string like "SUPER+ALT+D" into a set of KeyCode codes
pub fn parse_shortcut(shortcut: &str) -> Result<HashSet<KeyCode>> {
    let mut keys = HashSet::new();

    for part in shortcut.split('+') {
        let key = match part.trim().to_uppercase().as_str() {
            "SUPER" | "META" | "WIN" => KeyCode::KEY_LEFTMETA,
            "ALT" => KeyCode::KEY_LEFTALT,
            "CTRL" | "CONTROL" => KeyCode::KEY_LEFTCTRL,
            "SHIFT" => KeyCode::KEY_LEFTSHIFT,
            // Letter keys
            "A" => KeyCode::KEY_A,
            "B" => KeyCode::KEY_B,
            "C" => KeyCode::KEY_C,
            "D" => KeyCode::KEY_D,
            "E" => KeyCode::KEY_E,
            "F" => KeyCode::KEY_F,
            "G" => KeyCode::KEY_G,
            "H" => KeyCode::KEY_H,
            "I" => KeyCode::KEY_I,
            "J" => KeyCode::KEY_J,
            "K" => KeyCode::KEY_K,
            "L" => KeyCode::KEY_L,
            "M" => KeyCode::KEY_M,
            "N" => KeyCode::KEY_N,
            "O" => KeyCode::KEY_O,
            "P" => KeyCode::KEY_P,
            "Q" => KeyCode::KEY_Q,
            "R" => KeyCode::KEY_R,
            "S" => KeyCode::KEY_S,
            "T" => KeyCode::KEY_T,
            "U" => KeyCode::KEY_U,
            "V" => KeyCode::KEY_V,
            "W" => KeyCode::KEY_W,
            "X" => KeyCode::KEY_X,
            "Y" => KeyCode::KEY_Y,
            "Z" => KeyCode::KEY_Z,
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
    fn test_parse_shortcut_single_key() {
        let keys = parse_shortcut("F12").unwrap();
        assert_eq!(keys.len(), 1);
    }
}

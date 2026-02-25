use tokio::process::Command;

pub fn run_hook(label: &str, command: &str) {
    let label = label.to_owned();
    let command = command.to_owned();

    tokio::task::spawn(async move {
        tracing::info!("[{}] Running hook: {}", label, command);

        match Command::new("sh")
            .arg("-c")
            .arg(&command)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(child) => match child.wait_with_output().await {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        tracing::warn!(
                            "[{}] Hook exited with {}: {}",
                            label,
                            output.status,
                            stderr.trim()
                        );
                    }
                }
                Err(e) => tracing::warn!("[{}] Failed to wait on hook: {}", label, e),
            },
            Err(e) => tracing::warn!("[{}] Failed to spawn hook: {}", label, e),
        }
    });
}

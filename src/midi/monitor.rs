/// MIDI device monitoring via subprocess
/// MIDI libraries cache device lists and won't update without working notifications.
/// We spawn fresh processes to enumerate devices, which see current system state.
#[cfg(target_os = "macos")]
pub mod macos {
    use crate::events::AppEvent;
    use crossbeam::channel::Sender;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// Start monitoring by spawning subprocesses to get fresh device enumeration
    pub fn start_monitor(event_tx: Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>> {
        thread::spawn(move || {
            let mut previous_devices = String::new();

            loop {
                thread::sleep(Duration::from_millis(1000));

                // Get the path to our own executable
                let exe_path = std::env::current_exe().ok();
                if exe_path.is_none() {
                    continue;
                }

                // Spawn ourselves with --list-ports flag to get fresh enumeration
                let output = Command::new(exe_path.unwrap())
                    .arg("--list-ports")
                    .output();

                if let Ok(output) = output {
                    if output.status.success() {
                        let devices_json = String::from_utf8_lossy(&output.stdout).to_string();

                        // Check if devices changed
                        if devices_json != previous_devices && !previous_devices.is_empty() {
                            // Parse the JSON to extract device names
                            if let Ok(parsed) = parse_port_json(&devices_json) {
                                let _ = event_tx.send(AppEvent::PortListUpdate {
                                    inputs: parsed.0,
                                    outputs: parsed.1,
                                });
                            }
                        }

                        previous_devices = devices_json;
                    }
                }
            }
        });

        Ok(())
    }

    /// Parse JSON output from --list-ports into PortId vectors
    pub(super) fn parse_port_json(json: &str) -> Result<(Vec<crate::connection::PortId>, Vec<crate::connection::PortId>), Box<dyn std::error::Error>> {
        use crate::connection::PortId;
        use crate::midi::virtual_ports::{
            VIRTUAL_INPUT_A_NAME, VIRTUAL_INPUT_B_NAME,
            VIRTUAL_OUTPUT_A_NAME, VIRTUAL_OUTPUT_B_NAME
        };

        // Simple JSON parsing (we control the format)
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        let mut in_inputs = false;
        let mut in_outputs = false;

        for line in json.lines() {
            let trimmed = line.trim();
            if trimmed.contains("\"inputs\"") {
                in_inputs = true;
                in_outputs = false;
            } else if trimmed.contains("\"outputs\"") {
                in_inputs = false;
                in_outputs = true;
            } else if trimmed.starts_with('"') && trimmed.len() > 2 {
                // Extract device name between quotes
                if let Some(end_quote) = trimmed[1..].find('"') {
                    let name = &trimmed[1..end_quote + 1];
                    let is_virtual = name == VIRTUAL_INPUT_A_NAME
                        || name == VIRTUAL_INPUT_B_NAME
                        || name == VIRTUAL_OUTPUT_A_NAME
                        || name == VIRTUAL_OUTPUT_B_NAME;
                    let port = PortId::new(name.to_string(), is_virtual);

                    if in_inputs {
                        inputs.push(port);
                    } else if in_outputs {
                        outputs.push(port);
                    }
                }
            }
        }

        Ok((inputs, outputs))
    }
}

#[cfg(not(target_os = "macos"))]
pub mod other {
    use crate::events::AppEvent;
    use crossbeam::channel::Sender;
    use std::process::Command;
    use std::thread;
    use std::time::Duration;

    /// Start monitoring by spawning subprocesses to get fresh device enumeration
    /// Same approach as macOS - works on all platforms
    pub fn start_monitor(event_tx: Sender<AppEvent>) -> Result<(), Box<dyn std::error::Error>> {
        thread::spawn(move || {
            let mut previous_devices = String::new();

            loop {
                thread::sleep(Duration::from_millis(1000));

                // Get the path to our own executable
                let exe_path = std::env::current_exe().ok();
                if exe_path.is_none() {
                    continue;
                }

                // Spawn ourselves with --list-ports flag to get fresh enumeration
                let output = Command::new(exe_path.unwrap())
                    .arg("--list-ports")
                    .output();

                if let Ok(output) = output {
                    if output.status.success() {
                        let devices_json = String::from_utf8_lossy(&output.stdout).to_string();

                        // Check if devices changed
                        if devices_json != previous_devices && !previous_devices.is_empty() {
                            // Parse the JSON to extract device names
                            if let Ok(parsed) = super::macos::parse_port_json(&devices_json) {
                                let _ = event_tx.send(AppEvent::PortListUpdate {
                                    inputs: parsed.0,
                                    outputs: parsed.1,
                                });
                            }
                        }

                        previous_devices = devices_json;
                    }
                }
            }
        });

        Ok(())
    }
}

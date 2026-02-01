use crate::connection::Connection;
use crate::events::AppEvent;
use crossbeam::channel::Sender;
use std::process::{Child, Command};

/// Handle for a running forwarder subprocess
pub struct ForwarderHandle {
    _connection: Connection,
    child: Child,
}

impl Drop for ForwarderHandle {
    fn drop(&mut self) {
        // Kill the worker subprocess when handle is dropped
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Starts a MIDI forwarder subprocess that forwards messages from input to output
/// The subprocess runs with fresh MIDI context that sees current device state
pub fn start_forwarder(
    connection: Connection,
    input_port_name: &str,
    output_port_name: &str,
    _event_tx: Sender<AppEvent>,
) -> Result<ForwarderHandle, Box<dyn std::error::Error>> {
    // Get the path to our own executable
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get executable path: {}", e))?;

    // Spawn a worker subprocess
    // The worker runs in a fresh process that sees current CoreMIDI state
    use std::process::Stdio;

    let child = Command::new(exe_path)
        .arg("worker")
        .arg(input_port_name)
        .arg(output_port_name)
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Failed to spawn worker: {}", e))?;

    Ok(ForwarderHandle {
        _connection: connection,
        child,
    })
}

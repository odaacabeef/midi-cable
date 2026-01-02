use crate::connection::Connection;
use crate::events::AppEvent;
use crate::midi::validation::{is_program_change, is_valid_midi_message, normalize_program_change};
use crossbeam::channel::Sender;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Handle for a running forwarder thread
pub struct ForwarderHandle {
    _connection: Connection,
    _join_handle: JoinHandle<()>,
    _midi_connection: MidiInputConnection<()>,
}

/// Starts a MIDI forwarder thread that forwards messages from input to output
pub fn start_forwarder(
    connection: Connection,
    input_port_name: &str,
    output_port_name: &str,
    event_tx: Sender<AppEvent>,
) -> Result<ForwarderHandle, Box<dyn std::error::Error>> {
    let midi_in = MidiInput::new("mc-forwarder")
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    let midi_out = MidiOutput::new("mc-forwarder")
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    // Find input port
    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).ok().as_deref() == Some(input_port_name))
        .ok_or_else(|| Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Input port '{}' not found", input_port_name)
        )) as Box<dyn std::error::Error>)?;

    // Find output port
    let out_ports = midi_out.ports();
    let out_port = out_ports
        .iter()
        .find(|p| midi_out.port_name(p).ok().as_deref() == Some(output_port_name))
        .ok_or_else(|| Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Output port '{}' not found", output_port_name)
        )) as Box<dyn std::error::Error>)?;

    // Open output connection
    let out_conn = midi_out.connect(out_port, "mc-out")
        .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to connect output: {:?}", e))) as Box<dyn std::error::Error>)?;
    let out_conn_shared = Arc::new(Mutex::new(out_conn));

    // Clone for the callback
    let out_conn_clone = Arc::clone(&out_conn_shared);
    let event_tx_clone = event_tx.clone();
    let conn_clone = connection.clone();

    // Open input connection with callback
    let in_conn = midi_in.connect(
        in_port,
        "mc-in",
        move |_timestamp, message, _| {
            handle_midi_message(message, &out_conn_clone, &event_tx_clone, &conn_clone);
        },
        (),
    ).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("Failed to connect input: {:?}", e))) as Box<dyn std::error::Error>)?;

    // Successfully started forwarding

    // The join handle is just a placeholder - actual work happens in the MIDI callback
    // The thread just waits for the connection to be dropped
    let join_handle = thread::spawn(|| {
        // This thread doesn't do much - the real work is in the MIDI callback
        // We just need to keep the connections alive
        thread::park();
    });

    Ok(ForwarderHandle {
        _connection: connection,
        _join_handle: join_handle,
        _midi_connection: in_conn,
    })
}

/// Handles a single MIDI message - validates and forwards it
fn handle_midi_message(
    message: &[u8],
    output: &Arc<Mutex<MidiOutputConnection>>,
    _event_tx: &Sender<AppEvent>,
    connection: &Connection,
) {
    if message.is_empty() {
        return;
    }

    // Special handling for Program Change messages (from Go's fwd.go lines 83-89)
    if is_program_change(message) {
        let normalized = normalize_program_change(message);
        if let Ok(mut out) = output.lock() {
            if let Err(e) = out.send(&normalized) {
                eprintln!("Error forwarding program change on {}: {}", connection, e);
            }
        }
        return;
    }

    // Validate other messages
    if is_valid_midi_message(message) {
        if let Ok(mut out) = output.lock() {
            if let Err(e) = out.send(message) {
                eprintln!("Error forwarding message on {}: {}", connection, e);
            }
        }
    }
    // Invalid messages are silently skipped
}

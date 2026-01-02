use anyhow::Result;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::{Arc, Mutex};

pub const VIRTUAL_INPUT_NAME: &str = "mc-virtual-in";
pub const VIRTUAL_OUTPUT_NAME: &str = "mc-virtual-out";

/// Manages virtual MIDI ports with broadcast forwarding
/// These connections must be kept alive for the virtual ports to remain available in the system
pub struct VirtualPorts {
    // Keep the connections alive - dropping them will destroy the virtual ports
    _input_connection: MidiInputConnection<()>,
    _output_connection: Arc<Mutex<MidiOutputConnection>>,
    // List of outputs to forward virtual input messages to
    input_outputs: Arc<Mutex<Vec<Arc<Mutex<MidiOutputConnection>>>>>,
}

impl VirtualPorts {
    /// Creates virtual MIDI input and output ports
    /// The ports will appear in the system as long as this struct is alive
    /// Messages received on mc-virtual-in are broadcast to all registered outputs
    #[cfg(unix)]
    pub fn create() -> Result<Self> {
        use midir::os::unix::{VirtualInput, VirtualOutput};

        // Create MIDI input and output objects
        let midi_in = MidiInput::new("mc")?;
        let midi_out = MidiOutput::new("mc")?;

        // Create virtual output port
        // This port will appear as "mc-virtual-out" in the system
        // Other applications can receive MIDI from this port
        let output_connection = midi_out
            .create_virtual(VIRTUAL_OUTPUT_NAME)
            .map_err(|e| anyhow::anyhow!("Failed to create virtual output: {:?}", e))?;

        // Wrap output in Arc<Mutex> for thread safety
        let output_shared = Arc::new(Mutex::new(output_connection));

        // Create list for broadcast outputs
        let input_outputs: Arc<Mutex<Vec<Arc<Mutex<MidiOutputConnection>>>>> = Arc::new(Mutex::new(Vec::new()));
        let outputs_for_callback = Arc::clone(&input_outputs);

        // Create virtual input port with callback that broadcasts to all outputs
        // This port will appear as "mc-virtual-in" in the system
        // Other applications can send MIDI to this port
        let input_connection = midi_in
            .create_virtual(
                VIRTUAL_INPUT_NAME,
                move |_timestamp, message, _| {
                    // Broadcast message to all registered outputs
                    if let Ok(outputs) = outputs_for_callback.lock() {
                        for output in outputs.iter() {
                            if let Ok(mut out) = output.lock() {
                                if let Err(e) = out.send(message) {
                                    eprintln!("Error forwarding from virtual input: {}", e);
                                }
                            }
                        }
                    }
                },
                (),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create virtual input: {:?}", e))?;

        Ok(VirtualPorts {
            _input_connection: input_connection,
            _output_connection: output_shared,
            input_outputs,
        })
    }

    /// Add an output connection to receive messages from virtual input
    /// Returns a handle that should be kept alive to maintain the connection
    pub fn add_virtual_input_output(&self, output_name: &str) -> Result<Arc<Mutex<MidiOutputConnection>>> {
        // Special case: if output is our own virtual output, use the existing connection
        if output_name == VIRTUAL_OUTPUT_NAME {
            let out_conn_shared = Arc::clone(&self._output_connection);

            // Add to the broadcast list
            if let Ok(mut outputs) = self.input_outputs.lock() {
                outputs.push(Arc::clone(&out_conn_shared));
            }

            return Ok(out_conn_shared);
        }

        // Regular output port - create new connection
        let midi_out = MidiOutput::new("mc-virtual-fwd")?;

        // Find the output port
        let out_ports = midi_out.ports();
        let out_port = out_ports
            .iter()
            .find(|p| midi_out.port_name(p).ok().as_deref() == Some(output_name))
            .ok_or_else(|| anyhow::anyhow!("Output port '{}' not found", output_name))?;

        // Connect to the output
        let out_conn = midi_out.connect(out_port, "mc-virtual-out")
            .map_err(|e| anyhow::anyhow!("Failed to connect to output: {:?}", e))?;

        let out_conn_shared = Arc::new(Mutex::new(out_conn));

        // Add to the broadcast list
        if let Ok(mut outputs) = self.input_outputs.lock() {
            outputs.push(Arc::clone(&out_conn_shared));
        }

        Ok(out_conn_shared)
    }

    /// Remove an output connection from virtual input broadcast list
    pub fn remove_virtual_input_output(&self, output: &Arc<Mutex<MidiOutputConnection>>) {
        if let Ok(mut outputs) = self.input_outputs.lock() {
            outputs.retain(|out| !Arc::ptr_eq(out, output));
        }
    }

    #[cfg(not(unix))]
    pub fn create() -> Result<Self> {
        Err(anyhow::anyhow!(
            "Virtual ports are only supported on Unix/macOS/Linux platforms"
        ))
    }
}

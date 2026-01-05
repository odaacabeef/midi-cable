use anyhow::Result;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::{Arc, Mutex};
use std::process::ChildStdin;
use std::io::Write;

pub const VIRTUAL_INPUT_A_NAME: &str = "mc-in-a";
pub const VIRTUAL_OUTPUT_A_NAME: &str = "mc-out-a";
pub const VIRTUAL_INPUT_B_NAME: &str = "mc-in-b";
pub const VIRTUAL_OUTPUT_B_NAME: &str = "mc-out-b";

/// Manages virtual MIDI ports with broadcast forwarding
/// Creates two independent port pairs (A and B) for message isolation
pub struct VirtualPorts {
    // Port pair A
    _input_connection_a: MidiInputConnection<()>,
    _output_connection_a: Arc<Mutex<MidiOutputConnection>>,
    input_outputs_a: Arc<Mutex<Vec<Arc<Mutex<MidiOutputConnection>>>>>,
    pipe_workers_a: Arc<Mutex<Vec<Arc<Mutex<ChildStdin>>>>>,

    // Port pair B
    _input_connection_b: MidiInputConnection<()>,
    _output_connection_b: Arc<Mutex<MidiOutputConnection>>,
    input_outputs_b: Arc<Mutex<Vec<Arc<Mutex<MidiOutputConnection>>>>>,
    pipe_workers_b: Arc<Mutex<Vec<Arc<Mutex<ChildStdin>>>>>,
}

impl VirtualPorts {
    /// Creates virtual MIDI input and output ports
    /// The ports will appear in the system as long as this struct is alive
    /// Creates two independent pairs (A and B) for message isolation
    #[cfg(unix)]
    pub fn create() -> Result<Self> {
        use midir::os::unix::{VirtualInput, VirtualOutput};

        // Create MIDI input and output objects for pair A
        let midi_in_a = MidiInput::new("mc-a")?;
        let midi_out_a = MidiOutput::new("mc-a")?;

        // Create virtual output port A
        let output_connection_a = midi_out_a
            .create_virtual(VIRTUAL_OUTPUT_A_NAME)
            .map_err(|e| anyhow::anyhow!("Failed to create virtual output A: {:?}", e))?;
        let output_shared_a = Arc::new(Mutex::new(output_connection_a));

        // Create broadcast lists for input A
        let input_outputs_a: Arc<Mutex<Vec<Arc<Mutex<MidiOutputConnection>>>>> = Arc::new(Mutex::new(Vec::new()));
        let outputs_for_callback_a = Arc::clone(&input_outputs_a);
        let pipe_workers_a: Arc<Mutex<Vec<Arc<Mutex<ChildStdin>>>>> = Arc::new(Mutex::new(Vec::new()));
        let pipes_for_callback_a = Arc::clone(&pipe_workers_a);

        // Create virtual input port A with callback
        let input_connection_a = midi_in_a
            .create_virtual(
                VIRTUAL_INPUT_A_NAME,
                move |_timestamp, message, _| {
                    // Forward to in-process outputs
                    if let Ok(outputs) = outputs_for_callback_a.lock() {
                        for output in outputs.iter() {
                            if let Ok(mut out) = output.lock() {
                                if let Err(e) = out.send(message) {
                                    eprintln!("Error forwarding from {}: {}", VIRTUAL_INPUT_A_NAME, e);
                                }
                            }
                        }
                    }
                    // Forward to pipe workers (for hot-plug support)
                    if let Ok(pipes) = pipes_for_callback_a.lock() {
                        for pipe in pipes.iter() {
                            if let Ok(mut p) = pipe.lock() {
                                let _ = p.write_all(message);
                                let _ = p.flush();
                            }
                        }
                    }
                },
                (),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create virtual input A: {:?}", e))?;

        // Create MIDI input and output objects for pair B
        let midi_in_b = MidiInput::new("mc-b")?;
        let midi_out_b = MidiOutput::new("mc-b")?;

        // Create virtual output port B
        let output_connection_b = midi_out_b
            .create_virtual(VIRTUAL_OUTPUT_B_NAME)
            .map_err(|e| anyhow::anyhow!("Failed to create virtual output B: {:?}", e))?;
        let output_shared_b = Arc::new(Mutex::new(output_connection_b));

        // Create broadcast lists for input B
        let input_outputs_b: Arc<Mutex<Vec<Arc<Mutex<MidiOutputConnection>>>>> = Arc::new(Mutex::new(Vec::new()));
        let outputs_for_callback_b = Arc::clone(&input_outputs_b);
        let pipe_workers_b: Arc<Mutex<Vec<Arc<Mutex<ChildStdin>>>>> = Arc::new(Mutex::new(Vec::new()));
        let pipes_for_callback_b = Arc::clone(&pipe_workers_b);

        // Create virtual input port B with callback
        let input_connection_b = midi_in_b
            .create_virtual(
                VIRTUAL_INPUT_B_NAME,
                move |_timestamp, message, _| {
                    // Forward to in-process outputs
                    if let Ok(outputs) = outputs_for_callback_b.lock() {
                        for output in outputs.iter() {
                            if let Ok(mut out) = output.lock() {
                                if let Err(e) = out.send(message) {
                                    eprintln!("Error forwarding from {}: {}", VIRTUAL_INPUT_B_NAME, e);
                                }
                            }
                        }
                    }
                    // Forward to pipe workers (for hot-plug support)
                    if let Ok(pipes) = pipes_for_callback_b.lock() {
                        for pipe in pipes.iter() {
                            if let Ok(mut p) = pipe.lock() {
                                let _ = p.write_all(message);
                                let _ = p.flush();
                            }
                        }
                    }
                },
                (),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create virtual input B: {:?}", e))?;

        Ok(VirtualPorts {
            _input_connection_a: input_connection_a,
            _output_connection_a: output_shared_a,
            input_outputs_a,
            pipe_workers_a,
            _input_connection_b: input_connection_b,
            _output_connection_b: output_shared_b,
            input_outputs_b,
            pipe_workers_b,
        })
    }

    /// Add an output connection to receive messages from virtual input
    /// Returns a handle that should be kept alive to maintain the connection
    pub fn add_virtual_input_output(&self, input_name: &str, output_name: &str) -> Result<Arc<Mutex<MidiOutputConnection>>> {
        // Determine which input's broadcast list to use
        let input_outputs = if input_name == VIRTUAL_INPUT_A_NAME {
            &self.input_outputs_a
        } else if input_name == VIRTUAL_INPUT_B_NAME {
            &self.input_outputs_b
        } else {
            return Err(anyhow::anyhow!("Unknown virtual input: {}", input_name));
        };

        // Special case: if output is one of our virtual outputs, use the existing connection
        if output_name == VIRTUAL_OUTPUT_A_NAME {
            let out_conn_shared = Arc::clone(&self._output_connection_a);

            // Add to the broadcast list
            if let Ok(mut outputs) = input_outputs.lock() {
                outputs.push(Arc::clone(&out_conn_shared));
            }

            return Ok(out_conn_shared);
        } else if output_name == VIRTUAL_OUTPUT_B_NAME {
            let out_conn_shared = Arc::clone(&self._output_connection_b);

            // Add to the broadcast list
            if let Ok(mut outputs) = input_outputs.lock() {
                outputs.push(Arc::clone(&out_conn_shared));
            }

            return Ok(out_conn_shared);
        }

        // Regular output port - spawn pipe worker subprocess (supports hot-plug)
        use std::process::{Command, Stdio};

        // Determine which pipe worker list to use
        let pipe_workers = if input_name == VIRTUAL_INPUT_A_NAME {
            &self.pipe_workers_a
        } else {
            &self.pipe_workers_b
        };

        // Get executable path
        let exe_path = std::env::current_exe()
            .map_err(|e| anyhow::anyhow!("Failed to get executable path: {}", e))?;

        // Spawn pipe worker subprocess with stderr redirected to log
        use std::fs::OpenOptions;
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/mc-pipe-worker.log")
            .ok();

        let mut cmd = Command::new(exe_path);
        cmd.arg("pipe-worker")
            .arg(output_name)
            .stdin(Stdio::piped());

        if let Some(log) = log_file {
            cmd.stderr(Stdio::from(log));
        }

        let mut child = cmd.spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn pipe worker: {}", e))?;

        // Take stdin handle for writing MIDI data
        let stdin = child.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to get pipe worker stdin"))?;

        let stdin_shared = Arc::new(Mutex::new(stdin));

        // Add to pipe workers list
        if let Ok(mut workers) = pipe_workers.lock() {
            workers.push(Arc::clone(&stdin_shared));
        }

        // Return a dummy MidiOutputConnection (manager expects this type)
        // The actual forwarding happens via the pipe worker
        // Try to connect to any available MIDI output port for the dummy connection
        let dummy_out = MidiOutput::new("mc-dummy")?;
        let out_ports = dummy_out.ports();

        // Try to find any non-virtual port first (avoid our own virtual ports)
        let dummy_port = out_ports.iter()
            .find(|p| {
                if let Ok(name) = dummy_out.port_name(p) {
                    name != VIRTUAL_INPUT_A_NAME && name != VIRTUAL_INPUT_B_NAME
                        && name != VIRTUAL_OUTPUT_A_NAME && name != VIRTUAL_OUTPUT_B_NAME
                } else {
                    false
                }
            })
            .or_else(|| out_ports.first()); // Fallback to any port

        if let Some(port) = dummy_port {
            let dummy_conn = dummy_out.connect(port, "mc-dummy")
                .map_err(|e| anyhow::anyhow!("Failed to create dummy connection: {:?}", e))?;
            return Ok(Arc::new(Mutex::new(dummy_conn)));
        }

        Err(anyhow::anyhow!("No MIDI output ports available for dummy connection"))
    }

    /// Remove an output connection from virtual input broadcast list
    pub fn remove_virtual_input_output(&self, input_name: &str, output: &Arc<Mutex<MidiOutputConnection>>) {
        let input_outputs = if input_name == VIRTUAL_INPUT_A_NAME {
            &self.input_outputs_a
        } else if input_name == VIRTUAL_INPUT_B_NAME {
            &self.input_outputs_b
        } else {
            return;
        };

        if let Ok(mut outputs) = input_outputs.lock() {
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

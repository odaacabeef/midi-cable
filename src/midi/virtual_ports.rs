use anyhow::Result;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Mutex};

pub const VIRTUAL_INPUT_NAME: &str = "mc-virtual-in";
pub const VIRTUAL_OUTPUT_NAME: &str = "mc-virtual-out";

/// Manages virtual MIDI ports with controllable forwarding from input to output
/// These connections must be kept alive for the virtual ports to remain available in the system
pub struct VirtualPorts {
    // Keep the connections alive - dropping them will destroy the virtual ports
    _input_connection: MidiInputConnection<()>,
    _output_connection: Arc<Mutex<MidiOutputConnection>>,
    // Control whether messages are forwarded
    forwarding_enabled: Arc<AtomicBool>,
}

impl VirtualPorts {
    /// Creates virtual MIDI input and output ports
    /// The ports will appear in the system as long as this struct is alive
    /// Messages sent to mc-virtual-in are forwarded to mc-virtual-out when enabled (default: enabled)
    #[cfg(unix)]
    pub fn create() -> Result<Self> {
        use midir::os::unix::{VirtualInput, VirtualOutput};

        // Create MIDI input and output objects
        let midi_in = MidiInput::new("mc")?;
        let midi_out = MidiOutput::new("mc")?;

        // Create virtual output port first
        // This port will appear as "mc-virtual-out" in the system
        // Other applications can receive MIDI from this port
        let output_connection = midi_out
            .create_virtual(VIRTUAL_OUTPUT_NAME)
            .map_err(|e| anyhow::anyhow!("Failed to create virtual output: {:?}", e))?;

        // Wrap output in Arc<Mutex> so we can use it in the input callback
        let output_shared = Arc::new(Mutex::new(output_connection));
        let output_for_callback = Arc::clone(&output_shared);

        // Create the forwarding control flag (enabled by default)
        let forwarding_enabled = Arc::new(AtomicBool::new(true));
        let forwarding_flag = Arc::clone(&forwarding_enabled);

        // Create virtual input port with forwarding callback
        // This port will appear as "mc-virtual-in" in the system
        // Other applications can send MIDI to this port
        let input_connection = midi_in
            .create_virtual(
                VIRTUAL_INPUT_NAME,
                move |_timestamp, message, _| {
                    // Only forward if forwarding is enabled
                    if forwarding_flag.load(Ordering::Relaxed) {
                        if let Ok(mut output) = output_for_callback.lock() {
                            if let Err(e) = output.send(message) {
                                eprintln!("Error forwarding virtual input to output: {}", e);
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
            forwarding_enabled,
        })
    }

    /// Enable forwarding from virtual input to virtual output
    pub fn enable_forwarding(&self) {
        self.forwarding_enabled.store(true, Ordering::Relaxed);
    }

    /// Disable forwarding from virtual input to virtual output
    pub fn disable_forwarding(&self) {
        self.forwarding_enabled.store(false, Ordering::Relaxed);
    }

    #[cfg(not(unix))]
    pub fn create() -> Result<Self> {
        Err(anyhow::anyhow!(
            "Virtual ports are only supported on Unix/macOS/Linux platforms"
        ))
    }
}

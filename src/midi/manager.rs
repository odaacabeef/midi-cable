use crate::connection::{Connection, ConnectionStatus, PortId};
use crate::events::AppEvent;
use crate::midi::forwarder::{start_forwarder, ForwarderHandle};
use crate::midi::virtual_ports::{VirtualPorts, VIRTUAL_INPUT_NAME, VIRTUAL_OUTPUT_NAME};
use crossbeam::channel::Sender;
use midir::MidiOutputConnection;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Manages MIDI ports and connections
pub struct MidiManager {
    pub virtual_ports: Option<VirtualPorts>,
    forwarders: HashMap<Connection, ForwarderHandle>,
    virtual_input_outputs: HashMap<Connection, Arc<Mutex<MidiOutputConnection>>>,
    event_tx: Sender<AppEvent>,
    monitoring_active: Arc<AtomicBool>,
}

impl MidiManager {
    /// Creates a new MIDI manager
    pub fn new(event_tx: Sender<AppEvent>) -> Self {
        Self {
            virtual_ports: None,
            forwarders: HashMap::new(),
            virtual_input_outputs: HashMap::new(),
            event_tx,
            monitoring_active: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Initialize virtual ports
    pub fn init_virtual_ports(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match VirtualPorts::create() {
            Ok(ports) => {
                self.virtual_ports = Some(ports);
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to create virtual ports: {}", e);
                Err(e.into())
            }
        }
    }

    /// Lists all available MIDI input ports (sources we can receive from)
    /// Returns hardware outputs (sources) + virtual input created by this app
    pub fn list_input_ports() -> Vec<PortId> {
        use midir::{MidiInput, MidiOutput};
        use std::collections::HashSet;

        let mut ports = HashSet::new();

        // Get hardware outputs (sources we can read from), excluding our virtual ports
        if let Ok(midi_in) = MidiInput::new("mc-list") {
            for port in midi_in.ports().iter() {
                if let Ok(name) = midi_in.port_name(port) {
                    if name != VIRTUAL_OUTPUT_NAME && name != VIRTUAL_INPUT_NAME {
                        ports.insert(PortId::new(name, false));
                    }
                }
            }
        }

        // Add virtual input created by this app (appears as destination in MidiOutput.ports)
        if let Ok(midi_out) = MidiOutput::new("mc-list") {
            for port in midi_out.ports().iter() {
                if let Ok(name) = midi_out.port_name(port) {
                    if name == VIRTUAL_INPUT_NAME {
                        ports.insert(PortId::new(name, true));
                    }
                }
            }
        }

        let mut sorted: Vec<_> = ports.into_iter().collect();
        // Sort: virtual ports first, then alphabetically by name
        sorted.sort_by(|a, b| {
            match (a.is_virtual, b.is_virtual) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        sorted
    }

    /// Lists all available MIDI output ports (destinations we can send to)
    /// Returns hardware inputs (destinations) + virtual output created by this app
    pub fn list_output_ports() -> Vec<PortId> {
        use midir::{MidiInput, MidiOutput};
        use std::collections::HashSet;

        let mut ports = HashSet::new();

        // Get hardware inputs (destinations we can write to), excluding our virtual ports
        if let Ok(midi_out) = MidiOutput::new("mc-list") {
            for port in midi_out.ports().iter() {
                if let Ok(name) = midi_out.port_name(port) {
                    if name != VIRTUAL_INPUT_NAME && name != VIRTUAL_OUTPUT_NAME {
                        ports.insert(PortId::new(name, false));
                    }
                }
            }
        }

        // Add virtual output created by this app (appears as source in MidiInput.ports)
        if let Ok(midi_in) = MidiInput::new("mc-list") {
            for port in midi_in.ports().iter() {
                if let Ok(name) = midi_in.port_name(port) {
                    if name == VIRTUAL_OUTPUT_NAME {
                        ports.insert(PortId::new(name, true));
                    }
                }
            }
        }

        let mut sorted: Vec<_> = ports.into_iter().collect();
        // Sort: virtual ports first, then alphabetically by name
        sorted.sort_by(|a, b| {
            match (a.is_virtual, b.is_virtual) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });
        sorted
    }

    /// Starts a new MIDI connection
    pub fn start_connection(
        &mut self,
        connection: Connection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if connection already exists
        if self.forwarders.contains_key(&connection) || self.virtual_input_outputs.contains_key(&connection) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Connection already exists"
            )) as Box<dyn std::error::Error>);
        }

        // Check if input is the virtual input created by this app
        if connection.input.name == VIRTUAL_INPUT_NAME && connection.input.is_virtual {
            // Use virtual port broadcast instead of regular forwarder
            if let Some(ref virtual_ports) = self.virtual_ports {
                let output_handle = virtual_ports.add_virtual_input_output(&connection.output.name)?;
                self.virtual_input_outputs.insert(connection.clone(), output_handle);
                let _ = self.event_tx.send(AppEvent::ConnectionStatus);
                return Ok(());
            } else {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Virtual ports not initialized"
                )) as Box<dyn std::error::Error>);
            }
        }

        // Regular connection - start the forwarder
        let handle = start_forwarder(
            connection.clone(),
            &connection.input.name,
            &connection.output.name,
            self.event_tx.clone(),
        )?;

        self.forwarders.insert(connection.clone(), handle);

        let _ = self.event_tx.send(AppEvent::ConnectionStatus);

        Ok(())
    }

    /// Stops a MIDI connection
    pub fn stop_connection(&mut self, connection: &Connection) {
        // Check if it's a virtual input connection
        if let Some(output_handle) = self.virtual_input_outputs.remove(connection) {
            // Remove from virtual ports broadcast list
            if let Some(ref virtual_ports) = self.virtual_ports {
                virtual_ports.remove_virtual_input_output(&output_handle);
            }
            // The handle will be dropped, closing the MIDI connection
            return;
        }

        // Regular forwarder connection
        if let Some(_handle) = self.forwarders.remove(connection) {
            // The forwarder will be dropped, closing the MIDI connections
        }
    }

    /// Gets the status of all active connections
    pub fn get_connection_statuses(&self) -> HashMap<Connection, ConnectionStatus> {
        let mut statuses: HashMap<Connection, ConnectionStatus> = self.forwarders
            .keys()
            .map(|conn| (conn.clone(), ConnectionStatus::Active))
            .collect();

        // Add virtual input connections
        for conn in self.virtual_input_outputs.keys() {
            statuses.insert(conn.clone(), ConnectionStatus::Active);
        }

        statuses
    }


    /// Start monitoring for MIDI port changes (hot-plug detection)
    /// Uses subprocess-based monitoring to get fresh device enumeration
    pub fn start_port_monitoring(&mut self) {
        self.monitoring_active.store(true, Ordering::Relaxed);

        #[cfg(target_os = "macos")]
        {
            use crate::midi::monitor::macos;
            if let Err(e) = macos::start_monitor(self.event_tx.clone()) {
                eprintln!("Failed to start MIDI monitor: {}", e);
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            use crate::midi::monitor::other;
            if let Err(e) = other::start_monitor(self.event_tx.clone()) {
                eprintln!("Failed to start MIDI monitor: {}", e);
            }
        }
    }

    /// Stop monitoring for MIDI port changes
    pub fn stop_port_monitoring(&self) {
        self.monitoring_active.store(false, Ordering::Relaxed);
    }
}

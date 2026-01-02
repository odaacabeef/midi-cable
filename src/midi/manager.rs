use crate::connection::{Connection, ConnectionStatus, PortId};
use crate::events::AppEvent;
use crate::midi::forwarder::{start_forwarder, ForwarderHandle};
use crate::midi::virtual_ports::{VirtualPorts, VIRTUAL_INPUT_NAME, VIRTUAL_OUTPUT_NAME};
use crossbeam::channel::Sender;
use midir::{MidiInput, MidiOutput};
use std::collections::HashMap;

/// Manages MIDI ports and connections
pub struct MidiManager {
    pub virtual_ports: Option<VirtualPorts>,
    forwarders: HashMap<Connection, ForwarderHandle>,
    event_tx: Sender<AppEvent>,
}

impl MidiManager {
    /// Creates a new MIDI manager
    pub fn new(event_tx: Sender<AppEvent>) -> Self {
        Self {
            virtual_ports: None,
            forwarders: HashMap::new(),
            event_tx,
        }
    }

    /// Initialize virtual ports
    pub fn init_virtual_ports(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match VirtualPorts::create() {
            Ok(ports) => {
                self.virtual_ports = Some(ports);
                let _ = self.event_tx.send(AppEvent::Log(format!(
                    "Created virtual ports: {} and {}",
                    VIRTUAL_INPUT_NAME, VIRTUAL_OUTPUT_NAME
                )));
                Ok(())
            }
            Err(e) => {
                let _ = self
                    .event_tx
                    .send(AppEvent::Error(format!("Failed to create virtual ports: {}", e)));
                Err(e.into())
            }
        }
    }

    /// Lists all available MIDI input ports
    /// Returns an empty list if MIDI system is not available
    pub fn list_input_ports() -> Vec<PortId> {
        match MidiInput::new("mc-list") {
            Ok(midi_in) => {
                midi_in.ports()
                    .iter()
                    .filter_map(|port| {
                        midi_in.port_name(port).ok().map(|name| {
                            let is_virtual = name == VIRTUAL_INPUT_NAME || name == VIRTUAL_OUTPUT_NAME;
                            PortId::new(name, is_virtual)
                        })
                    })
                    .collect()
            }
            Err(_) => {
                // MIDI system not available - return empty list
                Vec::new()
            }
        }
    }

    /// Lists all available MIDI output ports
    /// Returns an empty list if MIDI system is not available
    pub fn list_output_ports() -> Vec<PortId> {
        match MidiOutput::new("mc-list") {
            Ok(midi_out) => {
                midi_out.ports()
                    .iter()
                    .filter_map(|port| {
                        midi_out.port_name(port).ok().map(|name| {
                            let is_virtual = name == VIRTUAL_INPUT_NAME || name == VIRTUAL_OUTPUT_NAME;
                            PortId::new(name, is_virtual)
                        })
                    })
                    .collect()
            }
            Err(_) => {
                // MIDI system not available - return empty list
                Vec::new()
            }
        }
    }

    /// Starts a new MIDI connection
    pub fn start_connection(
        &mut self,
        connection: Connection,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check if connection already exists
        if self.forwarders.contains_key(&connection) {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "Connection already exists"
            )) as Box<dyn std::error::Error>);
        }

        // Start the forwarder
        let handle = start_forwarder(
            connection.clone(),
            &connection.input.name,
            &connection.output.name,
            self.event_tx.clone(),
        )?;

        self.forwarders.insert(connection.clone(), handle);

        let _ = self.event_tx.send(AppEvent::ConnectionStatus {
            connection,
            status: "Active".to_string(),
        });

        Ok(())
    }

    /// Stops a MIDI connection
    pub fn stop_connection(&mut self, connection: &Connection) {
        if let Some(_handle) = self.forwarders.remove(connection) {
            // The forwarder will be dropped, closing the MIDI connections
            let _ = self.event_tx.send(AppEvent::Log(format!(
                "Stopped connection: {}",
                connection
            )));
        }
    }

    /// Gets the status of all active connections
    pub fn get_connection_statuses(&self) -> HashMap<Connection, ConnectionStatus> {
        self.forwarders
            .keys()
            .map(|conn| (conn.clone(), ConnectionStatus::Active))
            .collect()
    }

    /// Checks if a connection exists
    pub fn has_connection(&self, connection: &Connection) -> bool {
        self.forwarders.contains_key(connection)
    }
}

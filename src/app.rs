use crate::connection::{Connection, ConnectionStatus, PortId};
use crate::events::AppEvent;
use crate::midi::MidiManager;
use crossbeam::channel::{Receiver, Sender};

#[derive(Debug, Clone)]
pub enum UiState {
    /// Browsing inputs with cursor
    Idle {
        cursor_idx: usize,
    },
    /// Selected an input, now selecting outputs
    SelectingOutputs {
        input_idx: usize,
        selected_outputs: Vec<usize>,
        cursor_idx: usize,
    },
}

pub struct App {
    pub midi_inputs: Vec<PortId>,
    pub midi_outputs: Vec<PortId>,
    pub active_connections: Vec<(Connection, ConnectionStatus)>,
    pub ui_state: UiState,
    pub should_quit: bool,

    midi_manager: MidiManager,
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
    show_virtual_connection: bool,
    virtual_connection: Option<Connection>,
}

impl App {
    pub fn new() -> Self {
        let (event_tx, event_rx) = crossbeam::channel::unbounded();
        let midi_manager = MidiManager::new(event_tx.clone());

        Self {
            midi_inputs: Vec::new(),
            midi_outputs: Vec::new(),
            active_connections: Vec::new(),
            ui_state: UiState::Idle { cursor_idx: 0 },
            should_quit: false,
            midi_manager,
            event_tx,
            event_rx,
            show_virtual_connection: false,
            virtual_connection: None,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Try to initialize virtual ports (optional, won't fail if not supported)
        let virtual_ports_created = self.midi_manager.init_virtual_ports().is_ok();

        // Refresh port lists
        self.refresh_ports();

        // If virtual ports were created, establish the default connection
        if virtual_ports_created {
            use crate::midi::virtual_ports::{VIRTUAL_INPUT_NAME, VIRTUAL_OUTPUT_NAME};

            // Find the virtual input and output
            let virtual_input = self.midi_inputs.iter()
                .find(|p| p.name == VIRTUAL_INPUT_NAME)
                .cloned();
            let virtual_output = self.midi_outputs.iter()
                .find(|p| p.name == VIRTUAL_OUTPUT_NAME)
                .cloned();

            // Create the default connection if both ports exist
            if let (Some(input), Some(output)) = (virtual_input, virtual_output) {
                let connection = Connection::new(input, output);
                // The virtual ports have built-in forwarding via the callback in virtual_ports.rs,
                // so we track this connection separately and don't create a forwarder
                self.virtual_connection = Some(connection);
                self.show_virtual_connection = true;
                // Update the connection list to show the virtual connection
                self.update_connection_list();
            }
        }

        // Start monitoring for port changes (hot-plug detection)
        self.midi_manager.start_port_monitoring();

        Ok(())
    }


    pub fn refresh_ports(&mut self) {
        self.midi_inputs = MidiManager::list_input_ports();
        self.midi_outputs = MidiManager::list_output_ports();
    }

    pub fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                AppEvent::ConnectionStatus { .. } => {
                    self.update_connection_list();
                }
                AppEvent::PortsChanged => {
                    self.handle_ports_changed();
                }
                AppEvent::PortListUpdate { inputs, outputs } => {
                    // Directly update port lists from subprocess enumeration
                    self.midi_inputs = inputs;
                    self.midi_outputs = outputs;
                    // Clean up stale connections
                    self.cleanup_stale_connections();
                }
                _ => {}
            }
        }
    }

    fn handle_ports_changed(&mut self) {
        // Refresh port lists
        self.refresh_ports();
        self.cleanup_stale_connections();
    }

    fn cleanup_stale_connections(&mut self) {
        // Find connections that reference removed ports
        let stale_connections: Vec<Connection> = self
            .active_connections
            .iter()
            .filter(|(conn, _)| {
                // Check if input port still exists
                let input_exists = self.midi_inputs.iter().any(|p| p == &conn.input);
                // Check if output port still exists
                let output_exists = self.midi_outputs.iter().any(|p| p == &conn.output);
                // Connection is stale if either port is missing
                !input_exists || !output_exists
            })
            .map(|(conn, _)| conn.clone())
            .collect();

        // Stop all stale connections
        for conn in stale_connections {
            self.stop_connection(&conn);
        }

        // Update the connection list to reflect changes
        self.update_connection_list();
    }


    fn update_connection_list(&mut self) {
        let statuses = self.midi_manager.get_connection_statuses();
        self.active_connections = statuses.into_iter().collect();

        // Add virtual connection if it should be shown
        if self.show_virtual_connection {
            if let Some(ref conn) = self.virtual_connection {
                self.active_connections.push((conn.clone(), ConnectionStatus::Active));
            }
        }
    }

    pub fn start_connection(&mut self, connection: Connection) -> Result<(), Box<dyn std::error::Error>> {
        // Check if this is the virtual connection
        if let Some(ref virtual_conn) = self.virtual_connection {
            if &connection == virtual_conn {
                // Enable forwarding and show it in the UI
                self.midi_manager.enable_virtual_forwarding();
                self.show_virtual_connection = true;
                self.update_connection_list();
                return Ok(());
            }
        }

        // Regular connection - start the forwarder
        self.midi_manager.start_connection(connection.clone())?;
        self.update_connection_list();
        Ok(())
    }

    pub fn stop_connection(&mut self, connection: &Connection) {
        // Check if this is the virtual connection
        if let Some(ref virtual_conn) = self.virtual_connection {
            if connection == virtual_conn {
                // Disable forwarding and hide from the UI
                self.midi_manager.disable_virtual_forwarding();
                self.show_virtual_connection = false;
                self.update_connection_list();
                return;
            }
        }

        // Regular connection - stop the forwarder
        self.midi_manager.stop_connection(connection);
        self.update_connection_list();
    }

    /// Get outputs connected to a specific input
    pub fn get_connected_outputs(&self, input: &PortId) -> Vec<PortId> {
        self.active_connections
            .iter()
            .filter(|(conn, _)| &conn.input == input)
            .map(|(conn, _)| conn.output.clone())
            .collect()
    }

    /// Check if an output is connected to a specific input
    pub fn is_output_connected(&self, input: &PortId, output: &PortId) -> bool {
        self.active_connections
            .iter()
            .any(|(conn, _)| &conn.input == input && &conn.output == output)
    }

    // Keyboard input handlers

    pub fn handle_key_up(&mut self) {
        match &mut self.ui_state {
            UiState::Idle { cursor_idx } if *cursor_idx > 0 => {
                *cursor_idx -= 1;
            }
            UiState::SelectingOutputs { cursor_idx, .. } if *cursor_idx > 0 => {
                *cursor_idx -= 1;
            }
            _ => {}
        }
    }

    pub fn handle_key_down(&mut self) {
        match &mut self.ui_state {
            UiState::Idle { cursor_idx } if *cursor_idx < self.midi_inputs.len().saturating_sub(1) => {
                *cursor_idx += 1;
            }
            UiState::SelectingOutputs { cursor_idx, .. } if *cursor_idx < self.midi_outputs.len().saturating_sub(1) => {
                *cursor_idx += 1;
            }
            _ => {}
        }
    }

    pub fn handle_space(&mut self) {
        match self.ui_state.clone() {
            UiState::Idle { cursor_idx } => {
                // Select the input and jump to output selection
                if let Some(input) = self.midi_inputs.get(cursor_idx) {
                    // Get currently connected outputs for this input
                    let selected_outputs: Vec<usize> = self
                        .get_connected_outputs(input)
                        .iter()
                        .filter_map(|out| {
                            self.midi_outputs.iter().position(|o| o == out)
                        })
                        .collect();

                    self.ui_state = UiState::SelectingOutputs {
                        input_idx: cursor_idx,
                        selected_outputs,
                        cursor_idx: 0,
                    };
                }
            }
            UiState::SelectingOutputs {
                mut selected_outputs,
                cursor_idx,
                input_idx,
            } => {
                // Toggle output selection
                if let Some(pos) = selected_outputs.iter().position(|&idx| idx == cursor_idx) {
                    selected_outputs.remove(pos);
                } else {
                    selected_outputs.push(cursor_idx);
                }

                self.ui_state = UiState::SelectingOutputs {
                    input_idx,
                    selected_outputs,
                    cursor_idx,
                };
            }
        }
    }

    pub fn handle_enter(&mut self) {
        if let UiState::SelectingOutputs {
            input_idx,
            selected_outputs,
            ..
        } = &self.ui_state.clone()
        {
            if let Some(input) = self.midi_inputs.get(*input_idx).cloned() {
                // Remove all existing connections for this input
                let existing_connections: Vec<Connection> = self
                    .active_connections
                    .iter()
                    .filter(|(conn, _)| &conn.input == &input)
                    .map(|(conn, _)| conn.clone())
                    .collect();

                for conn in existing_connections {
                    self.stop_connection(&conn);
                }

                // Create new connections for selected outputs
                for &output_idx in selected_outputs {
                    if let Some(output) = self.midi_outputs.get(output_idx).cloned() {
                        let connection = Connection::new(input.clone(), output);
                        let _ = self.start_connection(connection);
                    }
                }
            }

            // Return to idle
            self.ui_state = UiState::Idle { cursor_idx: 0 };
        }
    }

    pub fn handle_escape(&mut self) {
        // Cancel selection and return to idle
        if matches!(&self.ui_state, UiState::SelectingOutputs { .. }) {
            self.ui_state = UiState::Idle { cursor_idx: 0 };
        }
    }

    pub fn handle_refresh(&mut self) {
        // Manually refresh the port lists and clean up stale connections
        self.handle_ports_changed();
    }

    pub fn quit(&mut self) {
        // Stop port monitoring
        self.midi_manager.stop_port_monitoring();
        self.should_quit = true;
    }
}

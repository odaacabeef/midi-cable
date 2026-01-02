use crate::connection::{Connection, ConnectionStatus, PortId};
use crate::events::AppEvent;
use crate::midi::MidiManager;
use crossbeam::channel::{Receiver, Sender};
use std::collections::VecDeque;

const MAX_LOG_MESSAGES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneFocus {
    Inputs,
    Outputs,
    Connections,
}

#[derive(Debug, Clone)]
pub enum UiState {
    Idle {
        focus: PaneFocus,
        selected_input_idx: usize,
        selected_output_idx: usize,
        selected_connection_idx: usize,
    },
    SelectingSource {
        selected_input_idx: usize,
    },
    SelectingDestination {
        source: PortId,
        selected_output_idx: usize,
    },
}

pub struct App {
    pub midi_inputs: Vec<PortId>,
    pub midi_outputs: Vec<PortId>,
    pub active_connections: Vec<(Connection, ConnectionStatus)>,
    pub ui_state: UiState,
    pub log_messages: VecDeque<String>,
    pub should_quit: bool,

    midi_manager: MidiManager,
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
}

impl App {
    pub fn new() -> Self {
        let (event_tx, event_rx) = crossbeam::channel::unbounded();
        let midi_manager = MidiManager::new(event_tx.clone());

        Self {
            midi_inputs: Vec::new(),
            midi_outputs: Vec::new(),
            active_connections: Vec::new(),
            ui_state: UiState::Idle {
                focus: PaneFocus::Inputs,
                selected_input_idx: 0,
                selected_output_idx: 0,
                selected_connection_idx: 0,
            },
            log_messages: VecDeque::new(),
            should_quit: false,
            midi_manager,
            event_tx,
            event_rx,
        }
    }

    pub fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Try to initialize virtual ports (optional, won't fail if not supported)
        if let Err(e) = self.midi_manager.init_virtual_ports() {
            self.add_log(format!("Note: Virtual ports not available: {}", e));
        }

        // Refresh port lists
        self.refresh_ports();

        // Add welcome message
        if self.midi_inputs.is_empty() && self.midi_outputs.is_empty() {
            self.add_log("No MIDI devices found. Connect a MIDI device or enable IAC Driver.".to_string());
        } else {
            self.add_log(format!("Found {} input(s) and {} output(s)",
                self.midi_inputs.len(), self.midi_outputs.len()));
        }

        // Create default connection from virtual input to virtual output
        // Note: This might fail if the ports aren't ready yet or if connecting virtual ports
        // to each other isn't supported. We log but don't fail initialization.
        if let Err(e) = self.create_default_connection() {
            self.add_log(format!("Note: Could not create default virtual connection: {}", e));
        }

        Ok(())
    }

    fn create_default_connection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let virtual_in = self
            .midi_inputs
            .iter()
            .find(|p| p.name == crate::midi::VIRTUAL_INPUT_NAME)
            .cloned();

        let virtual_out = self
            .midi_outputs
            .iter()
            .find(|p| p.name == crate::midi::VIRTUAL_OUTPUT_NAME)
            .cloned();

        if let (Some(input), Some(output)) = (virtual_in, virtual_out) {
            let connection = Connection::new(input, output);
            self.start_connection(connection)?;
        }

        Ok(())
    }

    pub fn refresh_ports(&mut self) {
        self.midi_inputs = MidiManager::list_input_ports();
        self.midi_outputs = MidiManager::list_output_ports();
    }

    pub fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                AppEvent::Log(msg) => {
                    self.add_log(msg);
                }
                AppEvent::Error(msg) => {
                    self.add_log(format!("ERROR: {}", msg));
                }
                AppEvent::ConnectionStatus { connection, status } => {
                    self.add_log(format!("{}: {}", connection, status));
                    self.update_connection_list();
                }
                AppEvent::PortsChanged => {
                    self.refresh_ports();
                }
                _ => {}
            }
        }
    }

    fn add_log(&mut self, msg: String) {
        self.log_messages.push_back(msg);
        if self.log_messages.len() > MAX_LOG_MESSAGES {
            self.log_messages.pop_front();
        }
    }

    fn update_connection_list(&mut self) {
        let statuses = self.midi_manager.get_connection_statuses();
        self.active_connections = statuses.into_iter().collect();
    }

    pub fn start_connection(&mut self, connection: Connection) -> Result<(), Box<dyn std::error::Error>> {
        self.midi_manager.start_connection(connection.clone())?;
        self.update_connection_list();
        Ok(())
    }

    pub fn stop_connection(&mut self, connection: &Connection) {
        self.midi_manager.stop_connection(connection);
        self.update_connection_list();
    }

    pub fn get_selected_input_idx(&self) -> Option<usize> {
        match &self.ui_state {
            UiState::Idle { selected_input_idx, focus, .. } if *focus == PaneFocus::Inputs => {
                Some(*selected_input_idx)
            }
            UiState::SelectingSource { selected_input_idx } => Some(*selected_input_idx),
            _ => None,
        }
    }

    pub fn get_selected_output_idx(&self) -> Option<usize> {
        match &self.ui_state {
            UiState::Idle { selected_output_idx, focus, .. } if *focus == PaneFocus::Outputs => {
                Some(*selected_output_idx)
            }
            UiState::SelectingDestination { selected_output_idx, .. } => Some(*selected_output_idx),
            _ => None,
        }
    }

    // Keyboard input handlers

    pub fn handle_key_up(&mut self) {
        match &mut self.ui_state {
            UiState::Idle { focus, selected_input_idx, selected_output_idx, selected_connection_idx } => {
                match focus {
                    PaneFocus::Inputs if *selected_input_idx > 0 => *selected_input_idx -= 1,
                    PaneFocus::Outputs if *selected_output_idx > 0 => *selected_output_idx -= 1,
                    PaneFocus::Connections if *selected_connection_idx > 0 => *selected_connection_idx -= 1,
                    _ => {}
                }
            }
            UiState::SelectingSource { selected_input_idx } if *selected_input_idx > 0 => {
                *selected_input_idx -= 1
            }
            UiState::SelectingDestination { selected_output_idx, .. } if *selected_output_idx > 0 => {
                *selected_output_idx -= 1
            }
            _ => {}
        }
    }

    pub fn handle_key_down(&mut self) {
        match &mut self.ui_state {
            UiState::Idle { focus, selected_input_idx, selected_output_idx, selected_connection_idx } => {
                match focus {
                    PaneFocus::Inputs if *selected_input_idx < self.midi_inputs.len().saturating_sub(1) => {
                        *selected_input_idx += 1
                    }
                    PaneFocus::Outputs if *selected_output_idx < self.midi_outputs.len().saturating_sub(1) => {
                        *selected_output_idx += 1
                    }
                    PaneFocus::Connections if *selected_connection_idx < self.active_connections.len().saturating_sub(1) => {
                        *selected_connection_idx += 1
                    }
                    _ => {}
                }
            }
            UiState::SelectingSource { selected_input_idx } if *selected_input_idx < self.midi_inputs.len().saturating_sub(1) => {
                *selected_input_idx += 1
            }
            UiState::SelectingDestination { selected_output_idx, .. } if *selected_output_idx < self.midi_outputs.len().saturating_sub(1) => {
                *selected_output_idx += 1
            }
            _ => {}
        }
    }

    pub fn handle_tab(&mut self) {
        if let UiState::Idle { focus, selected_input_idx, selected_output_idx, selected_connection_idx } = &self.ui_state {
            let new_focus = match focus {
                PaneFocus::Inputs => PaneFocus::Outputs,
                PaneFocus::Outputs => PaneFocus::Connections,
                PaneFocus::Connections => PaneFocus::Inputs,
            };

            self.ui_state = UiState::Idle {
                focus: new_focus,
                selected_input_idx: *selected_input_idx,
                selected_output_idx: *selected_output_idx,
                selected_connection_idx: *selected_connection_idx,
            };
        }
    }

    pub fn handle_space(&mut self) {
        match &self.ui_state {
            UiState::Idle { focus: PaneFocus::Inputs, selected_input_idx, .. } => {
                // Start selecting a source
                self.ui_state = UiState::SelectingSource {
                    selected_input_idx: *selected_input_idx,
                };
            }
            UiState::SelectingSource { selected_input_idx } => {
                // Move to destination selection
                if let Some(source) = self.midi_inputs.get(*selected_input_idx).cloned() {
                    self.ui_state = UiState::SelectingDestination {
                        source,
                        selected_output_idx: 0,
                    };
                }
            }
            UiState::SelectingDestination { source, selected_output_idx } => {
                // Create connection
                if let Some(dest) = self.midi_outputs.get(*selected_output_idx).cloned() {
                    let connection = Connection::new(source.clone(), dest);
                    let _ = self.start_connection(connection);
                }

                // Return to idle
                self.ui_state = UiState::Idle {
                    focus: PaneFocus::Inputs,
                    selected_input_idx: 0,
                    selected_output_idx: 0,
                    selected_connection_idx: 0,
                };
            }
            _ => {}
        }
    }

    pub fn handle_delete(&mut self) {
        if let UiState::Idle { focus: PaneFocus::Connections, selected_connection_idx, .. } = &self.ui_state {
            if let Some((connection, _)) = self.active_connections.get(*selected_connection_idx) {
                let connection = connection.clone();
                self.stop_connection(&connection);
            }
        }
    }

    pub fn handle_escape(&mut self) {
        // Cancel selection and return to idle
        if !matches!(&self.ui_state, UiState::Idle { .. }) {
            self.ui_state = UiState::Idle {
                focus: PaneFocus::Inputs,
                selected_input_idx: 0,
                selected_output_idx: 0,
                selected_connection_idx: 0,
            };
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }
}

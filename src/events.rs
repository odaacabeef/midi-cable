use crate::connection::{Connection, PortId};

/// Events sent between threads
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Request to start a new connection
    StartConnection(Connection),

    /// Request to stop a connection
    StopConnection(Connection),

    /// Connection status update
    ConnectionStatus {
        connection: Connection,
        status: String,
    },

    /// Error occurred
    Error(String),

    /// Log message
    Log(String),

    /// MIDI ports changed (hotplug event)
    PortsChanged,

    /// Updated port lists from subprocess enumeration
    PortListUpdate {
        inputs: Vec<PortId>,
        outputs: Vec<PortId>,
    },
}

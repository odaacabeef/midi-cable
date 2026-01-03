use crate::connection::PortId;

/// Events sent between threads
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Connection status update
    ConnectionStatus,

    /// Updated port lists from subprocess enumeration
    PortListUpdate {
        inputs: Vec<PortId>,
        outputs: Vec<PortId>,
    },
}

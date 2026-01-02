use std::fmt;

/// Unique identifier for MIDI ports
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PortId {
    pub name: String,
    pub is_virtual: bool,
}

impl PortId {
    pub fn new(name: String, is_virtual: bool) -> Self {
        Self { name, is_virtual }
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Represents a connection from one input to one output
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Connection {
    pub input: PortId,
    pub output: PortId,
}

impl Connection {
    pub fn new(input: PortId, output: PortId) -> Self {
        Self { input, output }
    }
}

impl fmt::Display for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} → {}", self.input, self.output)
    }
}

/// Status of a connection
#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Active,
    Error(String),
}

impl fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionStatus::Active => write!(f, "OK"),
            ConnectionStatus::Error(err) => write!(f, "ERR: {}", err),
        }
    }
}

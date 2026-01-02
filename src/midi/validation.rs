/// Validates the length of a MIDI message based on its type
/// Ported from Go's fwd.go lines 111-143
pub fn is_valid_midi_message(msg: &[u8]) -> bool {
    if msg.is_empty() {
        return false;
    }

    let status = msg[0] & 0xF0; // Get the message type (high nibble)

    match status {
        // Note Off, Note On, Poly Pressure, Control Change, Pitch Bend
        0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0 => msg.len() == 3,

        // Program Change, Channel Pressure
        0xC0 | 0xD0 => msg.len() == 2,

        // System messages
        0xF0 => validate_system_message(msg),

        _ => false,
    }
}

/// Validates system messages (0xF0 status byte)
fn validate_system_message(msg: &[u8]) -> bool {
    if msg.len() < 2 {
        return false;
    }

    match msg[1] {
        // SysEx start - variable length
        0xF0 => true,

        // MIDI Time Code, Song Select
        0xF1 | 0xF3 => msg.len() == 2,

        // Song Position Pointer
        0xF2 => msg.len() == 3,

        // Tune Request, Clock, Start, Continue, Stop, Active Sensing, Reset
        0xF6 | 0xF7 | 0xF8 | 0xFA | 0xFB | 0xFC => msg.len() == 1,

        _ => false,
    }
}

/// Checks if a message is a Program Change message
/// Program Change messages need special handling (truncate to 2 bytes if longer)
pub fn is_program_change(msg: &[u8]) -> bool {
    !msg.is_empty() && (msg[0] & 0xF0) == 0xC0
}

/// Truncates a Program Change message to 2 bytes if needed
/// Returns the correct message to forward
pub fn normalize_program_change(msg: &[u8]) -> Vec<u8> {
    if msg.len() >= 2 {
        msg[..2].to_vec()
    } else {
        msg.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_on() {
        // Note On with 3 bytes (valid)
        assert!(is_valid_midi_message(&[0x90, 0x3C, 0x64]));

        // Note On with wrong length
        assert!(!is_valid_midi_message(&[0x90, 0x3C]));
        assert!(!is_valid_midi_message(&[0x90]));
    }

    #[test]
    fn test_program_change() {
        // Program Change with 2 bytes (valid)
        assert!(is_valid_midi_message(&[0xC0, 0x05]));

        // Program Change with wrong length
        assert!(!is_valid_midi_message(&[0xC0]));
        assert!(!is_valid_midi_message(&[0xC0, 0x05, 0x00]));

        // Check program change detection
        assert!(is_program_change(&[0xC0, 0x05]));
        assert!(is_program_change(&[0xC0, 0x05, 0x00]));
        assert!(!is_program_change(&[0x90, 0x3C, 0x64]));

        // Normalize program change
        assert_eq!(normalize_program_change(&[0xC0, 0x05, 0x00]), vec![0xC0, 0x05]);
        assert_eq!(normalize_program_change(&[0xC0, 0x05]), vec![0xC0, 0x05]);
    }

    #[test]
    fn test_control_change() {
        // Control Change with 3 bytes (valid)
        assert!(is_valid_midi_message(&[0xB0, 0x07, 0x7F]));

        // Control Change with wrong length
        assert!(!is_valid_midi_message(&[0xB0, 0x07]));
    }

    #[test]
    fn test_system_messages() {
        // MIDI Time Code (2 bytes)
        assert!(is_valid_midi_message(&[0xF0, 0xF1]));

        // Song Position Pointer (3 bytes)
        assert!(is_valid_midi_message(&[0xF0, 0xF2, 0x00]));

        // Clock (1 byte)
        assert!(is_valid_midi_message(&[0xF0, 0xF8]));
    }
}

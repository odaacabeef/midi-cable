# MIDI Cable

TUI application for managing MIDI message routing between devices.

## Features

- **Interactive TUI**: Terminal-based interface for managing MIDI connections
- **Multiple Simultaneous Connections**: Route MIDI from multiple inputs to multiple outputs at once
- **Virtual MIDI Ports**: Creates `mc-virtual-in` and `mc-virtual-out` ports automatically
- **Real-time Message Forwarding**: Low-latency MIDI message routing
- **Message Validation**: Validates MIDI messages before forwarding
- **Program Change Handling**: Special handling for Program Change messages
- **Connection Management**: Add and remove connections on the fly

## Usage

Run the application:

```bash
cargo run --release
# or after installation
mc
```

### Keyboard Controls

- **↑/↓** - Navigate through lists
- **Tab** - Switch between panes (Inputs, Outputs, Connections)
- **Space** - Select input source, then select output destination to create connection
- **d** - Delete selected connection
- **q** or **Ctrl+C** - Quit

### Interface Layout

```
┌─────────────────────────────────────────────┐
│ MIDI Cable - Routing Matrix                 │
├─────────────────────┬───────────────────────┤
│ INPUTS              │ OUTPUTS               │
│ [>] IAC Driver Bus 1│ [ ] IAC Driver Bus 1  │
│ [ ] MIDI Device 1   │ [ ] MIDI Device 2     │
├─────────────────────────────────────────────┤
│ ACTIVE CONNECTIONS                          │
│ • MIDI Device 1 → MIDI Device 2 [OK]       │
├─────────────────────────────────────────────┤
│ LOG                                         │
│ Started connection...                       │
├─────────────────────────────────────────────┤
│ [↑↓] Navigate | [Tab] Switch | [Space] Connect | [d] Delete | [q] Quit │
└─────────────────────────────────────────────┘
```

## Installation

Using the Makefile:

```bash
make install
```

This will build and install the binary to `~/.cargo/bin/mc` using `cargo install`.

Or directly with Cargo:

```bash
cargo install --path .
```

Make sure `~/.cargo/bin` is in your PATH.

## Architecture

- **TUI**: Built with [ratatui](https://github.com/ratatui-org/ratatui)
- **MIDI**: Uses [midir](https://github.com/Boddlnagg/midir) for cross-platform MIDI support
- **Multi-threading**: Each connection runs in its own thread for optimal performance

## Implementation Details

### MIDI Message Validation

Messages are validated based on MIDI message type:
- Note On/Off, Control Change, etc.: 3 bytes
- Program Change, Channel Pressure: 2 bytes
- System messages: Variable length

### Program Change Handling

Program Change messages are automatically truncated to 2 bytes if they arrive as 3 bytes (some devices send them this way).

### Connection Routing

Multiple simultaneous connections are supported. Each connection:
- Runs in a separate thread
- Validates messages before forwarding
- Reports errors without crashing

### Virtual MIDI Ports

On startup, the application creates two virtual MIDI ports:
- **mc-virtual-in**: A virtual input port that other applications can send MIDI to
- **mc-virtual-out**: A virtual output port that other applications can receive MIDI from

**Automatic Forwarding**: By default, any MIDI messages sent to `mc-virtual-in` are automatically forwarded to `mc-virtual-out`. This creates a virtual MIDI cable that other applications can use.

These ports appear in your system's MIDI device list and can be:
- Used as a MIDI cable (send to `mc-virtual-in`, receive from `mc-virtual-out`)
- Used by other MIDI applications (e.g., DAWs can send to `mc-virtual-in` or receive from `mc-virtual-out`)
- Combined with physical MIDI devices in the routing matrix (e.g., route `Hardware Keyboard → mc-virtual-out`)

The virtual ports exist as long as the application is running.

## Known Issues / TODO

- Port hotplugging not yet supported
- Connections are not persisted between sessions

## Troubleshooting

### "Device not configured" error

This error can have different causes:

**If the error occurs immediately when starting the application:**
- The application requires a real terminal (TTY)
- Make sure you're running it in an interactive terminal session, not through a script or non-TTY environment
- Test: Run `tty` in your terminal - if it says "not a tty", you need a proper terminal

**If the error occurs after the TUI appears:**
- MIDI system is not available
- MIDI drivers need to be installed
- On macOS: No MIDI devices are available

### No MIDI Devices Found

If the application starts but shows "No MIDI devices found":

To enable IAC Driver on macOS (for testing without physical MIDI devices):
1. Open "Audio MIDI Setup" (in Applications/Utilities)
2. Window → Show MIDI Studio
3. Double-click "IAC Driver"
4. Check "Device is online"

The application will work without any MIDI devices - it will just show empty lists until you connect something.

## Development

### Building

```bash
make build
# or
cargo build --release
```

### Running

```bash
make run
# or
cargo run --release
```

### Running Tests

```bash
make test
# or
cargo test
```

### Cleaning

```bash
make clean
# or
cargo clean
```

### Project Structure

```
src/
├── main.rs              # Entry point, TUI loop
├── app.rs               # Application state and logic
├── ui.rs                # TUI rendering
├── connection.rs        # Connection data structures
├── events.rs            # Event system
└── midi/
    ├── mod.rs           # MIDI module
    ├── manager.rs       # MIDI manager
    ├── forwarder.rs     # Message forwarding
    ├── validation.rs    # Message validation
    └── virtual_ports.rs # Virtual port management
```

## Migration from Go

This is a complete rewrite of the original Go CLI tool with significant enhancements:

### Improvements
- Interactive TUI instead of CLI arguments
- Multiple simultaneous connections
- Real-time status feedback
- Better error handling and logging

### Preserved Behavior
- MIDI message validation logic
- Program Change message handling
- Port matching by name

## License

Same as original project

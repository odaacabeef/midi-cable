# MIDI Cable

A Rust TUI for MIDI message routing.

## Usage

**Installation:** There's no binary distribution so you must compile it. Use
`make build` or `make install`.

```bash
mc                 # Launch TUI
mc --list-ports    # List available MIDI ports
```

## Interface

![screenshot](docs/screenshot.png)

## Virtual Ports

MIDI Cable creates two pairs of virtual MIDI ports:

- **mc-dest-a / mc-dest-b**: External apps send MIDI **to** these (destinations)
- **mc-source-a / mc-source-b**: External apps receive MIDI **from** these (sources)

Example: Route hardware synth through MIDI Cable to your DAW:
```
Hardware Synth → mc-dest-a → mc-source-b → DAW
```

## How It Works

Most MIDI tools can't see devices plugged in after they start due to CoreMIDI's
process-level caching. MIDI Cable spawns fresh subprocesses that bypass this
limitation, enabling reliable hot-plug support.

See [docs/architecture.md](docs/architecture.md) for technical details.

## Platform Support

macOS only (CoreMIDI). Linux/Windows support possible with ALSA/JACK or Windows
MIDI APIs.

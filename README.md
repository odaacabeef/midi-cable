# MIDI Cable

CLI for managing the flow of MIDI messages among devices.

## Usage

### List Available MIDI Devices

To see all available MIDI input and output devices:

```bash
mc list
```

### Forward MIDI Messages

To forward MIDI messages from one device to another:

```bash
mc fwd "Input Device Name" "Output Device Name"
```

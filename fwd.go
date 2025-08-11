package main

import (
	"context"
	"fmt"
	"log"

	"gitlab.com/gomidi/midi/v2/drivers"
	_ "gitlab.com/gomidi/midi/v2/drivers/rtmididrv"
)

type Forwarder struct {
	input  drivers.In
	output drivers.Out
}

func NewForwarder(inputName, outputName string) (*Forwarder, error) {
	// Get available ports
	ins, err := drivers.Ins()
	if err != nil {
		return nil, fmt.Errorf("failed to get MIDI inputs: %w", err)
	}

	outs, err := drivers.Outs()
	if err != nil {
		return nil, fmt.Errorf("failed to get MIDI outputs: %w", err)
	}

	// Find input port
	var input drivers.In
	for _, in := range ins {
		if in.String() == inputName {
			input = in
			break
		}
	}
	if input == nil {
		return nil, fmt.Errorf("input port '%s' not found", inputName)
	}

	// Find output port
	var output drivers.Out
	for _, out := range outs {
		if out.String() == outputName {
			output = out
			break
		}
	}
	if output == nil {
		return nil, fmt.Errorf("output port '%s' not found", outputName)
	}

	return &Forwarder{
		input:  input,
		output: output,
	}, nil
}

func (mf *Forwarder) Start(ctx context.Context) error {
	// Open input port
	if err := mf.input.Open(); err != nil {
		return fmt.Errorf("failed to open input port: %w", err)
	}
	defer mf.input.Close()

	// Open output port
	if err := mf.output.Open(); err != nil {
		return fmt.Errorf("failed to open output port: %w", err)
	}
	defer mf.output.Close()

	log.Printf("Starting MIDI forwarding from '%s' to '%s'", mf.input.String(), mf.output.String())
	log.Println("Press Ctrl+C to stop")

	// Set up message handler using Listen
	stopFn, err := mf.input.Listen(func(msg []byte, timestampms int32) {
		// Validate and forward the message
		if len(msg) > 0 {
			// Log the message for debugging
			log.Printf("Received MIDI message: %v (length: %d)", msg, len(msg))

			// Handle program change messages that might be 3 bytes
			if len(msg) >= 2 && (msg[0]&0xF0) == 0xC0 {
				// Program Change message - take only first 2 bytes
				programMsg := msg[:2]
				log.Printf("Program Change detected, using first 2 bytes: %v", programMsg)
				if err := mf.output.Send(programMsg); err != nil {
					log.Printf("Error forwarding program change: %v", err)
				}
			} else if isValidMIDIMessage(msg) {
				// Forward other valid messages as-is
				if err := mf.output.Send(msg); err != nil {
					log.Printf("Error forwarding message: %v", err)
				}
			} else {
				log.Printf("Invalid MIDI message length, skipping: %v", msg)
			}
		}
	}, drivers.ListenConfig{})
	if err != nil {
		return fmt.Errorf("failed to start listening: %w", err)
	}
	defer stopFn()

	// Wait for context cancellation
	<-ctx.Done()
	log.Println("Stopping MIDI forwarding...")
	return nil
}

// isValidMIDIMessage validates the length of a MIDI message based on its type
func isValidMIDIMessage(msg []byte) bool {
	if len(msg) == 0 {
		return false
	}

	status := msg[0] & 0xF0 // Get the message type (high nibble)

	switch status {
	case 0x80, 0x90, 0xA0, 0xB0, 0xE0: // Note Off, Note On, Poly Pressure, Control Change, Pitch Bend
		return len(msg) == 3
	case 0xC0, 0xD0: // Program Change, Channel Pressure
		return len(msg) == 2
	case 0xF0: // System messages
		if len(msg) < 2 {
			return false
		}
		switch msg[1] {
		case 0xF0: // SysEx start
			return true // Variable length, but we'll accept it
		case 0xF1, 0xF3: // MIDI Time Code, Song Select
			return len(msg) == 2
		case 0xF2: // Song Position Pointer
			return len(msg) == 3
		case 0xF6, 0xF7, 0xF8, 0xFA, 0xFB, 0xFC: // Tune Request, Clock, Start, Continue, Stop, Active Sensing, Reset
			return len(msg) == 1
		default:
			return false
		}
	default:
		return false
	}
}

package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"

	"gitlab.com/gomidi/midi/v2/drivers"
	_ "gitlab.com/gomidi/midi/v2/drivers/rtmididrv"
)

type MIDIForwarder struct {
	input  drivers.In
	output drivers.Out
}

func NewMIDIForwarder(inputName, outputName string) (*MIDIForwarder, error) {
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

	return &MIDIForwarder{
		input:  input,
		output: output,
	}, nil
}

func (mf *MIDIForwarder) Start(ctx context.Context) error {
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
		// Forward the message to output
		if err := mf.output.Send(msg); err != nil {
			log.Printf("Error forwarding message: %v", err)
		} else {
			log.Printf("Forwarded: %v", msg)
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

func listPorts() {
	fmt.Println("Available MIDI Input Ports:")
	ins, err := drivers.Ins()
	if err != nil {
		log.Printf("Error getting inputs: %v", err)
		return
	}
	for i, in := range ins {
		fmt.Printf("  %d: %s\n", i, in.String())
	}

	fmt.Println("\nAvailable MIDI Output Ports:")
	outs, err := drivers.Outs()
	if err != nil {
		log.Printf("Error getting outputs: %v", err)
		return
	}
	for i, out := range outs {
		fmt.Printf("  %d: %s\n", i, out.String())
	}
}

func main() {
	// Check command line arguments
	if len(os.Args) < 2 {
		fmt.Println("Usage: midi-cable <input-port-name> <output-port-name>")
		fmt.Println("   or: midi-cable --list")
		fmt.Println()
		fmt.Println("Examples:")
		fmt.Println("  midi-cable \"MIDI Device 1\" \"MIDI Device 2\"")
		fmt.Println("  midi-cable --list")
		os.Exit(1)
	}

	// Handle --list flag
	if os.Args[1] == "--list" {
		listPorts()
		return
	}

	// Check if we have both input and output names
	if len(os.Args) < 3 {
		fmt.Println("Error: Both input and output port names are required")
		fmt.Println("Usage: midi-cable <input-port-name> <output-port-name>")
		os.Exit(1)
	}

	inputName := os.Args[1]
	outputName := os.Args[2]

	// Create MIDI forwarder
	forwarder, err := NewMIDIForwarder(inputName, outputName)
	if err != nil {
		log.Fatalf("Failed to create MIDI forwarder: %v", err)
	}

	// Set up context with cancellation
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigChan
		cancel()
	}()

	// Start forwarding
	if err := forwarder.Start(ctx); err != nil {
		log.Fatalf("Error during MIDI forwarding: %v", err)
	}
}

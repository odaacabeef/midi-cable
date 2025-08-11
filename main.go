package main

import (
	"context"
	"fmt"
	"log"
	"os"
	"os/signal"
	"syscall"
)

func main() {
	// Check command line arguments
	if len(os.Args) < 2 {
		fmt.Println("Usage: mc <input-port-name> <output-port-name>")
		fmt.Println("   or: mc --list")
		fmt.Println()
		fmt.Println("Examples:")
		fmt.Println("  mc \"MIDI Device 1\" \"MIDI Device 2\"")
		fmt.Println("  mc --list")
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
		fmt.Println("Usage: mc <input-port-name> <output-port-name>")
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

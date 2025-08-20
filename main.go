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
		fmt.Println("Usage: mc <command> [arguments]")
		fmt.Println()
		fmt.Println("Commands:")
		fmt.Println("  list                            List available MIDI ports")
		fmt.Println("  fwd <input-name> <output-name>  Forward MIDI from input to output")
		fmt.Println("  port [name]                     Open virtual port (defaults to 'mc-port')")
		fmt.Println()
		fmt.Println("Examples:")
		fmt.Println("  mc list")
		fmt.Println("  mc fwd \"MIDI Device 1\" \"MIDI Device 2\"")
		fmt.Println("  mc port")
		os.Exit(1)
	}

	command := os.Args[1]

	switch command {
	case "list":
		listPorts()
	case "fwd":
		handleForwardCommand()
	case "port":
		handlePortCommand()
	default:
		fmt.Printf("Unknown command: %s\n", command)
		fmt.Println("Usage: mc <command> [arguments]")
		fmt.Println("Commands: fwd, list")
		os.Exit(1)
	}
}

func handleForwardCommand() {
	// Check if we have both input and output names
	if len(os.Args) < 4 {
		fmt.Println("Error: Both input and output port names are required")
		fmt.Println("Usage: mc fwd <input-port-name> <output-port-name>")
		os.Exit(1)
	}

	inputName := os.Args[2]
	outputName := os.Args[3]

	// Create MIDI forwarder
	forwarder, err := NewForwarder(inputName, outputName)
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

func handlePortCommand() {
	// Set default port name if none provided
	portName := "mc-port"

	// Check if a custom port name was provided
	if len(os.Args) >= 3 {
		portName = os.Args[2]
	}

	// Create virtual MIDI port
	virtualPort, err := NewVirtualPort(portName)
	if err != nil {
		log.Fatalf("Failed to create virtual MIDI port: %v", err)
	}

	defer virtualPort.cancel()

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigChan
		virtualPort.cancel()
	}()

	// Start virtual port
	if err := virtualPort.Start(); err != nil {
		log.Fatalf("Error during virtual port operation: %v", err)
	}
}

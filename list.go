package main

import (
	"fmt"
	"log"

	"gitlab.com/gomidi/midi/v2/drivers"
	_ "gitlab.com/gomidi/midi/v2/drivers/rtmididrv"
)

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

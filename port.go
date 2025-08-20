package main

import (
	"context"
	"fmt"
	"log"

	"gitlab.com/gomidi/midi/v2/drivers"
	"gitlab.com/gomidi/midi/v2/drivers/rtmididrv"
)

type VirtualPort struct {
	name    string
	inPort  drivers.In
	outPort drivers.Out
	ctx     context.Context
	cancel  context.CancelFunc
}

func NewVirtualPort(name string) (*VirtualPort, error) {
	ctx, cancel := context.WithCancel(context.Background())

	driver, ok := drivers.Get().(*rtmididrv.Driver)
	if !ok {
		cancel() // Cancel context on error
		return nil, fmt.Errorf("rtmididrv driver not available")
	}

	// Create virtual input port (for receiving MIDI from other applications)
	inPort, err := driver.OpenVirtualIn(name)
	if err != nil {
		cancel() // Cancel context on error
		return nil, fmt.Errorf("failed to create virtual MIDI input port '%s': %w", name, err)
	}

	// Create virtual output port (for sending MIDI to other applications)
	outPort, err := driver.OpenVirtualOut(name)
	if err != nil {
		cancel()       // Cancel context on error
		inPort.Close() // Clean up input port
		return nil, fmt.Errorf("failed to create virtual MIDI output port '%s': %w", name, err)
	}

	return &VirtualPort{
		name:    name,
		inPort:  inPort,
		outPort: outPort,
		ctx:     ctx,
		cancel:  cancel,
	}, nil
}

func (vp *VirtualPort) Start() error {
	stopFn, err := vp.inPort.Listen(func(msg []byte, timestampms int32) {
		if err := vp.outPort.Send(msg); err != nil {
			log.Printf("Error sending to %q output: %v", msg, err)
		}
	}, drivers.ListenConfig{
		TimeCode: true,
	})
	if err != nil {
		return fmt.Errorf("failed to start listening on input port: %w", err)
	}
	defer stopFn()

	log.Printf("Virtual MIDI port '%s' is now available", vp.name)

	// Wait for context cancellation
	<-vp.ctx.Done()

	log.Printf("Closing virtual MIDI port '%s'...", vp.name)
	return nil
}

# MIDI Cable

TUI application for managing MIDI message routing between devices.

## Interface

<img src="docs/screenshot.png" align="left" width="350" alt="The Synth UI">

<br>

This is what you see if you have no other MIDI devices connected. If you do they
will also be listed and routable.

`mc-in-a`/`mc-out-a` & `mc-in-b`/`mc-out-b` are two pairs of virtual ports this
application creates. By default each in passes messages through to its
corresponding out.

Each in can be forwarded to many outs.

<br clear="left"/>

## Usage

"?" toggles help

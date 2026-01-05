mod app;
mod connection;
mod events;
mod midi;
mod ui;

use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for CLI mode
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "--list-ports" => return list_ports_and_exit(),
            "worker" => {
                if args.len() < 4 {
                    eprintln!("Usage: {} worker <input-port> <output-port>", args[0]);
                    return Err("Missing arguments for worker mode".into());
                }
                return run_worker(&args[2], &args[3]);
            }
            "pipe-worker" => {
                if args.len() < 3 {
                    eprintln!("Usage: {} pipe-worker <output-port>", args[0]);
                    return Err("Missing arguments for pipe-worker mode".into());
                }
                return run_pipe_worker(&args[2]);
            }
            _ => {}
        }
    }

    // Create app
    let mut app = App::new();

    // Initialize MIDI before setting up terminal
    // This ensures virtual ports are ready before entering TUI mode
    if let Err(e) = app.initialize() {
        eprintln!("Failed to initialize MIDI: {}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))));
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Application error: {}", e);
        return Err(Box::new(e));
    }

    Ok(())
}

/// Pipe worker mode: read MIDI messages from stdin and forward to output port
/// Used for virtual input connections - stdin receives data from virtual input callback
fn run_pipe_worker(output_port_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    use midir::MidiOutput;
    use std::io::{self, Read};

    eprintln!("Pipe worker starting for output: {}", output_port_name);

    // Create MIDI output
    let midi_out = MidiOutput::new("mc-pipe-worker")?;

    // Find output port
    let out_ports = midi_out.ports();
    let out_port = out_ports
        .iter()
        .find(|p| midi_out.port_name(p).ok().as_deref() == Some(output_port_name))
        .ok_or_else(|| format!("Output port '{}' not found", output_port_name))?;

    // Connect to output
    let mut out_conn = midi_out.connect(out_port, "mc-pipe-worker-out")?;

    eprintln!("Pipe worker connected to: {}", output_port_name);

    // Read MIDI messages from stdin and forward to output
    let stdin = io::stdin();
    let mut stdin_lock = stdin.lock();
    let mut buffer = [0u8; 1024];

    loop {
        match stdin_lock.read(&mut buffer) {
            Ok(0) => {
                // EOF - parent closed pipe
                eprintln!("Pipe worker: stdin closed, exiting");
                break;
            }
            Ok(n) => {
                // Forward MIDI message
                if let Err(e) = out_conn.send(&buffer[..n]) {
                    eprintln!("Pipe worker error forwarding: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Pipe worker error reading stdin: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Worker mode: create a MIDI connection and forward messages until killed
/// This runs in a subprocess with fresh MIDI context that sees current system state
fn run_worker(input_port_name: &str, output_port_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    use midir::{MidiInput, MidiOutput};
    use std::sync::{Arc, Mutex};
    use midi::validation::{is_program_change, is_valid_midi_message, normalize_program_change};

    // Create MIDI input and output (worker runs after ports verified to exist)
    let midi_in = MidiInput::new("mc-worker")?;
    let midi_out = MidiOutput::new("mc-worker")?;

    // DEBUG: Log what ports the worker actually sees
    eprintln!("Worker input ports:");
    for port in midi_in.ports() {
        if let Ok(name) = midi_in.port_name(&port) {
            eprintln!("  - {}", name);
        }
    }
    eprintln!("Worker output ports:");
    for port in midi_out.ports() {
        if let Ok(name) = midi_out.port_name(&port) {
            eprintln!("  - {}", name);
        }
    }

    // Find input port
    let in_ports = midi_in.ports();
    let in_port = in_ports
        .iter()
        .find(|p| midi_in.port_name(p).ok().as_deref() == Some(input_port_name))
        .ok_or_else(|| format!("Input port '{}' not found", input_port_name))?;

    // Find output port
    let out_ports = midi_out.ports();
    let out_port = out_ports
        .iter()
        .find(|p| midi_out.port_name(p).ok().as_deref() == Some(output_port_name))
        .ok_or_else(|| format!("Output port '{}' not found", output_port_name))?;

    // Connect to output
    let out_conn = midi_out.connect(out_port, "mc-worker-out")?;
    let out_conn_shared = Arc::new(Mutex::new(out_conn));
    let out_conn_clone = Arc::clone(&out_conn_shared);

    // Connect to input with forwarding callback
    let _in_conn = midi_in.connect(
        in_port,
        "mc-worker-in",
        move |_timestamp, message, _| {
            if message.is_empty() {
                return;
            }

            // Handle Program Change messages
            if is_program_change(message) {
                let normalized = normalize_program_change(message);
                if let Ok(mut out) = out_conn_clone.lock() {
                    if let Err(e) = out.send(&normalized) {
                        eprintln!("Error forwarding program change: {}", e);
                    }
                }
                return;
            }

            // Validate and forward other messages
            if is_valid_midi_message(message) {
                if let Ok(mut out) = out_conn_clone.lock() {
                    if let Err(e) = out.send(message) {
                        eprintln!("Error forwarding message: {}", e);
                    }
                }
            }
        },
        (),
    )?;

    eprintln!("Worker started: {} -> {}", input_port_name, output_port_name);

    // Keep the worker alive until killed
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

/// CLI mode: list all MIDI ports and exit
/// This creates a fresh MIDI context that sees current system state
fn list_ports_and_exit() -> Result<(), Box<dyn std::error::Error>> {
    use midi::MidiManager;

    let inputs = MidiManager::list_input_ports();
    let outputs = MidiManager::list_output_ports();

    // Print in JSON format for easy parsing
    println!("{{");
    println!("  \"inputs\": [");
    for (i, port) in inputs.iter().enumerate() {
        let comma = if i < inputs.len() - 1 { "," } else { "" };
        println!("    \"{}\"{}",port.name, comma);
    }
    println!("  ],");
    println!("  \"outputs\": [");
    for (i, port) in outputs.iter().enumerate() {
        let comma = if i < outputs.len() - 1 { "," } else { "" };
        println!("    \"{}\"{}",port.name, comma);
    }
    println!("  ]");
    println!("}}");

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        // Process any pending MIDI events
        app.process_events();

        // Draw UI
        terminal.draw(|f| ui::render(f, app))?;

        // Handle keyboard input with timeout
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.quit();
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.quit();
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        app.handle_key_up();
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        app.handle_key_down();
                    }
                    KeyCode::Char(' ') => {
                        app.handle_space();
                    }
                    KeyCode::Enter => {
                        app.handle_enter();
                    }
                    KeyCode::Esc => {
                        app.handle_escape();
                    }
                    KeyCode::Char('?') => {
                        app.toggle_help();
                    }
                    _ => {}
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

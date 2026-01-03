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
    if args.len() > 1 && args[1] == "--list-ports" {
        return list_ports_and_exit();
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new();

    // Initialize MIDI
    if let Err(e) = app.initialize() {
        // Clean up terminal before showing error
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        eprintln!("Failed to initialize MIDI: {}", e);
        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))));
    }

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
                    KeyCode::Char('R') => {
                        app.handle_refresh();
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

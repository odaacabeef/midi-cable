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
                    KeyCode::Up => {
                        app.handle_key_up();
                    }
                    KeyCode::Down => {
                        app.handle_key_down();
                    }
                    KeyCode::Tab => {
                        app.handle_tab();
                    }
                    KeyCode::Char(' ') => {
                        app.handle_space();
                    }
                    KeyCode::Char('d') => {
                        app.handle_delete();
                    }
                    KeyCode::Esc => {
                        app.handle_escape();
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

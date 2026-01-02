use crate::app::{App, UiState};
use crate::connection::{ConnectionStatus, PortId};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),      // Title
            Constraint::Min(10),        // Main content (inputs/outputs)
            Constraint::Length(8),      // Connections
            Constraint::Length(6),      // Log
            Constraint::Length(1),      // Help
        ])
        .split(f.area());

    render_title(f, chunks[0]);
    render_ports(f, chunks[1], app);
    render_connections(f, chunks[2], app);
    render_log(f, chunks[3], app);
    render_help(f, chunks[4], app);
}

fn render_title(f: &mut Frame, area: Rect) {
    let title = Paragraph::new("MIDI Cable - Routing Matrix")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, area);
}

fn render_ports(f: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Determine which pane is active
    let (input_active, output_active) = match &app.ui_state {
        UiState::Idle { focus, .. } => match focus {
            crate::app::PaneFocus::Inputs => (true, false),
            crate::app::PaneFocus::Outputs => (false, true),
            _ => (false, false),
        },
        UiState::SelectingSource { .. } => (true, false),
        UiState::SelectingDestination { .. } => (false, true),
    };

    render_port_list(
        f,
        chunks[0],
        "INPUTS",
        &app.midi_inputs,
        app.get_selected_input_idx(),
        input_active,
    );

    render_port_list(
        f,
        chunks[1],
        "OUTPUTS",
        &app.midi_outputs,
        app.get_selected_output_idx(),
        output_active,
    );
}

fn render_port_list(
    f: &mut Frame,
    area: Rect,
    title: &str,
    ports: &[PortId],
    selected_idx: Option<usize>,
    is_active: bool,
) {
    let items: Vec<ListItem> = ports
        .iter()
        .enumerate()
        .map(|(idx, port)| {
            let is_selected = selected_idx == Some(idx);
            let prefix = if is_selected { "[>] " } else { "[ ] " };

            let style = if port.is_virtual {
                Style::default().fg(Color::LightGreen)
            } else {
                Style::default()
            };

            let style = if is_selected {
                style.add_modifier(Modifier::BOLD)
            } else {
                style
            };

            ListItem::new(format!("{}{}", prefix, port.name)).style(style)
        })
        .collect();

    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn render_connections(f: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .active_connections
        .iter()
        .enumerate()
        .map(|(idx, (conn, status))| {
            let is_selected = matches!(&app.ui_state, UiState::Idle { focus: crate::app::PaneFocus::Connections, selected_connection_idx, .. } if *selected_connection_idx == idx);

            let status_str = match status {
                ConnectionStatus::Active => "[OK]",
                ConnectionStatus::Error(_) => "[ERR]",
            };

            let status_color = match status {
                ConnectionStatus::Active => Color::Green,
                ConnectionStatus::Error(_) => Color::Red,
            };

            let prefix = if is_selected { "> " } else { "  " };

            let line = Line::from(vec![
                Span::raw(prefix),
                Span::raw("• "),
                Span::raw(format!("{} ", conn)),
                Span::styled(status_str, Style::default().fg(status_color)),
            ]);

            let style = if is_selected {
                Style::default().add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let is_active = matches!(&app.ui_state, UiState::Idle { focus: crate::app::PaneFocus::Connections, .. });

    let border_style = if is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("ACTIVE CONNECTIONS")
            .border_style(border_style),
    );

    f.render_widget(list, area);
}

fn render_log(f: &mut Frame, area: Rect, app: &App) {
    let log_lines: Vec<Line> = app
        .log_messages
        .iter()
        .map(|msg| Line::from(msg.clone()))
        .collect();

    let paragraph = Paragraph::new(log_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title("LOG"),
    );

    f.render_widget(paragraph, area);
}

fn render_help(f: &mut Frame, area: Rect, app: &App) {
    let help_text = match &app.ui_state {
        UiState::Idle { .. } => {
            "[↑↓] Navigate | [Tab] Switch Pane | [Space] Select/Connect | [d] Delete | [q] Quit"
        }
        UiState::SelectingSource { .. } => {
            "[↑↓] Navigate | [Space] Select Source | [Esc] Cancel"
        }
        UiState::SelectingDestination { .. } => {
            "[↑↓] Navigate | [Space] Connect | [Esc] Cancel"
        }
    };

    let paragraph = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray));

    f.render_widget(paragraph, area);
}

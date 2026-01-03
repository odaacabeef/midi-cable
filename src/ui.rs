use crate::app::{App, UiState};
use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    // Show help overlay if requested
    if app.show_help {
        render_help(f);
        return;
    }

    let area = f.area();

    let mut lines = Vec::new();

    // Render inputs with inline connections
    match &app.ui_state {
        UiState::Idle { cursor_idx } => {
            // Idle mode: show inputs with cursor
            for (idx, input) in app.midi_inputs.iter().enumerate() {
                let is_cursor = idx == *cursor_idx;
                let connected_outputs = app.get_connected_outputs(input);

                let line = format_input_line(input, &connected_outputs, is_cursor, false);
                lines.push(line);
            }
        }
        UiState::SelectingOutputs { input_idx, .. } => {
            // Selecting outputs: mark input as selected
            for (idx, input) in app.midi_inputs.iter().enumerate() {
                let is_selected = idx == *input_idx;
                let connected_outputs = app.get_connected_outputs(input);

                let line = format_input_line(input, &connected_outputs, false, is_selected);
                lines.push(line);
            }
        }
    }

    // Add blank line separator
    lines.push(Line::from(""));

    // Render outputs
    match &app.ui_state {
        UiState::Idle { .. } => {
            // Idle mode: just list outputs
            for output in &app.midi_outputs {
                lines.push(Line::from(vec![
                    Span::raw("  [ ] "),
                    Span::raw(output.name.clone()),
                ]));
            }
        }
        UiState::SelectingOutputs {
            selected_outputs,
            cursor_idx,
            ..
        } => {
            // Selecting outputs: show cursor and selection
            for (idx, output) in app.midi_outputs.iter().enumerate() {
                let is_cursor = idx == *cursor_idx;
                let is_selected = selected_outputs.contains(&idx);

                let cursor_mark = if is_cursor { "> " } else { "  " };
                let checkbox = if is_selected { "[x]" } else { "[ ]" };

                lines.push(Line::from(vec![
                    Span::raw(cursor_mark),
                    Span::raw(checkbox),
                    Span::raw(" "),
                    Span::raw(output.name.clone()),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    f.render_widget(paragraph, area);
}

/// Format an input line with inline connections
fn format_input_line(
    input: &crate::connection::PortId,
    connected_outputs: &[crate::connection::PortId],
    is_cursor: bool,
    is_selected: bool,
) -> Line<'static> {
    let cursor_mark = if is_cursor { "> " } else { "  " };
    let checkbox = if is_selected { "[x]" } else { "[ ]" };

    let mut spans = vec![
        Span::raw(cursor_mark),
        Span::raw(checkbox),
        Span::raw(" "),
        Span::raw(input.name.clone()),
    ];

    // Add connection display if there are connections
    if !connected_outputs.is_empty() {
        spans.push(Span::raw(" -> "));

        let output_names: Vec<String> = connected_outputs
            .iter()
            .map(|o| o.name.clone())
            .collect();

        spans.push(Span::styled(
            output_names.join(", "),
            Style::default().fg(Color::Green),
        ));
    }

    Line::from(spans)
}

/// Render the help screen
fn render_help(f: &mut Frame) {
    let area = f.area();

    let help_text = vec![
        Line::from("Navigation:"),
        Line::from("  ↑/↓ or j/k       Move cursor up/down"),
        Line::from(""),
        Line::from("Managing Connections:"),
        Line::from("  Space            Select input, then toggle output selection(s)"),
        Line::from("  Enter            Commit selected connections"),
        Line::from("  Esc              Cancel output selection"),
        Line::from(""),
        Line::from("Other Commands:"),
        Line::from("  ?                Toggle this help screen"),
        Line::from("  q or ctrl+c      Quit application"),
        Line::from(""),
        Line::from("Virtual Ports:"),
        Line::from("  mc-virtual-in    Virtual input (other apps send to this)"),
        Line::from("  mc-virtual-out   Virtual output (other apps receive from this)"),
    ];

    let paragraph = Paragraph::new(help_text);
    f.render_widget(paragraph, area);
}

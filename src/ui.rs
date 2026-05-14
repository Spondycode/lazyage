use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use crate::app::{App, Pane};
use std::fs;

pub fn render(f: &mut Frame, app: &mut App) {
    // Top section (Files, Keys, Preview)
    // Bottom section (Logs, Actions)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(10),
        ])
        .split(f.area());

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(70),
        ])
        .split(chunks[0]);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(top_chunks[0]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(chunks[1]);

    // 1. File List
    let files: Vec<ListItem> = app.files
        .iter()
        .map(|path| {
            ListItem::new(path.file_name().unwrap().to_string_lossy().to_string())
        })
        .collect();

    let files_list = List::new(files)
        .block(Block::default().borders(Borders::ALL).title(format!("Files ({}/{})", if app.files.is_empty() { 0 } else { app.selected_file + 1 }, app.files.len())))
        .highlight_symbol(">> ")
        .highlight_style(if matches!(app.active_pane, Pane::Files) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        });
    f.render_stateful_widget(files_list, left_chunks[0], &mut app.file_list_state);

    // 2. Key List
    let keys: Vec<ListItem> = app.keys
        .iter()
        .map(|key| {
            let selection_mark = if key.selected { "[x] " } else { "[ ] " };
            let type_str = if key.is_passphrase_only { "[P]" } else if key.is_secret { "[S]" } else { "[K]" };
            ListItem::new(format!("{} {} {}", selection_mark, type_str, key.name))
        })
        .collect();

    let keys_list = List::new(keys)
        .block(Block::default().borders(Borders::ALL).title(format!("Keys ({}/{})", if app.keys.is_empty() { 0 } else { app.selected_key + 1 }, app.keys.len())))
        .highlight_symbol(">> ")
        .highlight_style(if matches!(app.active_pane, Pane::Keys) {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::BOLD)
        });
    f.render_stateful_widget(keys_list, left_chunks[1], &mut app.key_list_state);

    // 3. Preview Pane (Now in the main top-right area)
    let preview_block = Block::default().borders(Borders::ALL).title("File Preview");
    if let Some(path) = app.files.get(app.selected_file) {
        let content = if path.extension().and_then(|s| s.to_str()) == Some("age") {
            "--- Encrypted age file ---".to_string()
        } else {
            match fs::read_to_string(path) {
                Ok(c) => {
                    // Show first 1000 chars to avoid performance issues
                    if c.len() > 2000 {
                        format!("{}...", &c[..2000])
                    } else {
                        c
                    }
                }
                Err(_) => "--- Binary or unreadable file ---".to_string(),
            }
        };
        let preview_para = Paragraph::new(content)
            .block(preview_block)
            .wrap(Wrap { trim: true });
        f.render_widget(preview_para, top_chunks[1]);
    } else {
        f.render_widget(Paragraph::new("No file selected").block(preview_block), top_chunks[1]);
    }

    // 4. Log Pane (Narrower now)
    let logs: Vec<ListItem> = app.logs
        .iter()
        .rev()
        .take(10)
        .map(|log| ListItem::new(log.as_str()))
        .collect();
    let logs_list = List::new(logs).block(Block::default().borders(Borders::ALL).title("Logs"));
    f.render_widget(logs_list, bottom_chunks[0]);

    // 5. Actions Pane (Moved to bottom right)
    let action_block = Block::default().borders(Borders::ALL).title("Actions");
    let current_key = app.keys.get(app.selected_key);

    let mut action_text = vec![
        Line::from(vec![Span::styled("Tab", Style::default().fg(Color::Cyan)), Span::raw(": Switch Panes"), Span::raw(" | "), Span::styled("R", Style::default().fg(Color::Cyan)), Span::raw(": Refresh")]),
        Line::from(vec![Span::styled("e", Style::default().fg(Color::Green)), Span::raw(": Encrypt"), Span::raw(" | "), Span::styled("d", Style::default().fg(Color::Red)), Span::raw(": Decrypt")]),
        Line::from(vec![Span::styled("p", Style::default().fg(Color::Green)), Span::raw(": Encrypt (Passphrase)"), Span::raw(" | "), Span::styled("x", Style::default().fg(Color::Red)), Span::raw(": Delete")]),
        Line::from(vec![Span::styled("g", Style::default().fg(Color::Green)), Span::raw(": Generate Key"), Span::raw(" | "), Span::styled("Space", Style::default().fg(Color::Cyan)), Span::raw(": Toggle Key")]),
    ];

    if let Some(path) = app.files.get(app.selected_file) {
        action_text.push(Line::from(""));
        action_text.push(Line::from(vec![
            Span::styled("File: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(path.file_name().unwrap().to_string_lossy().to_string()),
        ]));
    }

    if let Some(key) = current_key {
        if app.files.get(app.selected_file).is_none() {
            action_text.push(Line::from(""));
        }
        action_text.push(Line::from(vec![Span::styled("Key: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&key.name)]));
    }

    let action_para = Paragraph::new(action_text).block(action_block);
    f.render_widget(action_para, bottom_chunks[1]);

    // Passphrase Input Modal
    if let crate::app::InputMode::EnteringPassphrase(_) = app.input_mode {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(ratatui::widgets::Clear, area);
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Enter Passphrase")
            .style(Style::default().bg(Color::DarkGray));
        let input_para = Paragraph::new(app.input.as_str())
            .block(input_block);
        f.render_widget(input_para, area);
    }

    // Key Generation Modal
    if let crate::app::InputMode::GeneratingKey = app.input_mode {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(ratatui::widgets::Clear, area);
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title("Enter new key filename (e.g. my_key)")
            .style(Style::default().bg(Color::DarkGray));
        let input_para = Paragraph::new(app.input.as_str())
            .block(input_block);
        f.render_widget(input_para, area);
    }

    // Delete Confirmation Modal
    if let crate::app::InputMode::Deleting = app.input_mode {
        let area = centered_rect(60, 20, f.area());
        f.render_widget(ratatui::widgets::Clear, area);
        let file_name = app.files.get(app.selected_file)
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        
        let confirm_block = Block::default()
            .borders(Borders::ALL)
            .title("Confirm Deletion")
            .style(Style::default().bg(Color::Red).fg(Color::White));
        
        let confirm_text = vec![
            Line::from(""),
            Line::from(vec![Span::raw("Are you sure you want to delete: ")]),
            Line::from(vec![Span::styled(file_name, Style::default().add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::raw("Press "), Span::styled("Enter", Style::default().add_modifier(Modifier::BOLD)), Span::raw(" to delete, "), Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)), Span::raw(" to cancel.")]),
        ];

        let confirm_para = Paragraph::new(confirm_text)
            .block(confirm_block)
            .wrap(Wrap { trim: true });
        f.render_widget(confirm_para, area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

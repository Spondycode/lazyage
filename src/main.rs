mod app;
mod ui;
mod crypto;
mod discovery;

use std::{io, time::Duration};
use crossterm::{
    event::{self, EnableMouseCapture, Event, KeyCode, EnableFocusChange},
    execute,
    terminal::{enable_raw_mode, EnterAlternateScreen},
};
use ratatui::DefaultTerminal;
use crate::app::{App, Pane, InputMode, PassphraseAction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableFocusChange)?;
    
    let mut terminal = ratatui::init();

    // Create app and load initial data
    let mut app = App::new();
    app.set_files(discovery::get_files_in_cwd());
    app.set_keys(discovery::discover_keys());
    app.log(format!("Found {} files and {} keys.", app.files.len(), app.keys.len()));

    let res = run_app(&mut terminal, app);

    // Restore terminal
    ratatui::restore();

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app(
    terminal: &mut DefaultTerminal,
    mut app: App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, &mut app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::FocusGained => {
                    app.set_files(discovery::get_files_in_cwd());
                }
                Event::Key(key) => {
                    match app.input_mode {
                        InputMode::Normal => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('R') => {
                                    app.set_files(discovery::get_files_in_cwd());
                                    app.set_keys(discovery::discover_keys());
                                    app.log("Refreshed files and keys.".to_string());
                                }
                                KeyCode::Tab => {
                                    app.switch_pane();
                                    app.set_files(discovery::get_files_in_cwd());
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    match app.active_pane {
                                        Pane::Files => app.next_file(),
                                        Pane::Keys => app.next_key(),
                                        _ => {}
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    match app.active_pane {
                                        Pane::Files => app.previous_file(),
                                        Pane::Keys => app.previous_key(),
                                        _ => {}
                                    }
                                }
                                KeyCode::Char('p') => {
                                    app.input_mode = InputMode::EnteringPassphrase(PassphraseAction::Encrypt);
                                }
                                KeyCode::Char('g') => {
                                    app.input.clear();
                                    app.input_mode = InputMode::GeneratingKey;
                                }
                                KeyCode::Char('x') => {
                                    if !app.files.is_empty() {
                                        app.input_mode = InputMode::Deleting;
                                    }
                                }
                                KeyCode::Char(' ') => {
                                    if matches!(app.active_pane, Pane::Keys) {
                                        app.toggle_key_selection();
                                    }
                                }
                                KeyCode::Char('e') => {
                                    if let Some(file) = app.files.get(app.selected_file) {
                                        let mut selected_recipients = Vec::new();
                                        let mut use_passphrase = false;

                                        let any_selected = app.keys.iter().any(|k| k.selected);
                                        
                                        if any_selected {
                                            for key in &app.keys {
                                                if key.selected {
                                                    if key.is_passphrase_only {
                                                        use_passphrase = true;
                                                    } else {
                                                        selected_recipients.extend(key.recipients.clone());
                                                    }
                                                }
                                            }
                                        } else if let Some(key) = app.keys.get(app.selected_key) {
                                            // Fallback to highlighted key if nothing is explicitly selected
                                            if key.is_passphrase_only {
                                                use_passphrase = true;
                                            } else {
                                                selected_recipients.extend(key.recipients.clone());
                                            }
                                        }

                                        if use_passphrase {
                                            app.input_mode = InputMode::EnteringPassphrase(PassphraseAction::Encrypt);
                                        } else if !selected_recipients.is_empty() {
                                            match crypto::encrypt_file(file, selected_recipients, None) {
                                                Ok(path) => app.log(format!("Encrypted to {}", path)),
                                                Err(e) => app.log(format!("Error: {}", e)),
                                            }
                                            app.set_files(discovery::get_files_in_cwd());
                                        } else {
                                            app.log("No recipients selected.".to_string());
                                        }
                                    }
                                }
                                KeyCode::Char('d') => {
                                    if let Some(file) = app.files.get(app.selected_file) {
                                        let id_paths: Vec<_> = app.keys.iter()
                                            .filter(|k| k.is_secret)
                                            .map(|k| k.path.clone())
                                            .collect();

                                        match crypto::decrypt_file(file, id_paths, None) {
                                            Ok(path) => app.log(format!("Decrypted to {}", path)),
                                            Err(e) => {
                                                if e.to_string().contains("Passphrase required") {
                                                    app.input_mode = InputMode::EnteringPassphrase(PassphraseAction::Decrypt);
                                                } else {
                                                    app.log(format!("Error: {}", e));
                                                }
                                            }
                                        }
                                        app.set_files(discovery::get_files_in_cwd());
                                    }
                                }
                                _ => {}
                            }
                        }
                        InputMode::EnteringPassphrase(action) => {
                            match key.code {
                                KeyCode::Enter => {
                                    let input = app.input.drain(..).collect::<String>();
                                    app.input_mode = InputMode::Normal;
                                    
                                    if let Some(file) = app.files.get(app.selected_file) {
                                        match action {
                                            PassphraseAction::Encrypt => {
                                                let mut selected_recipients = Vec::new();
                                                let any_selected = app.keys.iter().any(|k| k.selected);

                                                if any_selected {
                                                    for key in &app.keys {
                                                        if key.selected && !key.is_passphrase_only {
                                                            selected_recipients.extend(key.recipients.clone());
                                                        }
                                                    }
                                                } else if let Some(key) = app.keys.get(app.selected_key) {
                                                    // Only use highlighted key if it's NOT the passphrase only one
                                                    // (since we are already using a passphrase)
                                                    if !key.is_passphrase_only {
                                                        selected_recipients.extend(key.recipients.clone());
                                                    }
                                                }

                                                match crypto::encrypt_file(file, selected_recipients, Some(input)) {
                                                    Ok(path) => app.log(format!("Encrypted to {}", path)),
                                                    Err(e) => app.log(format!("Error: {}", e)),
                                                }
                                            }
                                            PassphraseAction::Decrypt => {
                                                let id_paths: Vec<_> = app.keys.iter()
                                                    .filter(|k| k.is_secret)
                                                    .map(|k| k.path.clone())
                                                    .collect();
                                                match crypto::decrypt_file(file, id_paths, Some(&input)) {
                                                    Ok(path) => app.log(format!("Decrypted to {}", path)),
                                                    Err(e) => app.log(format!("Error: {}", e)),
                                                }
                                            }
                                        }
                                        app.set_files(discovery::get_files_in_cwd());
                                    }
                                }
                                KeyCode::Char(c) => app.input.push(c),
                                KeyCode::Backspace => { app.input.pop(); },
                                KeyCode::Esc => {
                                    app.input.clear();
                                    app.input_mode = InputMode::Normal;
                                }
                                _ => {}
                            }
                        }
                        InputMode::Deleting => {
                            match key.code {
                                KeyCode::Enter => {
                                    if let Some(file) = app.files.get(app.selected_file) {
                                        let file_path = file.clone();
                                        match std::fs::remove_file(&file_path) {
                                            Ok(_) => app.log(format!("Deleted {}", file_path.display())),
                                            Err(e) => app.log(format!("Error deleting {}: {}", file_path.display(), e)),
                                        }
                                        app.set_files(discovery::get_files_in_cwd());
                                    }
                                    app.input_mode = InputMode::Normal;
                                }
                                KeyCode::Esc => {
                                    app.input_mode = InputMode::Normal;
                                }
                                _ => {}
                            }
                        }
                        InputMode::GeneratingKey => {
                            match key.code {
                                KeyCode::Enter => {
                                    let filename = app.input.drain(..).collect::<String>();
                                    app.input_mode = InputMode::Normal;
                                    
                                    if !filename.is_empty() {
                                        match crypto::generate_new_key(&filename) {
                                            Ok(path) => {
                                                app.log(format!("Generated key: {}", path));
                                                app.set_keys(discovery::discover_keys());
                                            }
                                            Err(e) => app.log(format!("Error: {}", e)),
                                        }
                                    }
                                }
                                KeyCode::Char(c) => app.input.push(c),
                                KeyCode::Backspace => { app.input.pop(); },
                                KeyCode::Esc => {
                                    app.input.clear();
                                    app.input_mode = InputMode::Normal;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

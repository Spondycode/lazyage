mod app;
mod ui;
mod crypto;
mod discovery;

use std::{io, time::Duration};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, EnableFocusChange, DisableFocusChange},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use crate::app::{App, Pane, InputMode, PassphraseAction};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture, EnableFocusChange)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and load initial data
    let mut app = App::new();
    app.files = discovery::get_files_in_cwd();
    app.keys = discovery::discover_keys();
    app.log(format!("Found {} files and {} keys.", app.files.len(), app.keys.len()));

    let res = run_app(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture,
        DisableFocusChange
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::FocusGained => {
                    app.files = discovery::get_files_in_cwd();
                }
                Event::Key(key) => {
                    match app.input_mode {
                        InputMode::Normal => {
                            match key.code {
                                KeyCode::Char('q') => return Ok(()),
                                KeyCode::Char('R') => {
                                    app.files = discovery::get_files_in_cwd();
                                    app.keys = discovery::discover_keys();
                                    app.log("Refreshed files and keys.".to_string());
                                }
                                KeyCode::Tab => {
                                    app.switch_pane();
                                    app.files = discovery::get_files_in_cwd();
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
                                KeyCode::Char('e') => {
                                    if let (Some(file), Some(key)) = (app.files.get(app.selected_file), app.keys.get(app.selected_key)) {
                                        if key.is_passphrase_only {
                                            app.input_mode = InputMode::EnteringPassphrase(PassphraseAction::Encrypt);
                                        } else if let Some(recipient) = &key.public_key {
                                            match crypto::encrypt_with_key(file, recipient) {
                                                Ok(path) => app.log(format!("Encrypted to {}", path)),
                                                Err(e) => app.log(format!("Error: {}", e)),
                                            }
                                            app.files = discovery::get_files_in_cwd();
                                        } else {
                                            app.log("No public key for selected entry.".to_string());
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
                                        app.files = discovery::get_files_in_cwd();
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
                                                match crypto::encrypt_with_passphrase(file, &input) {
                                                    Ok(path) => app.log(format!("Encrypted with passphrase to {}", path)),
                                                    Err(e) => app.log(format!("Error: {}", e)),
                                                }
                                            }
                                            PassphraseAction::Decrypt => {
                                                match crypto::decrypt_file(file, vec![], Some(&input)) {
                                                    Ok(path) => app.log(format!("Decrypted with passphrase to {}", path)),
                                                    Err(e) => app.log(format!("Error: {}", e)),
                                                }
                                            }
                                        }
                                        app.files = discovery::get_files_in_cwd();
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
                                        app.files = discovery::get_files_in_cwd();
                                        if app.selected_file >= app.files.len() && !app.files.is_empty() {
                                            app.selected_file = app.files.len() - 1;
                                        }
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
                                                app.keys = discovery::discover_keys();
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

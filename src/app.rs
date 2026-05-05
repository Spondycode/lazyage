use std::path::PathBuf;

#[derive(Clone, Copy)]
pub enum InputMode {
    Normal,
    EnteringPassphrase(PassphraseAction),
    Deleting,
}

#[derive(Clone, Copy)]
pub enum PassphraseAction {
    Encrypt,
    Decrypt,
}

pub enum Pane {
    Files,
    Keys,
    Actions,
}

pub struct App {
    pub files: Vec<PathBuf>,
    pub selected_file: usize,
    pub keys: Vec<KeyInfo>,
    pub selected_key: usize,
    pub active_pane: Pane,
    pub input_mode: InputMode,
    pub input: String,
    pub should_quit: bool,
    pub logs: Vec<String>,
}

pub struct KeyInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_secret: bool,
    pub is_passphrase_only: bool,
    pub public_key: Option<String>,
}

impl App {
    pub fn new() -> App {
        App {
            files: Vec::new(),
            selected_file: 0,
            keys: Vec::new(),
            selected_key: 0,
            active_pane: Pane::Files,
            input_mode: InputMode::Normal,
            input: String::new(),
            should_quit: false,
            logs: vec!["Welcome to lazyage!".to_string()],
        }
    }

    pub fn log(&mut self, message: String) {
        self.logs.push(message);
    }

    pub fn next_file(&mut self) {
        if !self.files.is_empty() {
            self.selected_file = (self.selected_file + 1) % self.files.len();
        }
    }

    pub fn previous_file(&mut self) {
        if !self.files.is_empty() {
            if self.selected_file > 0 {
                self.selected_file -= 1;
            } else {
                self.selected_file = self.files.len() - 1;
            }
        }
    }

    pub fn next_key(&mut self) {
        if !self.keys.is_empty() {
            self.selected_key = (self.selected_key + 1) % self.keys.len();
        }
    }

    pub fn previous_key(&mut self) {
        if !self.keys.is_empty() {
            if self.selected_key > 0 {
                self.selected_key -= 1;
            } else {
                self.selected_key = self.keys.len() - 1;
            }
        }
    }

    pub fn switch_pane(&mut self) {
        self.active_pane = match self.active_pane {
            Pane::Files => Pane::Keys,
            Pane::Keys => Pane::Actions,
            Pane::Actions => Pane::Files,
        };
    }
}

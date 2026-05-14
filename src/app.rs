use std::path::PathBuf;
use ratatui::widgets::ListState;

#[derive(Clone, Copy)]
pub enum InputMode {
    Normal,
    EnteringPassphrase(PassphraseAction),
    Deleting,
    GeneratingKey,
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
    pub file_list_state: ListState,
    pub keys: Vec<KeyInfo>,
    pub selected_key: usize,
    pub key_list_state: ListState,
    pub active_pane: Pane,
    pub input_mode: InputMode,
    pub input: String,
    pub logs: Vec<String>,
}

pub struct KeyInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_secret: bool,
    pub is_passphrase_only: bool,
    pub public_key: Option<String>,
    pub selected: bool,
}

impl App {
    pub fn new() -> App {
        let mut file_list_state = ListState::default();
        file_list_state.select(Some(0));
        let mut key_list_state = ListState::default();
        key_list_state.select(Some(0));

        App {
            files: Vec::new(),
            selected_file: 0,
            file_list_state,
            keys: Vec::new(),
            selected_key: 0,
            key_list_state,
            active_pane: Pane::Files,
            input_mode: InputMode::Normal,
            input: String::new(),
            logs: vec!["Welcome to lazyage!".to_string()],
        }
    }

    pub fn set_files(&mut self, files: Vec<PathBuf>) {
        self.files = files;
        if self.files.is_empty() {
            self.selected_file = 0;
            self.file_list_state.select(None);
        } else {
            if self.selected_file >= self.files.len() {
                self.selected_file = self.files.len() - 1;
            }
            self.file_list_state.select(Some(self.selected_file));
        }
    }

    pub fn set_keys(&mut self, keys: Vec<KeyInfo>) {
        // Try to preserve selection if possible, otherwise all false
        let previously_selected: Vec<String> = self.keys.iter()
            .filter(|k| k.selected)
            .map(|k| k.name.clone())
            .collect();

        self.keys = keys;
        for key in &mut self.keys {
            if previously_selected.contains(&key.name) {
                key.selected = true;
            }
        }

        if self.keys.is_empty() {
            self.selected_key = 0;
            self.key_list_state.select(None);
        } else {
            if self.selected_key >= self.keys.len() {
                self.selected_key = self.keys.len() - 1;
            }
            self.key_list_state.select(Some(self.selected_key));
        }
    }

    pub fn log(&mut self, message: String) {
        self.logs.push(message);
    }

    pub fn next_file(&mut self) {
        if !self.files.is_empty() && self.selected_file < self.files.len() - 1 {
            self.selected_file += 1;
            self.file_list_state.select(Some(self.selected_file));
        }
    }

    pub fn previous_file(&mut self) {
        if !self.files.is_empty() && self.selected_file > 0 {
            self.selected_file -= 1;
            self.file_list_state.select(Some(self.selected_file));
        }
    }

    pub fn next_key(&mut self) {
        if !self.keys.is_empty() && self.selected_key < self.keys.len() - 1 {
            self.selected_key += 1;
            self.key_list_state.select(Some(self.selected_key));
        }
    }

    pub fn previous_key(&mut self) {
        if !self.keys.is_empty() && self.selected_key > 0 {
            self.selected_key -= 1;
            self.key_list_state.select(Some(self.selected_key));
        }
    }

    pub fn toggle_key_selection(&mut self) {
        if !self.keys.is_empty() && self.selected_key < self.keys.len() {
            self.keys[self.selected_key].selected = !self.keys[self.selected_key].selected;
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

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileFilter {
    All,
    Encrypted,
    Decrypted,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FileSort {
    Alphabetical,
    EncryptedFirst,
    DecryptedFirst,
}

pub struct App {
    pub files: Vec<PathBuf>,
    pub raw_files: Vec<PathBuf>,
    pub selected_file: usize,
    pub file_list_state: ListState,
    pub keys: Vec<KeyInfo>,
    pub selected_key: usize,
    pub key_list_state: ListState,
    pub active_pane: Pane,
    pub input_mode: InputMode,
    pub input: String,
    pub logs: Vec<String>,
    pub file_filter: FileFilter,
    pub file_sort: FileSort,
}

pub struct KeyInfo {
    pub name: String,
    pub path: PathBuf,
    pub is_secret: bool,
    pub is_passphrase_only: bool,
    pub recipients: Vec<String>,
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
            raw_files: Vec::new(),
            selected_file: 0,
            file_list_state,
            keys: Vec::new(),
            selected_key: 0,
            key_list_state,
            active_pane: Pane::Files,
            input_mode: InputMode::Normal,
            input: String::new(),
            logs: vec!["Welcome to lazyage!".to_string()],
            file_filter: FileFilter::All,
            file_sort: FileSort::Alphabetical,
        }
    }

    pub fn set_files(&mut self, files: Vec<PathBuf>) {
        self.raw_files = files;
        self.apply_filter_and_sort();
    }

    pub fn next_filter(&mut self) {
        self.file_filter = match self.file_filter {
            FileFilter::All => FileFilter::Encrypted,
            FileFilter::Encrypted => FileFilter::Decrypted,
            FileFilter::Decrypted => FileFilter::All,
        };
        self.apply_filter_and_sort();
    }

    pub fn next_sort(&mut self) {
        self.file_sort = match self.file_sort {
            FileSort::Alphabetical => FileSort::EncryptedFirst,
            FileSort::EncryptedFirst => FileSort::DecryptedFirst,
            FileSort::DecryptedFirst => FileSort::Alphabetical,
        };
        self.apply_filter_and_sort();
    }

    pub fn apply_filter_and_sort(&mut self) {
        let selected_path = if !self.files.is_empty() && self.selected_file < self.files.len() {
            Some(self.files[self.selected_file].clone())
        } else {
            None
        };

        let mut filtered: Vec<PathBuf> = self.raw_files
            .iter()
            .filter(|path| {
                match self.file_filter {
                    FileFilter::All => true,
                    FileFilter::Encrypted => {
                        path.extension().and_then(|ext| ext.to_str()) == Some("age")
                    }
                    FileFilter::Decrypted => {
                        path.extension().and_then(|ext| ext.to_str()) == Some("decrypted")
                    }
                }
            })
            .cloned()
            .collect();

        filtered.sort_by(|a, b| {
            let a_name = a.file_name().unwrap_or_default().to_string_lossy();
            let b_name = b.file_name().unwrap_or_default().to_string_lossy();

            let cmp_names = || {
                let a_lower = a_name.to_lowercase();
                let b_lower = b_name.to_lowercase();
                if a_lower == b_lower {
                    a_name.cmp(&b_name)
                } else {
                    a_lower.cmp(&b_lower)
                }
            };

            match self.file_sort {
                FileSort::Alphabetical => cmp_names(),
                FileSort::EncryptedFirst => {
                    let a_is_age = a.extension().and_then(|ext| ext.to_str()) == Some("age");
                    let b_is_age = b.extension().and_then(|ext| ext.to_str()) == Some("age");
                    match (a_is_age, b_is_age) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => cmp_names(),
                    }
                }
                FileSort::DecryptedFirst => {
                    let a_is_dec = a.extension().and_then(|ext| ext.to_str()) == Some("decrypted");
                    let b_is_dec = b.extension().and_then(|ext| ext.to_str()) == Some("decrypted");
                    match (a_is_dec, b_is_dec) {
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                        _ => cmp_names(),
                    }
                }
            }
        });

        self.files = filtered;

        if let Some(path) = selected_path {
            if let Some(pos) = self.files.iter().position(|p| p == &path) {
                self.selected_file = pos;
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_sorting_and_filtering() {
        let mut app = App::new();
        let test_files = vec![
            PathBuf::from("B.decrypted"),
            PathBuf::from("a.age"),
            PathBuf::from("c.txt"),
            PathBuf::from("A.decrypted"),
            PathBuf::from("b.age"),
        ];

        // Set files - default filter: All, sort: Alphabetical
        app.set_files(test_files);
        
        // Alphabetical sort (case-insensitive)
        assert_eq!(
            app.files,
            vec![
                PathBuf::from("a.age"),
                PathBuf::from("A.decrypted"),
                PathBuf::from("b.age"),
                PathBuf::from("B.decrypted"),
                PathBuf::from("c.txt"),
            ]
        );

        // Filter: Encrypted
        app.next_filter(); // All -> Encrypted
        assert_eq!(app.file_filter, FileFilter::Encrypted);
        assert_eq!(
            app.files,
            vec![
                PathBuf::from("a.age"),
                PathBuf::from("b.age"),
            ]
        );

        // Filter: Decrypted
        app.next_filter(); // Encrypted -> Decrypted
        assert_eq!(app.file_filter, FileFilter::Decrypted);
        assert_eq!(
            app.files,
            vec![
                PathBuf::from("A.decrypted"),
                PathBuf::from("B.decrypted"),
            ]
        );

        // Filter: All again
        app.next_filter(); // Decrypted -> All
        assert_eq!(app.file_filter, FileFilter::All);

        // Sort: EncryptedFirst
        app.next_sort(); // Alphabetical -> EncryptedFirst
        assert_eq!(app.file_sort, FileSort::EncryptedFirst);
        assert_eq!(
            app.files,
            vec![
                PathBuf::from("a.age"),
                PathBuf::from("b.age"),
                PathBuf::from("A.decrypted"),
                PathBuf::from("B.decrypted"),
                PathBuf::from("c.txt"),
            ]
        );

        // Sort: DecryptedFirst
        app.next_sort(); // EncryptedFirst -> DecryptedFirst
        assert_eq!(app.file_sort, FileSort::DecryptedFirst);
        assert_eq!(
            app.files,
            vec![
                PathBuf::from("A.decrypted"),
                PathBuf::from("B.decrypted"),
                PathBuf::from("a.age"),
                PathBuf::from("b.age"),
                PathBuf::from("c.txt"),
            ]
        );
    }
}

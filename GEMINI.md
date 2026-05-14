# 🛡️ lazyage - Project Instructions

`lazyage` is a professional, cross-platform TUI for **age** encryption, inspired by the aesthetics of Lazygit. It is built with Rust and the **Ratatui** framework.

## 🏗️ Architecture & Core Components

- **Main Loop (`src/main.rs`):** Manages terminal initialization (raw mode, alternate screen) and the central event loop using `crossterm`.
- **State Management (`src/app.rs`):** The `App` struct holds the application state, including file/key lists, selected indices, active panes, and input modes.
- **UI & Rendering (`src/ui.rs`):** Defines the TUI layout and rendering logic for different panes (Files, Keys, Actions/Logs) and modals.
- **Cryptography (`src/crypto.rs`):** Implements high-level encryption and decryption using the native Rust `age` crate.
  - **Encryption:** Supports multiple recipients (age/SSH public keys) and optional passphrases. Defaults to ASCII armor.
  - **Decryption:** Automatically tries available secret keys and prompts for passphrases if required.
- **Discovery (`src/discovery.rs`):** Scans the filesystem for files and cryptographic keys.
  - **Keys:** Discovered in `~/.ssh/`, `~/.config/age/`, and the current directory. Supports `age` secret keys, `age` public keys, and SSH public keys (`ed25519`, `rsa`, etc.).

## 🛠️ Build & Development

### Commands

- **Build:** `cargo build`
- **Run:** `cargo run`
- **Test:** `cargo test`
- **Release:** `cargo build --release`

### Dependencies

- **ratatui:** TUI widgets and layout.
- **crossterm:** Terminal backend and event handling.
- **age:** Native implementation of the age encryption format.
- **directories / walkdir:** Filesystem navigation and key discovery.

## 📜 Development Conventions

- **TUI First:** All interactions should be possible via the TUI. Use modals for confirmations or text input.
- **Surgical Logic:** Cryptographic operations are encapsulated in `crypto.rs`. UI logic is separated from state in `ui.rs`.
- **Safety:** Always confirm destructive actions (like file deletion).
- **Armor by Default:** All encrypted files are produced with ASCII armor for maximum portability.
- **Key Discovery:** New key formats or locations should be added to `discovery.rs`.

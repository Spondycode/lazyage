# 🛡️ lazyage

A professional, cross-platform TUI for **age** encryption,
inspired by the aesthetics of Lazygit.

Built with Rust and [Ratatui](https://ratatui.rs/), `lazyage` provides a seamless terminal
interface for managing file encryption and decryption without needing
to remember complex CLI flags.

![License](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)

## ✨ Features

- **Lazygit-style UI**: Intuitive pane-based navigation.
- **File Explorer**: Browse and select files in your current directory.
- **Key Discovery**: Automatically finds `age` secret keys and SSH public
  keys (`~/.ssh/`).
- **Multiple Methods**:
  - Encrypt with public keys (`age1...` or `ssh-...`).
  - Encrypt with a passphrase.
- **File Preview**: Live preview of text files before encryption.
- **Sorting & Filtering**: Easily filter files (All, Encrypted `.age` only,
  or Decrypted `.decrypted` only) and change the sort order (Alphabetical,
  Encrypted First, or Decrypted First).
- **Safety First**: Confirmation modals for destructive actions like file
  deletion.
- **Armored Output**: All encryption uses ASCII armor by default for easy
  sharing.
- **Auto-Sync**: Automatically refreshes when you switch back to the terminal
  or switch panes.

## 🚀 Installation

### Homebrew (Recommended)

You can install `lazyage` via Homebrew by tapping the custom repository:

```bash
brew tap Spondycode/tap
brew install lazyage
```

### From Source

#### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (cargo)
- `age` (optional, as `lazyage` uses a native Rust implementation)

#### Build Steps

```bash
git clone https://github.com/Spondycode/lazyage.git
cd lazyage
cargo build --release
sudo cp target/release/lazyage /usr/local/bin/
```

## 🎮 Usage

Simply run `lazyage` in any directory:

```bash
lazyage
```

### Keybindings

| Key | Action |
| --- | ------ |
| `Tab` | Switch between Files and Keys panes |
| `↑/↓` or `k/j` | Navigate lists |
| `e` | **Encrypt** selected file with selected key |
| `p` | **Encrypt** selected file with **Passphrase** |
| `d` | **Decrypt** selected file |
| `f` | **Filter** files (All, Encrypted, Decrypted) |
| `s` | **Sort** files (Alpha, Encrypted first, Decrypted first) |
| `x` | **Delete** selected file (with confirmation) |
| `R` | **Refresh** file and key lists |
| `q` / `Esc` | Quit or Close Modal |

If you want to send to multiple recipients, you can create a recipients.txt file
in the config folder. ~/.config/age/
Looking in the Keys pane, you'll see that your text file will give you a count
of how many recipients.

## LazyAge supports the plug-ins, and in particular the YubiKey plugin

I find the best way to encrypt to YubiKey is to have a function in my
fish.config file. You can do something similar with ZSH.
You can encrypt through LazyAge encryption, then I recommend using a
shell script to decrypt.

## Fish function

```
# Age Yubikey with backup recipients
function vault
    age -R ~/.config/age/YubiKey.txt -o argv[1].vault.age $argv[1]
end

# Age Unlock the above encryption
function unlock
    age-plugin-yubikey --identity | grep AGE-PLUGIN | tr -d '[:space:]' >.tmp_key
    age -d -i .tmp_key $argv[1]
    rm .tmp_key
end
```

## 🏗️ Project Structure

- `src/app.rs`: Application state management.
- `src/ui.rs`: TUI layout and rendering logic.
- `src/crypto.rs`: Native `age` encryption/decryption.
- `src/discovery.rs`: Filesystem and key auto-detection.

## 📄 License

This project is licensed under the MIT License

- see the [LICENSE](LICENSE) file for details.

---

## Copyright Notice from Age

Copyright 2019 The age Authors
Copyright 2019 Google LLC
Copyright 2022 Filippo Valsorda

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are
met:

- Redistributions of source code must retain the above copyright
  notice, this list of conditions and the following disclaimer.
- Redistributions in binary form must reproduce the above
  copyright notice, this list of conditions and the following disclaimer
  in the documentation and/or other materials provided with the
  distribution.
- Neither the name of the age project nor the names of its
  contributors may be used to endorse or promote products derived from
  this software without specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
"AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

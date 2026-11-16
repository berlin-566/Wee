# ⚡ Wee

> The most lightweight, dedicated local HTML viewer written in pure Rust.

`wee` is built with a single purpose: **to view local HTML files effortlessly without the bloat, telemetry, or resource overhead of full desktop browsers.**

With a release binary of just **~1.6 MB** and near-instant startup, `wee` gives you a distraction-free window powered by your system's native WebKit engine.

---

## ✨ Features

- **Ultra-Lightweight & Fast:** Standalone **1.6 MB** native executable with minimal memory consumption.
- **Zero Browser Clutter:** Clean, borderless-ready native window strictly displaying your HTML content.
- **Safe Local Asset Resolution:** Effortlessly handles local relative assets (`<img src="...">`, `<link href="...">`, `<script src="...">`) through a dedicated in-memory custom protocol (`wee://`).
- **Smart Link Routing:** Local links navigate inside `wee`, while external `http://` and `https://` links automatically hand off to your default browser (Firefox, Chrome, etc.).
- **Live-Reload (File Watcher):** Optional `--watch` (`-w`) flag auto-reloads the view whenever you edit and save your HTML, CSS, or JS files.
- **Developer Tools on Demand:** Optional `--devtools` (`-d`) flag or `F12` opens WebKit DevTools and the JavaScript Console.
- **Dynamic Window Title:** Automatically syncs the window title with the HTML document's `<title>` tag.

---

## ⌨️ Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + R` or `F5` | Reload page |
| `Ctrl + Q` | Exit `wee` |
| `F12` | Toggle DevTools (when `-d` is enabled) |
| `Alt + ←` | Go Back |
| `Alt + →` | Go Forward |

---

## 🚀 Installation & Building

### Prerequisites

Ensure you have Rust and the WebKitGTK development packages installed:

```bash
# Arch Linux / CachyOS:
sudo pacman -S webkit2gtk-4.1

# Ubuntu / Debian:
sudo apt install libwebkit2gtk-4.1-dev

# Fedora:
sudo dnf install webkit2gtk4.1-devel
```

### Build from Source

```bash
git clone https://github.com/berlin-566/Wee.git
cd Wee

# Build the release binary
cargo build --release

# Install binary to ~/.cargo/bin (so you can run `wee` anywhere)
cargo install --path .
```

---

## 📖 Usage

### Quick Start

```bash
# View an HTML file directly:
wee path/to/document.html

# View using the -p flag:
wee -p path/to/document.html

# Open a folder (automatically looks for index.html):
wee ./my-docs

# Open with automatic live-reload on file save:
wee -w index.html

# Open with WebKit Developer Tools (F12 / Console):
wee -d index.html

# Launch in fullscreen mode:
wee --fullscreen index.html

# Set custom window dimensions:
wee --width 1280 --height 800 index.html
```

### Command Line Options

```text
Usage: wee [OPTIONS] [PATH]

Arguments:
  [PATH]  Path to local HTML file or directory containing index.html

Options:
  -p, --path <PATH>      Path flag alternative (-p / --path)
  -w, --watch            Enable live-reload (auto-refreshes when file or directory changes)
  -d, --devtools         Enable Developer Tools (F12 / right-click Inspect Element)
      --width <WIDTH>    Initial window width [default: 1024]
      --height <HEIGHT>  Initial window height [default: 768]
      --fullscreen       Start in fullscreen mode
  -t, --title <TITLE>    Custom window title (defaults to document <title>)
  -h, --help             Print help
  -V, --version          Print version
```

---

## 🧪 Try the Demo

Run the included showcase document:

```bash
cargo run -- examples/demo.html -w -d
```

---

## 📄 License

MIT License. See [LICENSE](LICENSE) for details.

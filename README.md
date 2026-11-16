# Wee

The most lightweight, dedicated local HTML viewer written in Rust.

Wee is built for one purpose: to view local HTML files effortlessly without the bloat, telemetry, or resource overhead of full desktop web browsers.

With a release binary of approximately 1.6 MB and fast startup, Wee provides a distraction-free window powered by your system's native WebKit engine.

## Installation

### One-liner (Linux x86_64)

Install the latest prebuilt release binary to `~/.local/bin/wee`:

```bash
curl -fsSL https://raw.githubusercontent.com/berlin-566/Wee/main/install.sh | sh
```

### Install via Cargo

```bash
cargo install --git https://github.com/berlin-566/Wee.git
```

### Build from Source

Prerequisites:
- Arch Linux / CachyOS: `sudo pacman -S webkit2gtk-4.1`
- Ubuntu / Debian: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev`
- Fedora: `sudo dnf install webkit2gtk4.1-devel gtk3-devel`

Build:

```bash
git clone https://github.com/berlin-566/Wee.git
cd Wee
cargo build --release
cargo install --path .
```

## Features

- Lightweight and fast: Standalone 1.6 MB native executable with minimal memory consumption.
- Zero browser clutter: Clean native window strictly displaying your HTML content.
- Native Wayland and X11 support: Seamlessly integrates with modern Linux desktop environments.
- Safe local asset resolution: Handles relative assets (images, stylesheets, scripts) via internal protocol (`wee://`).
- Smart link routing: Local links stay inside Wee; external web links (`http://`, `https://`) automatically open in your default system browser.
- Live-reload: Optional `--watch` (`-w`) flag auto-reloads whenever you save edits to local files.
- Developer tools: Optional `--devtools` (`-d`) flag or `F12` key opens WebKit DevTools and the JavaScript Console.
- Dynamic window title: Automatically synchronizes window title with the HTML `<title>` tag.

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Ctrl + R` or `F5` | Reload current page |
| `Ctrl + Q` | Quit Wee |
| `F12` | Toggle DevTools (when launched with `-d`) |
| `Alt + Left` | Navigate back |
| `Alt + Right` | Navigate forward |

## Usage

```bash
# View an HTML file directly
wee path/to/document.html

# View using the -p flag
wee -p path/to/document.html

# Open a directory (automatically finds index.html)
wee ./my-docs

# Open with automatic live-reload on file save
wee -w index.html

# Open with WebKit Developer Tools (F12 / Console)
wee -d index.html

# Launch in fullscreen mode
wee --fullscreen index.html

# Set custom window dimensions
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

## License

MIT License. See LICENSE for details.

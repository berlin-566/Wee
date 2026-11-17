# Wee

The most lightweight, dedicated local HTML viewer and live wallpaper daemon written in Rust.

Wee is built for viewing local HTML files effortlessly without the bloat, telemetry, or resource overhead of full desktop web browsers, and doubles as an animated, live HTML/video desktop wallpaper daemon for Linux.

With a release binary of approximately 1.6 MB and fast startup, Wee provides a distraction-free window or desktop background powered by your system's native WebKit engine.

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
- Arch Linux / CachyOS: `sudo pacman -S webkit2gtk-4.1 gtk-layer-shell`
- Ubuntu / Debian: `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libgtk-layer-shell-dev`
- Fedora: `sudo dnf install webkit2gtk4.1-devel gtk3-devel gtk-layer-shell-devel`

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
- Native Wayland & Layer-Shell support: Renders behind all windows as a desktop wallpaper on compositors like Niri, Sway, and Hyprland.
- Live Wallpaper Daemon (`-dmn`): Sets any HTML page or looping video as your background, persisting across system reboots.
- Daemon Management (`wee kill`): Safely terminates background instances and cleanly reveals your previous wallpaper (like eww).
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

### Document Viewer

```bash
# View an HTML file directly
wee path/to/document.html

# View using the -p flag
wee -p path/to/document.html

# Open detached from terminal as a standalone app (Ctrl+C won't kill it)
wee -n document.html

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

### Live Wallpaper Daemon Mode

```bash
# Set an HTML page or video as your desktop wallpaper (runs in background)
wee ~/path/to/wallpaper.html -dmn

# Set interactive wallpaper: click-through by default; hold Spacebar to interact with it
wee ~/path/to/interactive.html -dmn -s

# Restore last used wallpaper on login or reboot
wee -dmn

# Run with audio enabled (muted by default for quiet desktop use)
wee ~/path/to/video-wallpaper.html -dmn --audio

# Stop running wallpaper daemon and remove autostart entry
wee kill

# Terminate all running wee wallpaper instances
wee kill -a
```

### Command Line Options

```text
Usage: wee [COMMAND] [OPTIONS] [PATH]

Commands:
  kill                   Kill running wee wallpaper daemons and clean up autostart

Arguments:
  [PATH]                 Path to local HTML file or directory containing index.html

Options:
  -p, --path <PATH>      Path flag alternative (-p / --path)
  -D, --dmn              Run as wallpaper daemon in background (alias: --daemon)
  -s, --space            Enable Spacebar hold-to-interact mode (click-through by default; hold Space to interact)
  -n, --detach           Run detached in background as a standalone app (unhooked from terminal)
      --audio            Enable audio in wallpaper daemon mode (muted by default)
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

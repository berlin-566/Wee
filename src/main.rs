use clap::{Parser, Subcommand};
use notify::Watcher;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(
    name = "wee",
    about = "An ultra-lightweight local HTML page viewer and wallpaper daemon built in Rust",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to local HTML file or directory containing index.html
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Path flag alternative (-p / --path)
    #[arg(short = 'p', long = "path", value_name = "PATH")]
    path_flag: Option<PathBuf>,

    /// Run as wallpaper daemon in background
    #[arg(short = 'D', long = "dmn", visible_aliases = ["daemon", "wallpaper"])]
    daemon: bool,

    /// Run detached in background as a standalone app (unhooked from terminal)
    #[arg(short = 'n', long = "detach", visible_alias = "no-terminal")]
    detach: bool,

    /// Enable audio in wallpaper daemon mode (muted by default)
    #[arg(long = "audio")]
    audio: bool,

    /// Enable live-reload (auto-refreshes when file or directory changes)
    #[arg(short = 'w', long = "watch")]
    watch: bool,

    /// Enable Developer Tools (F12 / right-click Inspect Element)
    #[arg(short = 'd', long = "devtools")]
    devtools: bool,

    /// Initial window width
    #[arg(long, default_value_t = 1024)]
    width: u32,

    /// Initial window height
    #[arg(long, default_value_t = 768)]
    height: u32,

    /// Start in fullscreen mode
    #[arg(long)]
    fullscreen: bool,

    /// Custom window title (defaults to document <title>)
    #[arg(short = 't', long = "title")]
    title: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Kill running wee wallpaper daemons and clean up autostart
    Kill {
        /// Kill all running instances
        #[arg(short = 'a', long = "all")]
        all: bool,
    },
}

#[derive(Debug)]
enum UserEvent {
    Quit,
    Reload,
    ToggleDevTools,
    SetTitle(String),
}

fn get_config_dir() -> PathBuf {
    if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(p).join("wee")
    } else if let Ok(h) = std::env::var("HOME") {
        PathBuf::from(h).join(".config").join("wee")
    } else {
        PathBuf::from(".config/wee")
    }
}

fn get_state_dir() -> PathBuf {
    if let Ok(p) = std::env::var("XDG_STATE_HOME") {
        PathBuf::from(p).join("wee")
    } else if let Ok(h) = std::env::var("HOME") {
        PathBuf::from(h).join(".local/state").join("wee")
    } else {
        PathBuf::from(".local/state/wee")
    }
}

fn get_wallpaper_conf_path() -> PathBuf {
    get_config_dir().join("wallpaper.conf")
}

fn get_pid_file_path() -> PathBuf {
    get_state_dir().join("wallpaper.pid")
}

fn get_autostart_file_path() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("XDG_CONFIG_HOME") {
        Some(PathBuf::from(p).join("autostart").join("wee-wallpaper.desktop"))
    } else if let Ok(h) = std::env::var("HOME") {
        Some(PathBuf::from(h).join(".config").join("autostart").join("wee-wallpaper.desktop"))
    } else {
        None
    }
}

fn handle_kill_command(all: bool) {
    let pid_file = get_pid_file_path();
    let mut killed_any = false;

    if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                unsafe {
                    if libc::kill(pid, 0) == 0 {
                        libc::kill(pid, libc::SIGTERM);
                        let start = Instant::now();
                        while start.elapsed() < Duration::from_millis(1500) {
                            if libc::kill(pid, 0) != 0 {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        if libc::kill(pid, 0) == 0 {
                            libc::kill(pid, libc::SIGKILL);
                        }
                        println!("[wee] Stopped wallpaper daemon (PID {}).", pid);
                        killed_any = true;
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&pid_file);
    }

    if all {
        let current_pid = std::process::id() as i32;
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                if let Ok(name) = entry.file_name().into_string() {
                    if let Ok(pid) = name.parse::<i32>() {
                        if pid != current_pid {
                            let cmdline_path = entry.path().join("cmdline");
                            if let Ok(cmdline) = std::fs::read(cmdline_path) {
                                let cmd = String::from_utf8_lossy(&cmdline);
                                if cmd.contains("wee") && (cmd.contains("dmn") || cmd.contains("daemon") || cmd.contains("_WEE_DAEMON_INTERNAL")) {
                                    unsafe {
                                        libc::kill(pid, libc::SIGTERM);
                                    }
                                    println!("[wee] Terminated wee instance (PID {}).", pid);
                                    killed_any = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(autostart_path) = get_autostart_file_path() {
        if autostart_path.exists() {
            let _ = std::fs::remove_file(&autostart_path);
            println!("[wee] Removed autostart entry.");
        }
    }

    if !killed_any {
        println!("[wee] No running wallpaper daemon found.");
    }
}

fn write_autostart_file(exe_path: &Path) {
    if let Some(autostart_path) = get_autostart_file_path() {
        if let Some(parent) = autostart_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=Wee Wallpaper Daemon\n\
             Comment=Wee Live HTML Wallpaper Daemon\n\
             Exec={} -dmn\n\
             Terminal=false\n\
             StartupNotify=false\n\
             X-GNOME-Autostart-enabled=true\n",
            exe_path.display()
        );
        let _ = std::fs::write(autostart_path, content);
    }
}

fn stop_existing_daemon() {
    let pid_file = get_pid_file_path();
    if pid_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&pid_file) {
            if let Ok(pid) = content.trim().parse::<i32>() {
                unsafe {
                    if libc::kill(pid, 0) == 0 {
                        libc::kill(pid, libc::SIGTERM);
                        let start = Instant::now();
                        while start.elapsed() < Duration::from_millis(1000) {
                            if libc::kill(pid, 0) != 0 {
                                break;
                            }
                            std::thread::sleep(Duration::from_millis(50));
                        }
                        if libc::kill(pid, 0) == 0 {
                            libc::kill(pid, libc::SIGKILL);
                        }
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&pid_file);
    }
}

fn resolve_html_target(cli_path: Option<PathBuf>, is_daemon: bool) -> Result<(PathBuf, PathBuf), String> {
    let target = match cli_path {
        Some(p) => p,
        None => {
            if is_daemon {
                let conf = get_wallpaper_conf_path();
                if conf.exists() {
                    if let Ok(saved) = std::fs::read_to_string(&conf) {
                        let trimmed = saved.trim();
                        if !trimmed.is_empty() {
                            PathBuf::from(trimmed)
                        } else {
                            return Err("Saved wallpaper configuration is empty.".to_string());
                        }
                    } else {
                        return Err("Failed to read saved wallpaper configuration.".to_string());
                    }
                } else {
                    return Err(
                        "No HTML file provided and no saved wallpaper found.\n\
                         Usage: wee <path/to/file.html> -dmn"
                            .to_string(),
                    );
                }
            } else {
                let cwd_index = Path::new("index.html");
                if cwd_index.exists() {
                    PathBuf::from("index.html")
                } else {
                    return Err(
                        "No HTML file provided and no 'index.html' found in current directory.\n\
                         Usage: wee <path/to/file.html> [-w] [-d] [-dmn]"
                            .to_string(),
                    );
                }
            }
        }
    };

    let canonical = target
        .canonicalize()
        .map_err(|e| format!("Failed to find path '{}': {}", target.display(), e))?;

    if canonical.is_dir() {
        let index_candidate = canonical.join("index.html");
        if index_candidate.exists() && index_candidate.is_file() {
            let base_dir = canonical.clone();
            Ok((index_candidate, base_dir))
        } else {
            Err(format!(
                "Directory '{}' does not contain an 'index.html' file.",
                canonical.display()
            ))
        }
    } else if canonical.is_file() {
        let base_dir = canonical
            .parent()
            .unwrap_or_else(|| Path::new("/"))
            .to_path_buf();
        Ok((canonical, base_dir))
    } else {
        Err(format!("'{}' is not a valid file or directory.", canonical.display()))
    }
}

fn create_404_response(missing_path: &str) -> wry::http::Response<Cow<'static, [u8]>> {
    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <title>404 Not Found - wee</title>
    <style>
        body {{
            font-family: system-ui, -apple-system, sans-serif;
            background: #111318;
            color: #e2e8f0;
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
        }}
        .card {{
            background: #1a1f29;
            border: 1px solid #2d3748;
            padding: 2.5rem;
            border-radius: 12px;
            max-width: 500px;
            box-shadow: 0 10px 25px rgba(0,0,0,0.5);
        }}
        h1 {{ margin-top: 0; color: #f56565; font-size: 1.5rem; }}
        code {{ background: #2d3748; padding: 0.2rem 0.4rem; border-radius: 4px; font-size: 0.9em; }}
        p {{ line-height: 1.6; color: #a0aec0; }}
    </style>
</head>
<body>
    <div class="card">
        <h1>404 File Not Found</h1>
        <p>The requested file could not be found:</p>
        <p><code>{}</code></p>
    </div>
</body>
</html>"#,
        html_escape(missing_path)
    );

    wry::http::Response::builder()
        .status(404)
        .header("Content-Type", "text/html; charset=utf-8")
        .header("Cache-Control", "no-cache, no-store, must-revalidate")
        .body(Cow::Owned(html.into_bytes()))
        .unwrap()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

const INJECTED_HOTKEYS_SCRIPT: &str = r#"
(function() {
    window.addEventListener('keydown', function(e) {
        if (((e.ctrlKey || e.metaKey) && (e.key === 'r' || e.key === 'R')) || e.key === 'F5') {
            e.preventDefault();
            if (window.ipc) { window.ipc.postMessage('reload'); }
            else { location.reload(); }
        }
        else if ((e.ctrlKey || e.metaKey) && (e.key === 'q' || e.key === 'Q')) {
            e.preventDefault();
            if (window.ipc) { window.ipc.postMessage('quit'); }
        }
        else if (e.key === 'F12') {
            e.preventDefault();
            if (window.ipc) { window.ipc.postMessage('toggle_devtools'); }
        }
        else if (e.altKey && e.key === 'ArrowLeft') {
            e.preventDefault();
            window.history.back();
        }
        else if (e.altKey && e.key === 'ArrowRight') {
            e.preventDefault();
            window.history.forward();
        }
    });
})();
"#;

const INJECTED_MUTED_VIDEO_SCRIPT: &str = r#"
(function() {
    function enforceBackgroundVideo() {
        document.querySelectorAll('video, audio').forEach(function(el) {
            el.muted = true;
            if (el.tagName === 'VIDEO') {
                el.loop = true;
                el.playsInline = true;
            }
            if (el.paused) {
                el.play().catch(function(){});
            }
        });
    }
    window.addEventListener('DOMContentLoaded', enforceBackgroundVideo);
    window.addEventListener('load', enforceBackgroundVideo);
    var observer = new MutationObserver(enforceBackgroundVideo);
    observer.observe(document.documentElement, { childList: true, subtree: true });
})();
"#;

const INJECTED_UNMUTED_VIDEO_SCRIPT: &str = r#"
(function() {
    function enforceBackgroundVideo() {
        document.querySelectorAll('video').forEach(function(el) {
            el.loop = true;
            el.playsInline = true;
            if (el.paused) {
                el.play().catch(function(){});
            }
        });
    }
    window.addEventListener('DOMContentLoaded', enforceBackgroundVideo);
    window.addEventListener('load', enforceBackgroundVideo);
    var observer = new MutationObserver(enforceBackgroundVideo);
    observer.observe(document.documentElement, { childList: true, subtree: true });
})();
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let raw_args: Vec<String> = std::env::args()
        .map(|arg| {
            if arg == "-dmn" {
                "--dmn".to_string()
            } else if arg == "-path" {
                "--path".to_string()
            } else {
                arg
            }
        })
        .collect();

    let args = Cli::parse_from(raw_args);

    if let Some(Commands::Kill { all }) = args.command {
        handle_kill_command(all);
        return Ok(());
    }

    let is_daemon = args.daemon;
    let requested_path = args.path.or(args.path_flag);

    let (html_file, base_dir) = match resolve_html_target(requested_path, is_daemon) {
        Ok(paths) => paths,
        Err(err_msg) => {
            eprintln!("\x1b[1;31merror:\x1b[0m {}", err_msg);
            std::process::exit(1);
        }
    };

    if is_daemon {
        let conf_dir = get_config_dir();
        let _ = std::fs::create_dir_all(&conf_dir);
        let _ = std::fs::write(get_wallpaper_conf_path(), html_file.display().to_string());

        if let Ok(exe) = std::env::current_exe() {
            write_autostart_file(&exe);
        }
    }

    if is_daemon && std::env::var("_WEE_DAEMON_INTERNAL").is_err() {
        stop_existing_daemon();

        let exe = std::env::current_exe()?;
        let mut cmd = std::process::Command::new(exe);
        cmd.arg(&html_file);
        cmd.arg("--dmn");
        if args.audio {
            cmd.arg("--audio");
        }
        cmd.env("_WEE_DAEMON_INTERNAL", "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        let child = cmd.spawn()?;
        let pid = child.id() as i32;

        let pid_path = get_pid_file_path();
        let _ = std::fs::create_dir_all(get_state_dir());
        let _ = std::fs::write(pid_path, pid.to_string());

        println!("[wee] Wallpaper daemon started (PID {}).", pid);
        std::process::exit(0);
    }

    if args.detach && std::env::var("_WEE_DETACHED").is_err() && !is_daemon {
        let exe = std::env::current_exe()?;
        let mut cmd = std::process::Command::new(exe);
        cmd.arg(&html_file);
        if args.watch {
            cmd.arg("-w");
        }
        if args.devtools {
            cmd.arg("-d");
        }
        if args.fullscreen {
            cmd.arg("--fullscreen");
        }
        if let Some(ref title) = args.title {
            cmd.arg("-t").arg(title);
        }
        cmd.arg("--width").arg(args.width.to_string());
        cmd.arg("--height").arg(args.height.to_string());

        cmd.env("_WEE_DETACHED", "1")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        #[cfg(unix)]
        unsafe {
            use std::os::unix::process::CommandExt;
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }

        cmd.spawn()?;
        std::process::exit(0);
    }

    let filename = html_file
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "document.html".to_string());

    let initial_title = args
        .title
        .clone()
        .unwrap_or_else(|| format!("{} - wee", filename));

    let has_custom_title = args.title.is_some();

    let event_loop = tao::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let mut window_builder = tao::window::WindowBuilder::new()
        .with_title(&initial_title)
        .with_visible(!is_daemon)
        .with_decorations(!is_daemon)
        .with_inner_size(tao::dpi::LogicalSize::new(args.width as f64, args.height as f64));

    if args.fullscreen && !is_daemon {
        window_builder = window_builder.with_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
    }

    let window = window_builder.build(&event_loop)?;

    #[cfg(target_os = "linux")]
    if is_daemon {
        use tao::platform::unix::WindowExtUnix;

        if gtk_layer_shell::is_supported() {
            use gtk_layer_shell::{Edge, Layer, LayerShell};
            let gtk_win = window.gtk_window();
            gtk_win.init_layer_shell();
            gtk_win.set_layer(Layer::Background);
            gtk_win.set_anchor(Edge::Top, true);
            gtk_win.set_anchor(Edge::Bottom, true);
            gtk_win.set_anchor(Edge::Left, true);
            gtk_win.set_anchor(Edge::Right, true);
            gtk_win.set_exclusive_zone(-1);
            gtk_win.set_namespace("wee-wallpaper");
            gtk_win.set_keyboard_mode(gtk_layer_shell::KeyboardMode::None);
            window.set_visible(true);
        } else {
            use gtk::prelude::GtkWindowExt;
            let gtk_win = window.gtk_window();
            gtk_win.set_keep_below(true);
            gtk_win.set_skip_taskbar_hint(true);
            gtk_win.set_skip_pager_hint(true);
            window.set_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
            window.set_visible(true);
        }
    }

    let _watcher = if args.watch && !is_daemon {
        let p_watch = proxy.clone();
        let last_reload = Arc::new(AtomicU64::new(0));
        let watch_dir = base_dir.clone();

        println!(
            "\x1b[1;32m[wee]\x1b[0m Live-reload active. Watching: \x1b[1m{}\x1b[0m",
            watch_dir.display()
        );

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                use notify::event::EventKind;
                let is_relevant = match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => true,
                    _ => false,
                };

                if is_relevant {
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    let prev = last_reload.load(Ordering::Relaxed);
                    if now.saturating_sub(prev) > 120 {
                        last_reload.store(now, Ordering::Relaxed);
                        let _ = p_watch.send_event(UserEvent::Reload);
                    }
                }
            }
        })?;

        watcher.watch(&watch_dir, notify::RecursiveMode::Recursive)?;
        Some(watcher)
    } else {
        None
    };

    let rel_entry = html_file
        .strip_prefix(&base_dir)
        .unwrap_or(&html_file)
        .to_string_lossy()
        .replace('\\', "/");

    let initial_url = format!("wee://localhost/{}", rel_entry);

    let mut webview_builder = wry::WebViewBuilder::new();
    webview_builder = webview_builder.with_devtools(args.devtools);
    webview_builder = webview_builder.with_autoplay(true);
    webview_builder = webview_builder.with_initialization_script(INJECTED_HOTKEYS_SCRIPT);

    if is_daemon {
        if args.audio {
            webview_builder = webview_builder.with_initialization_script(INJECTED_UNMUTED_VIDEO_SCRIPT);
        } else {
            webview_builder = webview_builder.with_initialization_script(INJECTED_MUTED_VIDEO_SCRIPT);
        }
    }

    let p_ipc = proxy.clone();
    let devtools_enabled = args.devtools;
    webview_builder = webview_builder.with_ipc_handler(move |req: wry::http::Request<String>| {
        match req.body().as_str() {
            "quit" => {
                let _ = p_ipc.send_event(UserEvent::Quit);
            }
            "reload" => {
                let _ = p_ipc.send_event(UserEvent::Reload);
            }
            "toggle_devtools" => {
                if devtools_enabled {
                    let _ = p_ipc.send_event(UserEvent::ToggleDevTools);
                }
            }
            _ => {}
        }
    });

    let p_title = proxy.clone();
    webview_builder = webview_builder.with_document_title_changed_handler(move |doc_title| {
        if !doc_title.trim().is_empty() {
            let _ = p_title.send_event(UserEvent::SetTitle(doc_title));
        }
    });

    webview_builder = webview_builder.with_navigation_handler(move |nav_url: String| {
        if nav_url.starts_with("http://")
            || nav_url.starts_with("https://")
            || nav_url.starts_with("mailto:")
            || nav_url.starts_with("tel:")
        {
            let _ = open::that_detached(&nav_url);
            false
        } else {
            true
        }
    });

    let serve_base = base_dir.clone();
    webview_builder = webview_builder.with_custom_protocol("wee".into(), move |_id, req| {
        let uri_path = req.uri().path();
        let decoded = percent_encoding::percent_decode_str(uri_path).decode_utf8_lossy();
        let trimmed = decoded.trim_start_matches('/');

        let target = if trimmed.is_empty() {
            serve_base.join("index.html")
        } else {
            serve_base.join(trimmed)
        };

        if let Ok(canonical_target) = target.canonicalize() {
            if canonical_target.is_file() {
                match std::fs::read(&canonical_target) {
                    Ok(bytes) => {
                        let mime = mime_guess::from_path(&canonical_target)
                            .first_or_octet_stream()
                            .to_string();

                        return wry::http::Response::builder()
                            .header("Content-Type", mime)
                            .header("Access-Control-Allow-Origin", "*")
                            .header("Cache-Control", "no-cache, no-store, must-revalidate")
                            .body(Cow::Owned(bytes))
                            .unwrap();
                    }
                    Err(e) => {
                        let err_body = format!("<h1>500 Internal Error</h1><p>{}</p>", e);
                        return wry::http::Response::builder()
                            .status(500)
                            .header("Content-Type", "text/html")
                            .body(Cow::Owned(err_body.into_bytes()))
                            .unwrap();
                    }
                }
            }
        }

        create_404_response(uri_path)
    });

    webview_builder = webview_builder.with_url(&initial_url);

    #[cfg(target_os = "linux")]
    let webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().expect("Failed to get window default vbox");
        webview_builder.build_gtk(vbox)?
    };

    #[cfg(not(target_os = "linux"))]
    let webview = webview_builder.build(&window)?;

    if !is_daemon {
        println!(
            "\x1b[1;36m[wee]\x1b[0m Viewing: \x1b[1m{}\x1b[0m",
            html_file.display()
        );
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = tao::event_loop::ControlFlow::Wait;

        match event {
            tao::event::Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = tao::event_loop::ControlFlow::Exit;
            }

            tao::event::Event::UserEvent(user_event) => match user_event {
                UserEvent::Quit => {
                    *control_flow = tao::event_loop::ControlFlow::Exit;
                }
                UserEvent::Reload => {
                    let _ = webview.evaluate_script("location.reload();");
                }
                UserEvent::ToggleDevTools => {
                    if args.devtools {
                        if webview.is_devtools_open() {
                            webview.close_devtools();
                        } else {
                            webview.open_devtools();
                        }
                    }
                }
                UserEvent::SetTitle(doc_title) => {
                    if !has_custom_title && !is_daemon {
                        window.set_title(&format!("{} - wee", doc_title));
                    }
                }
            },

            _ => (),
        }
    });
}

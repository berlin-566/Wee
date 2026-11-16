use clap::Parser;
use notify::Watcher;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(
    name = "wee",
    about = "An ultra-lightweight, dedicated local HTML page viewer built in Rust",
    version
)]
struct Cli {
    /// Path to local HTML file or directory containing index.html
    #[arg(value_name = "PATH")]
    path: Option<PathBuf>,

    /// Path flag alternative (-p / --path)
    #[arg(short = 'p', long = "path", value_name = "PATH")]
    path_flag: Option<PathBuf>,

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

#[derive(Debug)]
enum UserEvent {
    Quit,
    Reload,
    ToggleDevTools,
    SetTitle(String),
}

fn resolve_html_target(cli_path: Option<PathBuf>) -> Result<(PathBuf, PathBuf), String> {
    let target = match cli_path {
        Some(p) => p,
        None => {
            // Check if index.html exists in current working directory
            let cwd_index = Path::new("index.html");
            if cwd_index.exists() {
                PathBuf::from("index.html")
            } else {
                return Err(
                    "No HTML file provided and no 'index.html' found in current directory.\n\
                     Usage: wee <path/to/file.html> [-w] [-d]"
                        .to_string(),
                );
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
    <title>404 Not Found — wee</title>
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
        // Ctrl+R or F5: Reload
        if (((e.ctrlKey || e.metaKey) && (e.key === 'r' || e.key === 'R')) || e.key === 'F5') {
            e.preventDefault();
            if (window.ipc) { window.ipc.postMessage('reload'); }
            else { location.reload(); }
        }
        // Ctrl+Q: Exit
        else if ((e.ctrlKey || e.metaKey) && (e.key === 'q' || e.key === 'Q')) {
            e.preventDefault();
            if (window.ipc) { window.ipc.postMessage('quit'); }
        }
        // F12: Toggle DevTools
        else if (e.key === 'F12') {
            e.preventDefault();
            if (window.ipc) { window.ipc.postMessage('toggle_devtools'); }
        }
        // Alt+Left Arrow: Back
        else if (e.altKey && e.key === 'ArrowLeft') {
            e.preventDefault();
            window.history.back();
        }
        // Alt+Right Arrow: Forward
        else if (e.altKey && e.key === 'ArrowRight') {
            e.preventDefault();
            window.history.forward();
        }
    });
})();
"#;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::parse();
    let requested_path = args.path.or(args.path_flag);

    let (html_file, base_dir) = match resolve_html_target(requested_path) {
        Ok(paths) => paths,
        Err(err_msg) => {
            eprintln!("\x1b[1;31merror:\x1b[0m {}", err_msg);
            std::process::exit(1);
        }
    };

    let filename = html_file
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "document.html".to_string());

    let initial_title = args
        .title
        .clone()
        .unwrap_or_else(|| format!("{} — wee", filename));

    let has_custom_title = args.title.is_some();

    // Set up tao event loop and proxy for cross-thread event dispatching
    let event_loop = tao::event_loop::EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // Build the native window
    let mut window_builder = tao::window::WindowBuilder::new()
        .with_title(&initial_title)
        .with_inner_size(tao::dpi::LogicalSize::new(args.width as f64, args.height as f64));

    if args.fullscreen {
        window_builder = window_builder.with_fullscreen(Some(tao::window::Fullscreen::Borderless(None)));
    }

    let window = window_builder.build(&event_loop)?;

    // Set up optional live-reload file watcher
    let _watcher = if args.watch {
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
                    // Debounce filesystem events within 120ms
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

    // Calculate relative entry file inside base_dir
    let rel_entry = html_file
        .strip_prefix(&base_dir)
        .unwrap_or(&html_file)
        .to_string_lossy()
        .replace('\\', "/");

    let initial_url = format!("wee://localhost/{}", rel_entry);

    // Build wry webview
    let mut webview_builder = wry::WebViewBuilder::new();
    webview_builder = webview_builder.with_devtools(args.devtools);
    webview_builder = webview_builder.with_initialization_script(INJECTED_HOTKEYS_SCRIPT);

    // IPC handler for JavaScript keyboard shortcuts
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

    // Sync window title with HTML <title> tag
    let p_title = proxy.clone();
    webview_builder = webview_builder.with_document_title_changed_handler(move |doc_title| {
        if !doc_title.trim().is_empty() {
            let _ = p_title.send_event(UserEvent::SetTitle(doc_title));
        }
    });

    // Navigation interceptor: route http/https links to external default browser
    webview_builder = webview_builder.with_navigation_handler(move |nav_url: String| {
        if nav_url.starts_with("http://")
            || nav_url.starts_with("https://")
            || nav_url.starts_with("mailto:")
            || nav_url.starts_with("tel:")
        {
            let _ = open::that_detached(&nav_url);
            false // Do not navigate internally; opened in system browser
        } else {
            true // Allow local wee:// navigation
        }
    });

    // Custom protocol handler to serve local files & assets safely
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

    // Load initial URL
    webview_builder = webview_builder.with_url(&initial_url);

    let webview = webview_builder.build(&window)?;

    println!(
        "\x1b[1;36m[wee]\x1b[0m Viewing: \x1b[1m{}\x1b[0m",
        html_file.display()
    );

    // Run the native application event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = tao::event_loop::ControlFlow::Wait;

        match event {
            tao::event::Event::WindowEvent {
                event: tao::event::WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = tao::event_loop::ControlFlow::Exit;
            }

            tao::event::Event::WindowEvent {
                event:
                    tao::event::WindowEvent::KeyboardInput {
                        event: _key_event,
                        ..
                    },
                ..
            } => {
                // Key events inside webview are captured by INJECTED_HOTKEYS_SCRIPT
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
                    if !has_custom_title {
                        window.set_title(&format!("{} — wee", doc_title));
                    }
                }
            },

            _ => (),
        }
    });
}

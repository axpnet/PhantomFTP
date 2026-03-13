//! PhantomFTP — CyberPunk TUI FTP Client
//!
//! A modern, asynchronous FTP/FTPS client with a Terminal User Interface.

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use tokio::sync::mpsc;
use tracing::error;

use phantomftp::app::{App, AppEvent};
use phantomftp::config::Config;
use phantomftp::import::import_aeroftp_servers;
use phantomftp::ui;

/// PhantomFTP — CyberPunk TUI FTP/FTPS Client
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// FTP server address (e.g., ftp.example.com:21)
    #[arg(short, long)]
    server: Option<String>,

    /// Username for authentication
    #[arg(short, long)]
    username: Option<String>,

    /// Password for authentication
    #[arg(short, long)]
    password: Option<String>,

    /// Import servers from AeroFTP export JSON file
    #[arg(long, value_name = "FILE")]
    import: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    // Handle --import before entering TUI mode
    if let Some(ref import_path) = args.import {
        let path = std::path::Path::new(import_path);
        let config_path = Config::get_default_config_path();
        let config_str = config_path.to_string_lossy().to_string();

        let mut config = if config_path.exists() {
            Config::load_from_file(&config_str).unwrap_or_default()
        } else {
            Config::default()
        };

        match import_aeroftp_servers(path, &mut config) {
            Ok(0) => {
                eprintln!("No compatible servers found (only FTP/FTPS are supported).");
            }
            Ok(n) => {
                config.save_to_file(&config_str)?;
                eprintln!("Imported {} server(s) into {}", n, config_str);
            }
            Err(e) => {
                eprintln!("Import failed: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and pre-fill from CLI args
    let mut app = App::new();
    app.pre_fill_connection(args.server, args.username, args.password);

    // Run app
    let res = run_app(&mut terminal, &mut app).await;

    // Graceful disconnect before terminal cleanup
    app.shutdown().await;

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!("Application error: {:?}", err);
        eprintln!("\nError: {:?}\n", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
    app.set_event_sender(event_sender);

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        if app.can_quit() {
                            break;
                        }
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if !app.upload_in_progress && !app.download_in_progress {
                            break;
                        }
                        app.handle_event(AppEvent::Key(key));
                    }
                    _ => {
                        app.handle_event(AppEvent::Key(key));
                    }
                }
            }
        }

        while let Ok(event) = event_receiver.try_recv() {
            app.handle_event(event);
        }

        app.process_events().await;
    }

    Ok(())
}

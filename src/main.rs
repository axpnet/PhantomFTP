//! Rust FTP TUI Client - Main Entry Point
//! 
//! A modern, asynchronous FTP client with a Terminal User Interface.

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
use tracing::{error, info};

mod app;
mod config;
mod ftp;
mod ui;

use app::{App, AppEvent};

/// Simple FTP client with TUI interface
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    info!("Starting Rust FTP TUI Client");

    let args = Args::parse();

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let app = App::new(
        args.server,
        args.username,
        args.password,
    );

    // Run app
    let res = run_app(&mut terminal, app).await;

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        error!("Application error: {:?}", err);
        eprintln!("\nError: {:?}\n", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
) -> Result<()> {
    // Create channel for async communication
    let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
    app.set_event_sender(event_sender);

    loop {
        // Render UI
        terminal.draw(|f| ui::draw(f, &mut app))?;

        // Handle input events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        if app.can_quit() {
                            break;
                        }
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        break;
                    }
                    _ => {
                        app.handle_event(AppEvent::Key(key));
                    }
                }
            }
        }

        // Handle async events from channel
        while let Ok(event) = event_receiver.try_recv() {
            app.handle_event(event);
        }

        // Process other async events
        app.process_events().await;
    }

    Ok(())
}
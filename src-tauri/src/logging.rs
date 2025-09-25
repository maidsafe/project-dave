use std::path::PathBuf;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use chrono::Local;

pub fn setup_logging() {
    // Get the unique log directory for this run
    let log_dir = get_unique_log_dir();

    // Create the unique log directory if it doesn't exist
    if let Some(ref dir) = log_dir {
        std::fs::create_dir_all(dir).ok();
    }

    // Set up file appender if we have a valid log directory
    let file_layer = log_dir.and_then(|dir| {
        // Create a non-rolling file writer - just write to dave.log in the unique directory
        let log_file_path = dir.join("dave.log");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file_path)
            .ok()?;
        
        let (non_blocking, _guard) = tracing_appender::non_blocking(file);

        // We need to leak the guard to ensure it lives for the entire program duration
        Box::leak(Box::new(_guard));

        Some(
            fmt::layer().with_writer(non_blocking).with_ansi(false), // No ANSI colors in log files
        )
    });

    // Set up console layer
    let console_layer = fmt::layer().with_writer(std::io::stdout).with_ansi(true); // Enable ANSI colors for console

    // Set up the subscriber with environment filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer);

    // Add file layer if available
    if let Some(file_layer) = file_layer {
        subscriber.with(file_layer).init();
    } else {
        subscriber.init();
    }
}

fn get_unique_log_dir() -> Option<PathBuf> {
    let qualifier = "com";
    let organization = "autonomi";
    let application = "dave";

    directories::ProjectDirs::from(qualifier, organization, application).map(|proj_dirs| {
        let mut log_dir = proj_dirs.data_dir().to_owned();
        log_dir.push("logs");
        
        // Create a unique folder name with timestamp
        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        log_dir.push(format!("log_{}", timestamp));
        
        log_dir
    })
}

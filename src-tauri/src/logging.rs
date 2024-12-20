use tracing_subscriber::FmtSubscriber;

pub fn setup_logging() {
    // Create a subscriber that logs to stdout with the log level determined from the RUST_LOG environment variable
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global default subscriber");
}

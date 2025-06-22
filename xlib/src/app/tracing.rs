use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Initialize tracing with environment-based filtering
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Initialize tracing with a specific log level
pub fn init_tracing_with_level(level: &str) {
    tracing_subscriber::registry()
        .with(EnvFilter::new(level))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

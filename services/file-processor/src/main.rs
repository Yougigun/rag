use anyhow::Result;
use std::time::Duration;
use tokio::time;
use tracing::info;
use xlib::app::{graceful_shutdown::shutdown_signal, tracing::init_tracing};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    init_tracing();

    info!("Starting file processor worker...");
    info!("Worker is running and waiting for shutdown signal");

    // Run indefinitely until shutdown signal
    tokio::select! {
        _ = worker_loop() => {
            info!("Worker loop completed");
        }
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
        }
    }

    info!("File processor worker shutting down gracefully");
    Ok(())
}

async fn worker_loop() {
    loop {
        // Worker does nothing but sleep
        time::sleep(Duration::from_secs(10)).await;
        info!("Worker heartbeat - still running...");
    }
}

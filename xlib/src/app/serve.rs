use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::info;

use super::graceful_shutdown::shutdown_signal;

/// Serve an Axum router with graceful shutdown
pub async fn serve_service(
    app: Router,
    addr: SocketAddr,
    service_name: &str,
) -> Result<()> {
    info!("Starting {} on {}", service_name, addr);
    
    let listener = TcpListener::bind(addr).await?;
    
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    info!("{} shutdown complete", service_name);
    Ok(())
} 
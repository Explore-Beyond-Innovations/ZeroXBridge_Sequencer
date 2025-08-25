#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

    tracing_subscriber::registry()
        .with(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into())))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting ZeroXBridge Sequencer");
    // Delegate to shared library entrypoint (expose this in sequencer-lib)
    sequencer_lib::run().await?;
    Ok(())
}

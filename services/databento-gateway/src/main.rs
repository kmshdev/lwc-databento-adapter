use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use databento_gateway::{app, config::GatewayConfig, historical::FakeHistorySource};

#[cfg(feature = "databento-compat")]
use databento_gateway::historical::DatabentoHistoricalSource;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let config = GatewayConfig::from_env()?;
    let source_mode =
        std::env::var("DATABENTO_GATEWAY_SOURCE").unwrap_or_else(|_| "seeded".to_string());
    if source_mode != "seeded" && source_mode != "historical" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "DATABENTO_GATEWAY_SOURCE must be seeded or historical",
        ));
    }
    #[cfg(not(feature = "databento-compat"))]
    if source_mode == "historical" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "DATABENTO_GATEWAY_SOURCE=historical requires the databento-compat feature",
        ));
    }
    #[cfg(feature = "databento-compat")]
    if source_mode == "historical" {
        let key = std::env::var("DATABENTO_API_KEY").map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "DATABENTO_GATEWAY_SOURCE=historical requires DATABENTO_API_KEY",
            )
        })?;
        let source = Arc::new(
            DatabentoHistoricalSource::new(key.clone(), config.allowed_datasets.clone()).map_err(
                |error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string()),
            )?,
        );
        return app::run_with_live_key(source, config, Some(key)).await;
    }

    // Default local-beta mode is deterministic seeded data. The official source
    // is opt-in via DATABENTO_GATEWAY_SOURCE=historical and is never selected
    // merely because a key happens to be present in the environment.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let source = Arc::new(FakeHistorySource::demo(now));
    app::run(source, config).await
}

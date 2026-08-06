use gulfstream::{Config, configured_path, run};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config_path = configured_path()?;
    let config = Config::load(&config_path)?;
    run(config).await
}

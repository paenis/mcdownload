use clap::Parser;
use mcdl::Mcdl;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

fn install_tracing() {
    // MCDL_LOG takes precedence over RUST_LOG
    let env = EnvFilter::try_from_env("MCDL_LOG").unwrap_or_else(|_| {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::INFO.into())
            .from_env_lossy()
    });
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env)
        .init();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    install_tracing();

    Mcdl::parse().run().await?;

    Ok(())
}

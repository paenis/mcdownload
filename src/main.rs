//! Minecraft server manager

#![feature(assert_matches)]
#![deny(rust_2018_idioms)]
#![warn(missing_docs, clippy::all)]

mod cli;
mod macros;
mod minecraft;
mod net;

use std::sync::LazyLock;

use cli::Execute;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

static RT: LazyLock<tokio::runtime::Runtime> = LazyLock::new(|| {
    tracing::trace!("init tokio runtime");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
});

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

fn main() {
    install_tracing();

    cli::parse().execute().unwrap();
}

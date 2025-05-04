//! Minecraft server manager

#![cfg_attr(channel = "nightly", feature(assert_matches))]
#![deny(rust_2018_idioms)]
#![warn(missing_docs, clippy::all)]

mod cli;
mod macros;
mod minecraft;
mod net;

use cli::Execute;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;

fn install_tracing() {
    let env = EnvFilter::try_from_env("MCDL_LOG")
        .unwrap_or_else(|_| EnvFilter::from_default_env())
        .add_directive(LevelFilter::INFO.into());
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(env)
        .init();
}

fn main() {
    install_tracing();

    cli::parse().execute().unwrap();
}

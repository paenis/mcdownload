//! Minecraft server manager

#![deny(rust_2018_idioms)]
#![warn(missing_docs, clippy::all)]

mod command;
mod macros;
mod models;
mod net;

use anyhow::Result;
use clap::{ArgAction, Args, ColorChoice, Parser, Subcommand};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use crate::command::{InfoCmd, InstallCmd, ListCmd, McdlCommand, UninstallCmd};

/// Minecraft server manager
#[derive(Debug, Parser)]
#[command(version, /* long_version = ..., */ about, max_term_width = 100)]
#[doc(hidden)]
pub struct Mcdl {
    /// Global options
    #[clap(flatten)]
    global: GlobalOpts,
    #[clap(subcommand)]
    command: Cmd,
}

impl Mcdl {
    /// Run the application.
    pub async fn run() -> Result<()> {
        let start = std::time::Instant::now();
        let app = Self::parse();
        app.install_tracing();
        tracing::debug!("parsed command line arguments: {app:?}");

        app.execute().await?;

        tracing::info!("ran in {:.2?}", start.elapsed());
        Ok(())
    }

    fn install_tracing(&self) {
        // verbose flag should override env
        // MCDL_LOG takes precedence over RUST_LOG

        // TODO better env filter setup.. default to "mcdl=warn,off" or something
        let env = EnvFilter::try_from_env("MCDL_LOG").unwrap_or_else(|_| {
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy()
        });
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .compact()
            .without_time()
            .with_filter(env);

        // boxing kinda sucks but its easy /shrug
        #[cfg_attr(not(feature = "console"), expect(unused_mut))]
        let mut layers = vec![fmt_layer.boxed()];

        #[cfg(feature = "console")]
        {
            let console_layer = console_subscriber::ConsoleLayer::builder()
                .with_default_env()
                .spawn();
            layers.push(console_layer.boxed());
        }

        tracing_subscriber::registry().with(layers).init();
    }
}

#[derive(Debug, Args)]
struct GlobalOpts {
    /// Color
    #[arg(long, value_enum, global = true, default_value_t = ColorChoice::Auto)]
    color: ColorChoice,

    /// Verbosity level (can be set multiple times)
    #[arg(long, short, global = true, action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Show information about a Minecraft version.
    Info(InfoCmd),
    /// Install an instance of a Minecraft server.
    Install(InstallCmd),
    /// List installed or available Minecraft versions.
    List(ListCmd),
    /// Uninstall a Minecraft server instance.
    Uninstall(UninstallCmd),
}

impl McdlCommand for Mcdl {
    async fn execute(&self) -> anyhow::Result<()> {
        tracing::debug!("executing command: {:?}", self.command);
        match &self.command {
            Cmd::Info(cmd) => cmd.execute().await,
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    #[test]
    fn verify_app() {
        Mcdl::command().debug_assert();
    }
}

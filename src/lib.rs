//! Minecraft server manager

#![deny(rust_2018_idioms)]
#![warn(missing_docs, clippy::all)]

mod command;
mod identifiers;
mod macros;
mod metadata;
mod net;

use clap::{ArgAction, Args, ColorChoice, Parser, Subcommand};
use color_eyre::Result;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use crate::command::{InfoCmd, InstallCmd, ListCmd, McdlCommand, UninstallCmd};

/// Minecraft server manager
#[derive(Debug, Parser)]
#[command(version, long_version = "foo", about, max_term_width = 100)]
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

        color_eyre::install()?;
        app.install_tracing();

        tracing::trace!(?app, "parsed command line arguments");

        app.execute().await?;

        tracing::info!("ran in {:.2?}", start.elapsed());
        Ok(())
    }

    fn filter(&self) -> EnvFilter {
        let verbose = match self.global.verbose {
            0 => None,
            1 => Some("warn"),
            2 => Some("info"),
            3 => Some("debug"),
            _ => Some("trace"),
        };

        if let Some(level) = verbose {
            EnvFilter::builder().parse_lossy(level)
        } else {
            EnvFilter::try_from_env("MCDL_LOG").unwrap_or_else(|_| EnvFilter::from_default_env())
        }
    }

    fn should_colorize(&self, stream: supports_color::Stream) -> bool {
        match self.global.color {
            ColorChoice::Always => true,
            ColorChoice::Auto => supports_color::on_cached(stream).is_some_and(|l| l.has_basic),
            ColorChoice::Never => false,
        }
    }

    fn install_tracing(&self) {
        let env = self.filter();
        let fmt_layer = tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .compact()
            .without_time()
            .with_ansi(self.should_colorize(supports_color::Stream::Stderr))
            .with_filter(env);

        let error_layer = tracing_error::ErrorLayer::default();

        // boxing kinda sucks but its easy /shrug
        #[cfg_attr(not(feature = "console"), expect(unused_mut))]
        let mut layers = vec![fmt_layer.boxed(), error_layer.boxed()];

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
    #[arg(long, value_enum, global = true, default_value_t, value_name = "WHEN")]
    color: ColorChoice,

    /// Verbosity level (can be set multiple times)
    ///
    /// If set, overrides any directives set via the MCDL_LOG or RUST_LOG environment variables
    #[arg(long, short, global = true, action = ArgAction::Count)]
    verbose: u8,
}

#[derive(Debug, Subcommand)]
#[command(infer_subcommands = true)]
enum Cmd {
    /// Show information about a Minecraft version
    #[command(visible_alias = "show")]
    Info(InfoCmd),
    /// Install an instance of a Minecraft server
    Install(InstallCmd),
    /// List installed or available Minecraft versions
    List(ListCmd),
    /// Uninstall a Minecraft server instance
    Uninstall(UninstallCmd),
}

impl McdlCommand for Mcdl {
    #[tracing::instrument]
    async fn execute(&self) -> color_eyre::Result<()> {
        tracing::debug!("executing command: {:?}", self.command);
        match &self.command {
            Cmd::Info(info) => info.execute().await,
            Cmd::Install(install) => install.execute().await,
            Cmd::List(list) => list.execute().await,
            Cmd::Uninstall(uninstall) => uninstall.execute().await,
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

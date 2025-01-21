mod parse;

use crate::cli::parse::Cli;

pub fn parse() -> Result<Cli, clap::Error> {
    use clap::Parser;
    Cli::try_parse()
}

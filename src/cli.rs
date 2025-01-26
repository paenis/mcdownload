mod parse;

use crate::cli::parse::{options, Options};

pub fn parse() -> Options {
    options().run()
}

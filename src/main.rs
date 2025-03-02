//! Minecraft server manager.

#![cfg_attr(channel = "nightly", feature(assert_matches))]
#![deny(rust_2018_idioms)]
#![warn(missing_docs, clippy::all)]

mod cli;
mod macros;
mod minecraft;
mod net;

use cli::Execute;

fn main() {
    cli::parse().execute().unwrap();
}

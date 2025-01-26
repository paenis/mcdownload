#![cfg_attr(channel = "nightly", feature(assert_matches))]

mod cli;
mod macros;
mod minecraft;

fn main() {
    let args = cli::parse();

    println!("args: {args:#?}");
}

#[derive(clap::Parser, Debug)]
pub struct Cli {
    #[clap(subcommand)]
    sub: Option<Subcommand>,
}

#[derive(clap::Subcommand, Debug)]
enum Subcommand {
    Foo,
}

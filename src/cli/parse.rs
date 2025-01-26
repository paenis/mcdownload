use bpaf::*;

use crate::minecraft::VersionNumber;

#[derive(Debug)]
pub struct Options {
    action: Cmd,
}

#[derive(Debug)]
enum Cmd {
    Install { instances: Vec<String> },
    Foo,
}

fn install() -> impl Parser<Cmd> {
    // NOTE: see https://docs.rs/bpaf/latest/bpaf/_documentation/_3_cookbook/_05_struct_groups/index.html for adding associated name for each instance
    let instances = short('v')
        .help("Version to install")
        .argument::<String>("VERSION")
        .many()
        .fallback_with(|| VersionNumber::latest().map(|v| vec![v.to_string()]))
        // .debug_fallback();
        ;
    construct!(Cmd::Install { instances })
}

fn cmd() -> impl Parser<Cmd> {
    let install = install()
        .to_options()
        .descr("Install versions")
        .command("install");

    let foo = positional::<String>("FOO")
        .optional()
        .hide()
        .map(|_| Cmd::Foo)
        .to_options()
        .descr("Foo")
        .command("foo");

    construct!([install, foo]).group_help("subcommands")
}

pub fn options() -> OptionParser<Options> {
    // figure out how to show program version info with short flag
    let action = cmd();
    construct!(Options { action }).to_options()
}

// TODO: unit tests!!!

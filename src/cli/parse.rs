use bpaf::*;

use crate::minecraft::{api, VersionNumber};

#[derive(Debug, Clone)]
pub enum Options {
    /// Show version
    ShowVersion,
    /// Subcommand
    Cmd(Cmd),
}

#[derive(Debug, Clone)]
enum Cmd {
    Install { instances: Vec<VersionNumber> },
    List { filter: ListFilter },
}

/// Filter the list of versions.
#[derive(Debug, Clone)]
struct ListFilter {
    /// If true, only include installed versions. This filter is _inclusive_.
    installed: bool,
    /// If the corresponding element is true, include release, pre-release, snapshot, and non-standard versions.
    ///
    /// At least one must be true. This filter is _exclusive_.
    included_types: (bool, bool, bool, bool),
}

fn install() -> impl Parser<Cmd> {
    // NOTE: see https://docs.rs/bpaf/latest/bpaf/_documentation/_3_cookbook/_05_struct_groups/index.html for adding associated name for each instance
    let instances = short('v')
        .help("Version(s) to install. If not specified, the latest release will be used.")
        .argument::<VersionNumber>("VERSION")
        .guard(|v| api::find_version(v).is_some(), "version not found")
        .some("must specify at least one version")
        .fallback_with(|| api::get_manifest().map(|m| vec![m.latest_release_id().to_owned()]));
    construct!(Cmd::Install { instances })
}

fn list() -> impl Parser<Cmd> {
    macro_rules! multi_flag {
        [$($name:ident $short:literal $long:literal),*] => {{
            construct!($(
                $name(short($short).help(concat!("Include ", $long, " versions")).switch()),
            )*)
        }}
    }

    let all = short('a')
        .help("Include all versions")
        .req_flag((true, true, true, true));

    let types = multi_flag![
        r 'r' "release",
        p 'p' "pre-release",
        s 's' "snapshot",
        n 'n' "non-standard"
    ]
    // this looks ugly
    .map(|(r, p, s, n)| {
        if r || p || s || n {
            (r, p, s, n)
        } else {
            (true, false, false, false)
        }
    });

    let included_types = construct!([all, types]).group_help("Version type filters");

    let installed = short('i')
        .help("Include only installed versions matching the type filters")
        .switch();

    let filter = construct!(ListFilter {
        installed,
        included_types,
    });

    construct!(Cmd::List { filter })
}

fn cmd() -> impl Parser<Cmd> {
    let install = install()
        .to_options()
        .descr("Install versions")
        .command("install");

    let list = list().to_options().descr("List versions").command("list");

    construct!([install, list]).group_help("subcommands")
}

pub fn options() -> OptionParser<Options> {
    let show_version = short('V')
        .help("Show version")
        .req_flag(Options::ShowVersion);

    let cmd = construct!(Options::Cmd(cmd()));

    construct!([show_version, cmd]).to_options()
}

// TODO: unit tests!!!

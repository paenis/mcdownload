use bpaf::{OptionParser, Parser, construct, short};

use super::{Cmd, ListFilter, Options};
use crate::minecraft::{VersionNumber, api};

fn version_exists(id: &VersionNumber) -> bool {
    api::find_version(id).is_some()
}

fn install() -> impl Parser<Cmd> {
    // NOTE: see https://docs.rs/bpaf/latest/bpaf/_documentation/_3_cookbook/_05_struct_groups/index.html for adding associated name for each instance
    let versions = short('v')
        .help("Version(s) to install")
        .argument::<VersionNumber>("VERSION")
        .guard(version_exists, "version not found")
        .some("must specify at least one version")
        .fallback_with(|| api::get_manifest().map(|m| vec![m.latest_release_id().to_owned()]));
    construct!(Cmd::Install { versions })
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
        if [r, p, s, n].into_iter().any(|b| b) {
            (r, p, s, n)
        } else {
            (true, false, false, false)
        }
    });

    let included_types = construct!([all, types]).group_help("Version type filters");

    let installed = short('i')
        .help("List installed versions instead of available versions")
        .switch();

    let filter = construct!(ListFilter {
        installed,
        included_types,
    });

    construct!(Cmd::List { filter })
}

fn info() -> impl Parser<Cmd> {
    let v = short('v')
        .argument::<VersionNumber>("VERSION")
        .guard(version_exists, "version not found");
    construct!(Cmd::Info { v })
}

fn cmd() -> impl Parser<Cmd> {
    let install = install()
        .to_options()
        .descr("Install versions")
        .header("If no versions are specified, the latest release version will be used.")
        .command("install");

    let list = list()
        .to_options()
        .descr("List versions matching the specified filters")
        .header("If no filters are specified, only release versions will be shown.")
        .command("list");

    let info = info()
        .to_options()
        .descr("Get information about a version")
        .command("info");

    construct!([install, list, info]).group_help("subcommands")
}

pub fn options() -> OptionParser<Options> {
    let show_version = short('V')
        .long("version")
        .help("Print version")
        .req_flag(Options::ShowVersion);

    let cmd = construct!(Options::Cmd(cmd()));

    construct!([show_version, cmd])
        .to_options()
        .descr("Minecraft server manager")
}

#[cfg(test)]
mod tests {
    use bpaf::ParseFailure;

    use super::*;

    #[test]
    fn bpaf_invariants() {
        options().check_invariants(false);
    }

    #[test]
    fn show_version() {
        let ver = options().run_inner(&["-V"]).unwrap();
        crate::macros::assert_matches!(ver, Options::ShowVersion);
    }

    #[test]
    fn install_default() {
        let install = options().run_inner(&["install"]).unwrap();
        crate::macros::assert_matches!(install, Options::Cmd(Cmd::Install { versions: _ }));

        let Options::Cmd(Cmd::Install {
            versions: instances,
        }) = install
        else {
            unreachable!()
        };

        assert_eq!(
            instances,
            vec![api::get_manifest().unwrap().latest_release_id().to_owned()]
        );
    }

    #[test]
    fn install_invalid() {
        macro_rules! assert_err {
            ($input:expr) => {
                crate::macros::assert_matches!(
                    options().run_inner($input),
                    Err(ParseFailure::Stderr(_))
                )
            };
        }

        assert_err!(&["install", "-v"]);
        assert_err!(&["install", "-v", "1.19."]);
        assert_err!(&["install", "-v "]);
        assert_err!(&["install", "-v-v", "1.19.4"]);
        assert_err!(&["install", "-v1.19.4", "1.19.4"]);
        assert_err!(&["install", "-v", "1.19.4", "1.19.4"]);
        assert_err!(&["install", "-v", "foobar"]);
        assert_err!(&["install", "-v", "1.19.4", "-v"]);
    }
}

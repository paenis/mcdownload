use rustc_version::{version_meta, Channel};

fn main() {
    println!(r#"cargo:rustc-check-cfg=cfg(channel, values("stable", "beta", "nightly", "dev"))"#);

    // Set cfg flags depending on release channel
    let channel = match version_meta().unwrap().channel {
        Channel::Stable => "stable",
        Channel::Beta => "beta",
        Channel::Nightly => "nightly",
        Channel::Dev => "dev",
    };
    println!("cargo:rustc-cfg=channel=\"{channel}\"");
}

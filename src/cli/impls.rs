use super::ListFilter;
use crate::macros::debug_unreachable;
use crate::minecraft::{VersionNumber, api};

pub(crate) fn install(versions: &[VersionNumber]) -> anyhow::Result<()> {
    tracing::debug!(?versions, "install");
    todo!()
}

pub(crate) fn list(filter: &ListFilter) -> anyhow::Result<()> {
    tracing::debug!(?filter, "list");
    api::get_manifest()?
        .versions
        .iter()
        .filter(|v| filter.includes(&v.id))
        .rev() // newest at bottom. should probably use Ord instead
        .for_each(|v| println!("{}", v.id));
    todo!()
}

pub(crate) fn info(v: &VersionNumber) -> anyhow::Result<()> {
    tracing::debug!(?v, "info");

    let package = api::find_version(v)
        .ok_or_else(|| debug_unreachable!() /* checked by parser */)?
        .get_package()?;

    println!("{package:#?}");
    todo!()
}

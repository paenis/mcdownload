mod info;
mod install;
mod list;

pub use info::InfoCmd;
pub use install::InstallCmd;
pub use list::ListCmd;

pub trait McdlCommand {
    // TODO: color-eyre or miette
    async fn execute(&self) -> anyhow::Result<()>;
}

use mcdl::Mcdl;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Mcdl::run().await?;
    Ok(())
}

use mcdl::Mcdl;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    Mcdl::run().await?;
    Ok(())
}

use vergen::EmitBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    EmitBuilder::builder()
        .git_sha(true)
        .git_branch()
        .cargo_opt_level()
        .emit()?;
    Ok(())
}

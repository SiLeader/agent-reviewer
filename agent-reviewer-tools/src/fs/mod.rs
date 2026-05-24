mod list_files;
mod read_file;

use anyhow::Context;
pub use list_files::*;
pub use read_file::*;

fn check_path_location(path: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()
        .context("failed to get current directory")?
        .canonicalize()?;
    let path = std::path::Path::new(path).canonicalize()?;
    if !path.starts_with(cwd) {
        anyhow::bail!("access to path outside of current directory is not allowed");
    }
    Ok(())
}

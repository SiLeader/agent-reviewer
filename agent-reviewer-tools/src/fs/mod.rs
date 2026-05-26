// Copyright 2026- SiLeader (Cerussite).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

mod ignore;
mod list_files;
mod read_file;
mod search;

use anyhow::Context;
pub use list_files::*;
pub use read_file::*;
pub use search::*;

fn check_path_location(path: &str) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()
        .context("failed to get current directory")?
        .canonicalize()?;
    let path = std::path::Path::new(path);
    if !path.exists() {
        return Ok(());
    }
    let path = path.canonicalize()?;
    if !path.starts_with(cwd) {
        anyhow::bail!("access to path outside of current directory is not allowed");
    }
    Ok(())
}

#[cfg(test)]
fn write_file(path: &std::path::Path, content: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

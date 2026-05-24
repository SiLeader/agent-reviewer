use crate::git::remotes::GitRemote;

pub struct GitHub {
    base_url: String,
}

impl Default for GitHub {
    fn default() -> Self {
        Self {
            base_url: "https://api.github.com".to_string(),
        }
    }
}

impl GitRemote for GitHub {
    fn get_default_branch(&self) -> anyhow::Result<String> {
        let res = std::process::Command::new("gh")
            .args([
                "repo",
                "view",
                "--json",
                "defaultBranchRef",
                "--jq",
                ".defaultBranchRef.name",
            ])
            .output()?;
        if !res.status.success() {
            anyhow::bail!(
                "Failed to get default branch: {}",
                String::from_utf8_lossy(&res.stderr)
            );
        }
        let branch_name = String::from_utf8_lossy(&res.stdout);
        Ok(branch_name.trim().to_string())
    }

    fn get_pull_request_base_branch(&self) -> anyhow::Result<String> {
        let res = std::process::Command::new("gh")
            .args([
                "pr",
                "view",
                "--json",
                "baseRefName",
                "--jq",
                ".baseRefName",
            ])
            .output()?;
        if !res.status.success() {
            anyhow::bail!(
                "Failed to get base branch of current pull request: {}",
                String::from_utf8_lossy(&res.stderr)
            );
        }
        let branch_name = String::from_utf8_lossy(&res.stdout);
        Ok(branch_name.trim().to_string())
    }
}

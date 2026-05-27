use crate::config::{GitHubConfig, GitRemoteConfig};
use octocrab::Octocrab;
use tracing::warn;

pub(crate) fn setup_git_remotes(git_remote: &GitRemoteConfig) -> anyhow::Result<()> {
    if let Some(github_config) = &git_remote.github {
        setup_github(github_config)?;
    }
    Ok(())
}

fn setup_github(github: &GitHubConfig) -> anyhow::Result<()> {
    match &github.token_env {
        None => {
            if let Err(e) = setup_github_token("GITHUB_TOKEN") {
                warn!(
                    "Failed to setup github token from GITHUB_TOKEN env var: {}",
                    e
                );
            }
            Ok(())
        }
        Some(env) => setup_github_token(env),
    }
}

fn setup_github_token(token_env: &str) -> anyhow::Result<()> {
    if let Ok(token) = std::env::var(token_env) {
        octocrab::initialise(Octocrab::builder().personal_token(token).build()?);
        Ok(())
    } else {
        anyhow::bail!("Failed to setup github token");
    }
}

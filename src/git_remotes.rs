use crate::config::{GitHubConfig, GitRemoteConfig};
use agent_reviewer_tools::git::GitHub;
use tracing::warn;

pub(crate) async fn setup_git_remotes(git_remote: &GitRemoteConfig) -> anyhow::Result<()> {
    if let Some(github_config) = &git_remote.github {
        setup_github(github_config).await
    } else {
        anyhow::bail!(
            "Currently, only GitHub is supported as a git remote. Please set github.token_env in agent-reviewer.toml or set GITHUB_TOKEN env var."
        );
    }
}

async fn setup_github(github: &GitHubConfig) -> anyhow::Result<()> {
    match &github.token_env {
        None => {
            if let Err(e) = setup_github_token("GITHUB_TOKEN").await {
                warn!(
                    "Failed to setup github token from GITHUB_TOKEN env var: {}",
                    e
                );
            }
            Ok(())
        }
        Some(env) => setup_github_token(env).await,
    }
}

async fn setup_github_token(token_env: &str) -> anyhow::Result<()> {
    if let Ok(token) = std::env::var(token_env) {
        GitHub.set_token(&token).await;
        Ok(())
    } else {
        anyhow::bail!("Failed to setup github token");
    }
}

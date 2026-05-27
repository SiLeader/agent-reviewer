use crate::config::{GitHubConfig, GitRemoteConfig};
use agent_reviewer_tools::git::GitHub;

pub(crate) async fn setup_git_remotes(git_remote: &GitRemoteConfig) -> anyhow::Result<()> {
    if let Some(github_config) = &git_remote.github {
        setup_github(github_config).await?;
    } else {
        if let Err(e) = setup_github_token(Some("GITHUB_TOKEN")).await {
            tracing::warn!(
                "Failed to setup github token, cannot use github related tools: {}",
                e
            );
        }
    }
    Ok(())
}

async fn setup_github(github: &GitHubConfig) -> anyhow::Result<()> {
    setup_github_token(github.token_env.as_deref()).await
}

async fn setup_github_token(token_env: Option<&str>) -> anyhow::Result<()> {
    let token = std::env::var(token_env.unwrap_or("GITHUB_TOKEN"))?;
    GitHub.set_token(&token).await;
    Ok(())
}

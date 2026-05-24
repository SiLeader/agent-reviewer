pub(super) mod github;

pub(super) trait GitRemote {
    fn get_default_branch(&self) -> anyhow::Result<String>;
    fn get_pull_request_base_branch(&self) -> anyhow::Result<String>;
}

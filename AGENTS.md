# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 Cargo workspace. The root crate, `agent-reviewer`, builds the CLI from `src/`. Orchestration lives in `src/orchestrator/`, prompt loading in `src/prompt.rs`, configuration parsing in `src/config.rs`, and default prompt templates in `src/default_prompts/`.

Workspace library crates are split by responsibility:

- `agent-reviewer-agent/`: ReAct runtime, builders, concurrency, and subagent wrappers.
- `agent-reviewer-tools/`: filesystem, git, remote, and compound tool implementations.
- `agent-reviewer-model-provider/`: provider configuration and `genai` model-provider setup.

Tests are colocated with implementation files in inline `#[cfg(test)] mod tests` blocks; there is no top-level `tests/` directory.

## Build, Test, and Development Commands

- `cargo build`: build the workspace.
- `cargo build --release`: build an optimized binary.
- `cargo test --workspace`: run all unit tests.
- `cargo test -p <crate> <test_name>`: run one focused test, for example `cargo test -p agent-reviewer renders_default_system_prompts`.
- `cargo clippy --workspace --all-targets`: run Rust lints across all targets.
- `cargo fmt`: format all Rust code.
- `cargo run -- [--config agent-reviewer.toml] [--output FILE] [PROMPT]`: run locally. Use `RUST_LOG=info` for tracing.

## Coding Style & Naming Conventions

Use standard `rustfmt` formatting and Rust 2024 idioms. Keep module boundaries aligned with the crate split: orchestration in the root crate, reusable tools in `agent-reviewer-tools`, agent loop behavior in `agent-reviewer-agent`, and provider wiring in `agent-reviewer-model-provider`.

Use `snake_case` for functions, modules, and variables; `PascalCase` for types and traits; and `SCREAMING_SNAKE_CASE` for constants.

## Testing Guidelines

Add focused unit tests beside the code they exercise using `#[cfg(test)] mod tests`. Name tests by behavior, such as `renders_default_system_prompts` or `parse_review_marker_arguments`. For prompt rendering, config parsing, marker tools, git tools, or orchestration data shapes, cover success and error paths where practical.

Run `cargo test --workspace` before submitting broad changes. Use package-scoped test runs while iterating.

## Commit & Pull Request Guidelines

Recent commits use imperative, sentence-case subjects without prefixes, for example `Enhance review templates with additional instructions`. Follow that style and keep subjects concise.

Pull requests should include a short summary, the commands run for verification, linked issues when applicable, and sample output or screenshots only when user-visible behavior changes. Call out config, prompt, or environment-variable changes explicitly.

## Security & Configuration Tips

The default config path is `agent-reviewer.toml`. The CLI expects git context and provider credentials through environment variables such as `GITHUB_TOKEN`, depending on configured providers. Do not commit secrets, local tokens, or provider-specific credentials.

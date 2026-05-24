# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

Cargo workspace, Rust edition 2024. The root crate `agent-reviewer` produces the binary; three member crates (`agent-reviewer-agent`, `agent-reviewer-model-provider`, `agent-reviewer-tools`) provide library code.

- Build everything: `cargo build` (workspace) — `cargo build --release` for optimized binary.
- Run all tests: `cargo test --workspace`.
- Run a single test: `cargo test -p <crate> <test_name>` (e.g. `cargo test -p agent-reviewer renders_default_system_prompts`). Tests live as inline `#[cfg(test)] mod tests` blocks; there is no top-level `tests/` directory.
- Lint / format: `cargo clippy --workspace --all-targets` and `cargo fmt`.
- Run the reviewer: `cargo run -- [--config agent-reviewer.toml] [--output FILE] [PROMPT]`. Tracing output is controlled by `RUST_LOG` (e.g. `RUST_LOG=info`).

The default config path is `agent-reviewer.toml` in the current working directory. The binary expects git context (uses `git2`) and a `GITHUB_TOKEN` (or other key env vars depending on configured providers) to talk to LLMs via `genai`.

## Architecture

The system is a **configuration-driven, multi-step code review pipeline** built on a ReAct-style agent loop. Understanding the following concepts is required to make non-trivial changes.

### Three-phase orchestration (`src/orchestrator/mod.rs`)

`Orchestrator::run` executes a fixed sequence:

1. **Triage** — one agent inspects the diff and emits a list of `ReviewUnit`s (each with a task, focus files, and a `ReviewModel` of `Light`/`Standard`/`Power`).
2. **Review (fan-out)** — `join_all` runs one review agent per unit *in parallel*. The unit's `model` field selects which of the configured `review_light_agent` / `review_standard_agent` / `review_power_agent` is built.
3. **Finalize** — a single agent consumes all `SubmitReviewArgs` results and produces the final review markdown.

The data shape passed between phases is defined by the `SubmitTriage` / `SubmitReview` / `SubmitReviewResult` *marker tools* in `src/orchestrator/submit_marker/`.

### Marker tools end the ReAct loop

`agent-reviewer-tools` distinguishes two tool kinds:

- `AgentTool` — has an async `run` returning a string back to the model.
- `MarkerAgentTool` — *schema only, no execution*. When the model calls one, `CompoundAgentTools::run_all` returns `ToolCallResponse::MarkerFound`, and `ReActAgent::run` returns the marker's `fn_arguments` as the agent's final value (other non-marker tool calls in the same turn are still executed but their results discarded).

This is how each orchestration phase obtains a structured result: the system prompt instructs the model to call e.g. `submit_triage`, and the JSON arguments deserialize into `SubmitTriageArgs`. When adding a new phase or changing the output shape of an existing one, you change the marker tool's `JsonSchema` struct — not parsing code.

### Tool schemas are generated from Rust types

`agent_reviewer_tools::tool_description::<T>(name, description)` uses `schemars` (OpenAPI 3 settings, `inline_subschemas = true`) to derive the tool's JSON schema from `T: JsonSchema`. To change a tool's parameters, edit the `#[derive(JsonSchema)]` struct — do not hand-write schemas.

### Configuration layering: providers → models → agents → steps

`agent-reviewer.toml` (parsed by `src/config.rs`) has four indirection layers, in order:

1. `[[model_providers]]` — provider type + credentials env var (`OpenAI` / `Anthropic` / `GitHub` / `Bedrock`, see `agent-reviewer-model-provider/src/config.rs`).
2. `[[models]]` — names a `genai` model string and binds it to a provider id.
3. `[[agents]]` — binds a model id to ReAct knobs (`effort`, `max_tokens`, `temperature`, `top_p`, `max_loops`).
4. `[steps]` and `[subagent.*]` — bind an agent id to each pipeline role (`triage_agent`, `review_{light,standard,power}_agent`, `finalize_agent`, `subagent.explorer.agent`).

`Orchestrator::build_agent(id, …)` is the single place this all comes together; it builds a fresh `ReActAgent` per phase by cloning the shared `ReActAgentBuilder` and overriding model/options/tools.

### Subagents are agents wrapped as tools

`agent-reviewer-agent/src/tools/subagent/explorer.rs` shows the pattern: `Explorer` wraps a `ReActAgent` and implements `AgentTool`. When the outer agent calls `explorer`, the wrapper runs the inner ReAct loop (with its own marker tool `submit`) and returns the marker payload as a string. Add new subagents by following this shape and wiring a `subagent.<name>` config block.

### Concurrency limiter gates LLM calls

`ConcurrencyLimiter` (a `tokio::sync::Semaphore` wrapper) is acquired around every `client.exec_chat` call in `ReActAgent::run`. A single limiter is created in `main.rs` from `config.concurrency` and shared by all agents — including parallel review-phase agents and subagent calls — so it caps *total in-flight model requests across the whole pipeline*, not per-agent parallelism.

### Prompt loading

`PromptManager` in `src/prompt.rs` holds three system strings + three Jinja user templates (`triage`, `review`, `finalize`). Defaults are compiled in via `include_str!` from `src/default_prompts/{triage,review,finalize}/`; the `[prompt.*]` TOML section can override any of them by file path. User templates receive `instructions` (loaded by `src/instruction.rs`), plus `prompt` / `unit` / `reviews` depending on phase.

`load_instructions` searches a fixed file list in order: `AGENT_REVIEWER.md`, `AGENTS.md`, `.github/copilot-instructions.md`, `GEMINI.md`, `CLAUDE.md`. The first one that reads successfully wins — so if you add review-relevant guidance to `CLAUDE.md` it will be **ignored** when any of the earlier files exist (currently `AGENT_REVIEWER.md` does). Edit `AGENT_REVIEWER.md` instead, or change the search order in `src/instruction.rs`.

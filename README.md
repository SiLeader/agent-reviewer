# Agent Reviewer

Agent Reviewer is an LLM-driven code review pipeline written in Rust. It inspects the current Git working tree, splits
the diff into focused review units, runs review agents in parallel, and consolidates the findings into a single Markdown
report.

## Features

- **Three-phase pipeline** — triage, parallel review, and finalize, orchestrated as a ReAct-style agent loop.
- **Configuration-driven** — model providers, models, agents, and per-phase bindings are defined in a single TOML file.
- **Multiple providers** — OpenAI, Anthropic, GitHub Models, and Amazon Bedrock via [
  `genai`](https://crates.io/crates/genai).
- **Tiered review models** — each review unit picks a `Light`, `Standard`, or `Power` agent so simple changes use
  cheaper models and risky ones use the most capable.
- **Built-in tools** — filesystem listing/reading/search and Git diff/branch inspection are exposed to agents out of the
  box.
- **Explorer and advisor subagents** — wrapped ReAct agents that reviewers can call as tools for cross-file exploration
  and focused implementation advice.
- **Normal and security review modes** — the default profile runs a general code review; `--security-review` switches to
  security-focused prompts and result schemas.
- **Customizable prompts** — sensible normal and security defaults are embedded in the binary; any phase's system or
  user template can be overridden by file path.
- **Concurrency limiter** — a single semaphore caps total in-flight LLM requests across all phases and subagents.

## Installation

Build from source with a recent stable Rust toolchain (edition 2024):

```bash
git clone <repository-url>
cd agent-reviewer
cargo build --release
```

The resulting binary is `target/release/agent-reviewer`.

## Quick start

1. Place an `agent-reviewer.toml` in the working directory (see [Configuration](#configuration)).
2. Export the credentials required by the configured providers, for example `GITHUB_TOKEN` for the GitHub provider.
3. Run the reviewer from inside a Git repository:

```bash
RUST_LOG=info cargo run --release -- --output review.md
```

The command inspects the current Git context, runs the pipeline, and writes the final Markdown review to `review.md` (or
stdout if `--output` is omitted).
Add `--security-review` to run the security-focused prompt profile and final report schema.

## CLI

```
agent-reviewer [OPTIONS] [PROMPT]
```

| Option                                    | Description                                                                       |
|-------------------------------------------|-----------------------------------------------------------------------------------|
| `-c`, `--config <PATH>`                   | Path to the TOML config file. Defaults to `agent-reviewer.toml`.                  |
| `-o`, `--output <FILE>`                   | File to write the final review to. Prints to stdout when omitted.                 |
| `-a`, `--allow-output-fallback-to-stdout` | If writing to `--output` fails, print to stdout instead of exiting with an error. |
| `-s`, `--security-review`                 | Run the security review profile instead of the normal code review profile.        |
| `-i`, `--id <ID>`                         | Use for saving review session steps.                                              |
| `[PROMPT]`                                | Optional free-form instruction passed to the triage step.                         |

Logging verbosity is controlled by `RUST_LOG` (e.g. `RUST_LOG=info`, `RUST_LOG=debug`).

## Configuration

The config file has four indirection layers — providers, models, agents, and step bindings — plus optional prompt
overrides.

```toml
concurrency = 4

# 1. Provider credentials
[[model_providers]]
id = "github"
type = "GitHub"
# key_env = "GITHUB_TOKEN"   # optional override

# 2. Model definitions bound to a provider
[[models]]
id = "gpt-5-mini"
model = "openai/gpt-5-mini"
provider = "github"

[[models]]
id = "gpt-4.1"
model = "openai/gpt-4.1"
provider = "github"

# 3. Agents bind a model to ReAct knobs
[[agents]]
id = "light"
model = "gpt-5-mini"

[[agents]]
id = "standard"
model = "gpt-5-mini"
effort = "medium"

[[agents]]
id = "power"
model = "gpt-4.1"
effort = "high"
# max_tokens = 8192
# max_loops = 20

# 4. Pipeline step -> agent bindings
[steps.triage]
agent = "light"

[steps.review.light]
main_agent = "light"
advisor_agent = "standard"

[steps.review.standard]
main_agent = "standard"
advisor_agent = "power"

[steps.review.power]
main_agent = "power"

[steps.finalize]
agent = "power"

[subagent.explorer]
agent = "light"

# Optional: override built-in prompts
# [prompt.triage]
# system_file        = "prompts/triage.system.md"
# user_template_file = "prompts/triage.user.j2"
```

### Supported provider types

| `type`      | Required fields | Notes                                                |
|-------------|-----------------|------------------------------------------------------|
| `OpenAI`    | `key_env`       | Optional `base_url` for OpenAI-compatible endpoints. |
| `Anthropic` | `key_env`       | Optional `base_url`.                                 |
| `GitHub`    | –               | `key_env` defaults to `GITHUB_TOKEN`.                |
| `Bedrock`   | `region`        | Optional `access_key_env` / `secret_access_key_env`. |

### Reasoning effort

Agents accept `effort = "none" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max"` or a token budget via
`{ Budget = N }`, depending on what the chosen model supports.

### Review instructions

The triage and review phases load repository-specific guidance from the first file that exists in this order:

1. `AGENT_REVIEWER.md`
2. `AGENTS.md`
3. `.github/copilot-instructions.md`
4. `GEMINI.md`
5. `CLAUDE.md`

Only the first match is used. Add review-relevant guidance to whichever file already takes precedence in your
repository (or change the search order in `src/instruction.rs`).

## How it works

1. **Triage** — a single agent inspects the diff and emits a list of review units. Each unit carries a task description,
   focus files, and a desired review tier (`Light` / `Standard` / `Power`).
2. **Review** — all units are dispatched to their tier's `main_agent` in parallel via `futures::future::join_all`.
   Agents may call the explorer subagent, an optional tier-specific advisor subagent, and the built-in filesystem/Git
   tools to gather additional context.
3. **Finalize** — a single agent consumes every unit's structured result and produces the final Markdown report.

When `--security-review` is set, the same three-phase pipeline runs with the embedded security triage, review, and
finalize prompts. Review agents return security-specific structured fields such as overall risk, exploitability, impact,
recommendations, assumptions, and unanswered security questions before final synthesis.

Each phase ends when the model calls a *marker tool* (`submit_triage`, `submit_review`, `submit_review_result`). Marker
tools have a schema but no execution body; their JSON arguments become the phase's return value. To change the output
shape of a phase, edit the `#[derive(JsonSchema)]` struct that backs the marker — schemas are generated from Rust types.

A shared `ConcurrencyLimiter` (a `tokio::sync::Semaphore`) wraps every model call, so `concurrency` caps total in-flight
requests across the whole pipeline rather than per-agent parallelism.

## Workspace layout

```text
.
├── src/                            # Root binary crate (`agent-reviewer`)
│   ├── main.rs                     # CLI entrypoint
│   ├── config.rs                   # TOML config schema
│   ├── instruction.rs              # Review instruction loader
│   ├── prompt.rs                   # Prompt manager + Jinja rendering
│   ├── orchestrator/               # Triage/review/finalize workflow
│   └── default_prompts/            # Built-in prompt templates
├── agent-reviewer-agent/           # ReAct-style agent runtime
├── agent-reviewer-tools/           # Filesystem and Git tool implementations
├── agent-reviewer-model-provider/  # genai provider/model wiring
└── agent-reviewer.toml             # Default config path
```

## Development

```bash
cargo build                                  # build the workspace
cargo test --workspace                       # run all unit tests
cargo test -p agent-reviewer <test_name>     # run a single test
cargo clippy --workspace --all-targets       # lint
cargo fmt                                    # format
```

Tests live as inline `#[cfg(test)] mod tests` blocks beside the code they exercise; there is no top-level `tests/`
directory.

## License

&copy; 2026 SiLeader. This project is licensed under the Apache License Version 2.0.

See [LICENSE](./LICENSE) for more details.

# AGENT_REVIEWER.md

## Overview

This repository is a **Rust** project organized as a **Cargo workspace**. It implements an agent-driven code review pipeline that:

1. loads model/provider and prompt configuration,
2. runs a triage step,
3. dispatches one or more review steps, and
4. finalizes the collected review results into a single output.

The main binary crate lives at the repository root. Supporting workspace crates provide agent runtime logic, model-provider integration, and tool implementations.

## Language and tooling

- **Primary language:** Rust
- **Rust edition:** 2024
- **Build system:** Cargo workspace
- **CLI parsing:** `clap`
- **Async runtime:** `tokio`
- **LLM client library:** `genai`
- **Templating:** `minijinja`
- **Serialization:** `serde`, `serde_json`, `toml`, `schemars`
- **Git integration:** `git2`

## Workspace layout

```text
.
├── Cargo.toml                     # Workspace manifest + root binary package
├── Cargo.lock
├── src/                           # Root binary crate
│   ├── main.rs                    # CLI entrypoint
│   ├── config.rs                  # Runtime config schema
│   ├── instruction.rs             # Loads review instructions
│   ├── prompt.rs                  # Prompt manager and template rendering
│   ├── orchestrator/              # Triage/review/finalize workflow
│   └── default_prompts/           # Built-in prompt files
├── agent-reviewer-agent/          # ReAct-style agent runtime
│   └── src/
│       ├── builder.rs
│       ├── concurrency.rs
│       ├── lib.rs
│       └── tools/subagent/        # Explorer subagent support
├── agent-reviewer-model-provider/ # Model/provider config bridge for genai
│   └── src/
├── agent-reviewer-tools/          # Agent tool implementations
│   └── src/
│       ├── fs/                    # File listing and file reading tools
│       ├── git/                   # Git diff and diff summary tools
│       ├── lib.rs
│       └── multi.rs
├── README.md
└── LICENSE
```

## Crate responsibilities

### Root crate (`src/`)

The root package builds the executable. It is responsible for:

- parsing CLI arguments,
- loading configuration from `agent-reviewer.toml` by default,
- loading prompt templates and instructions,
- constructing the model client, and
- orchestrating the multi-step review flow.

Key files:

- `src/main.rs`: entrypoint and CLI behavior
- `src/config.rs`: config schema for models, providers, agent IDs, prompts, and concurrency
- `src/orchestrator/mod.rs`: high-level flow control for triage, review, and finalize steps
- `src/orchestrator/submit_marker/`: marker payload types exchanged with the LLM workflow
- `src/prompt.rs`: default prompt loading and Jinja template rendering

### `agent-reviewer-agent`

Contains the reusable agent runtime. This is where the ReAct-style loop is implemented: send prompts to the model, execute requested tools, feed tool results back, and stop when the expected submit marker is returned.

### `agent-reviewer-model-provider`

Translates repository configuration into `genai` client/provider setup. Changes here usually affect model selection, provider resolution, or provider-specific client wiring.

### `agent-reviewer-tools`

Defines the tool interfaces exposed to agents and implements the current tool set:

- filesystem tools (`fs/`)
- git tools (`git/`)
- tool composition helpers (`multi.rs`)

## Review-relevant runtime behavior

- The application is **configuration-driven**. Runtime model/provider behavior is defined through TOML config rather than hardcoded values.
- The root CLI expects a config file path, defaulting to **`agent-reviewer.toml`**.
- Prompt content can come from configured files, but the repository also embeds **default prompts** under `src/default_prompts/`.
- Review execution is split into phases: **triage -> per-unit review -> finalize**.
- The tool surface currently centers on **repository file access** and **git diff inspection**.

## Test layout

There is no dedicated top-level `tests/` directory in the current repository snapshot. Tests are primarily **inline unit tests** inside Rust modules, for example in:

- `src/prompt.rs`
- `src/orchestrator/submit_marker/review.rs`
- `src/orchestrator/submit_marker/triage.rs`
- `agent-reviewer-tools/src/fs/list_files.rs`

## What reviewers should usually inspect first

For most changes, start with the files that match the change type:

| Change type | Best starting points |
| --- | --- |
| CLI/config behavior | `src/main.rs`, `src/config.rs` |
| Workflow changes | `src/orchestrator/mod.rs`, `src/orchestrator/submit_marker/` |
| Prompt/template changes | `src/prompt.rs`, `src/default_prompts/` |
| Tool behavior | `agent-reviewer-tools/src/fs/`, `agent-reviewer-tools/src/git/` |
| Agent execution loop | `agent-reviewer-agent/src/lib.rs`, `agent-reviewer-agent/src/builder.rs` |
| Model/provider wiring | `agent-reviewer-model-provider/src/` |

## Notes

- `target/` is a build artifact directory and is not part of the source design.
- `.idea/` is editor metadata and is not relevant to code review.
- The repository currently has a minimal `README.md`, so the source tree is the most reliable place to understand behavior.
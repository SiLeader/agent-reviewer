const INSTRUCTION_FILES: &[&str] = &[
    "AGENT_REVIEWER.md",
    "AGENTS.md",
    ".github/copilot-instructions.md",
    "GEMINI.md",
    "CLAUDE.md",
];

fn load_instructions() -> String {
    for file in INSTRUCTION_FILES {
        if let Ok(content) = std::fs::read_to_string(file) {
            return content;
        }
    }
    "".to_string()
}

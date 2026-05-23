const INSTRUCTION_FILES: &[&str] = &[
    "AGENT_REVIEWER.md",
    "AGENTS.md",
    ".github/copilot-instructions.md",
    "GEMINI.md",
    "CLAUDE.md",
];

pub(crate) fn load_instructions() -> Option<String> {
    for file in INSTRUCTION_FILES {
        if let Ok(content) = std::fs::read_to_string(file) {
            return Some(content);
        }
    }
    None
}

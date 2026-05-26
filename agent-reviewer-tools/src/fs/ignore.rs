// Copyright 2026- SiLeader (Cerussite).
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, anyhow};

pub(super) struct Ignore {
    root_dir: PathBuf,
    exclude_patterns: Vec<String>,
    gitignore_cache: HashMap<PathBuf, Vec<GitignoreRule>>,
}

impl Ignore {
    pub(super) fn new(root_dir: impl Into<PathBuf>, exclude_patterns: Vec<String>) -> Self {
        Self {
            root_dir: root_dir.into(),
            exclude_patterns,
            gitignore_cache: HashMap::new(),
        }
    }

    pub(super) fn contains(
        &mut self,
        relative_path: &Path,
        absolute_path: &Path,
    ) -> anyhow::Result<bool> {
        Ok(
            is_excluded_by_patterns(relative_path, absolute_path, &self.exclude_patterns)
                || self.check_excluded_by_gitignore_and_update(relative_path)?,
        )
    }

    fn check_excluded_by_gitignore_and_update(
        &mut self,
        relative_path: &Path,
    ) -> anyhow::Result<bool> {
        let mut ignored = false;

        for directory in gitignore_directories(&self.root_dir, relative_path) {
            let rules = gitignore_rules(&self.root_dir, &directory, &mut self.gitignore_cache)?;

            for rule in rules {
                if rule.matches(relative_path) {
                    ignored = !rule.negated;
                }
            }
        }

        Ok(ignored)
    }
}

fn is_excluded_by_patterns(
    relative_path: &Path,
    absolute_path: &Path,
    patterns: &[String],
) -> bool {
    let relative_path = normalize_path(relative_path);
    let absolute_path = normalize_path(absolute_path);

    patterns.iter().any(|pattern| {
        let normalized_pattern = normalize_pattern(pattern);

        if Path::new(pattern).is_absolute() {
            path_matches_pattern(&normalized_pattern, &absolute_path)
        } else if normalized_pattern.contains('/') {
            path_matches_pattern(&normalized_pattern, &relative_path)
        } else {
            path_or_ancestor_segment_matches(&normalized_pattern, &relative_path)
        }
    })
}

fn gitignore_directories(root_dir: &Path, relative_path: &Path) -> Vec<PathBuf> {
    let mut directories = vec![root_dir.to_path_buf()];
    let mut current = root_dir.to_path_buf();

    if let Some(parent) = relative_path.parent() {
        for component in parent.components() {
            current.push(component.as_os_str());
            directories.push(current.clone());
        }
    }

    directories
}

fn gitignore_rules<'a>(
    root_dir: &Path,
    directory: &Path,
    cache: &'a mut HashMap<PathBuf, Vec<GitignoreRule>>,
) -> anyhow::Result<&'a Vec<GitignoreRule>> {
    if !cache.contains_key(directory) {
        let path = directory.join(".gitignore");
        let rules = if path.exists() {
            let base_dir = directory.strip_prefix(root_dir).with_context(|| {
                format!(
                    "gitignore directory {} is outside root directory {}",
                    directory.display(),
                    root_dir.display()
                )
            })?;
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;

            parse_gitignore(&content, base_dir)
        } else {
            Vec::new()
        };

        cache.insert(directory.to_path_buf(), rules);
    }

    cache.get(directory).ok_or_else(|| {
        anyhow!(
            "failed to cache gitignore rules for {}",
            directory.display()
        )
    })
}

#[derive(Debug)]
struct GitignoreRule {
    base_dir: PathBuf,
    pattern: String,
    negated: bool,
    anchored: bool,
    directory_only: bool,
    has_slash: bool,
}

impl GitignoreRule {
    fn matches(&self, relative_path: &Path) -> bool {
        let scoped_path = match relative_path.strip_prefix(&self.base_dir) {
            Ok(path) => path,
            Err(_) => return false,
        };
        let scoped_path = normalize_path(scoped_path);

        if scoped_path.is_empty() {
            return false;
        }

        if self.directory_only {
            return ancestor_paths(&scoped_path)
                .iter()
                .any(|path| self.matches_scoped_path(path));
        }

        std::iter::once(&scoped_path)
            .chain(ancestor_paths(&scoped_path).iter())
            .any(|path| self.matches_scoped_path(path))
    }

    fn matches_scoped_path(&self, path: &str) -> bool {
        if self.has_slash || self.anchored {
            path_matches_pattern(&self.pattern, path)
        } else {
            path_or_ancestor_segment_matches(&self.pattern, path)
        }
    }
}

fn parse_gitignore(content: &str, base_dir: &Path) -> Vec<GitignoreRule> {
    content
        .lines()
        .filter_map(|line| parse_gitignore_line(line, base_dir))
        .collect()
}

fn parse_gitignore_line(line: &str, base_dir: &Path) -> Option<GitignoreRule> {
    let mut line = trim_unescaped_trailing_spaces(line.trim_end_matches('\r')).to_string();

    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let negated = line.starts_with('!');
    if negated || line.starts_with("\\!") || line.starts_with("\\#") {
        line.remove(0);
    }

    let anchored = line.starts_with('/');
    if anchored {
        line.remove(0);
    }

    let directory_only = line.ends_with('/');
    while line.ends_with('/') {
        line.pop();
    }

    if line.is_empty() {
        return None;
    }

    let pattern = normalize_gitignore_pattern(&line);
    let has_slash = pattern.contains('/');

    Some(GitignoreRule {
        base_dir: base_dir.to_path_buf(),
        pattern,
        negated,
        anchored,
        directory_only,
        has_slash,
    })
}

fn trim_unescaped_trailing_spaces(line: &str) -> &str {
    let mut end = line.len();

    while end > 0 {
        let Some((index, ch)) = line[..end].char_indices().next_back() else {
            break;
        };

        if ch != ' ' {
            break;
        }

        let backslashes = line[..index]
            .chars()
            .rev()
            .take_while(|ch| *ch == '\\')
            .count();

        if backslashes % 2 == 1 {
            break;
        }

        end = index;
    }

    &line[..end]
}

fn normalize_gitignore_pattern(pattern: &str) -> String {
    pattern.trim_start_matches("./").to_string()
}

fn normalize_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

pub(super) fn normalize_path(path: &Path) -> String {
    let mut normalized = String::new();

    for component in path.components() {
        match component {
            std::path::Component::RootDir if normalized.is_empty() => normalized.push('/'),
            std::path::Component::Normal(value) => {
                if !normalized.is_empty() && !normalized.ends_with('/') {
                    normalized.push('/');
                }
                normalized.push_str(&value.to_string_lossy());
            }
            _ => {}
        }
    }

    normalized
}

fn ancestor_paths(path: &str) -> Vec<String> {
    let mut ancestors = Vec::new();
    let mut parts = path.split('/').collect::<Vec<_>>();

    while parts.len() > 1 {
        parts.pop();
        ancestors.push(parts.join("/"));
    }

    ancestors
}

fn path_or_ancestor_segment_matches(pattern: &str, path: &str) -> bool {
    path.split('/')
        .any(|segment| segment_matches(pattern, segment))
}

fn path_matches_pattern(pattern: &str, path: &str) -> bool {
    let pattern_segments = split_path(pattern);
    let path_segments = split_path(path);

    path_segments_match(&pattern_segments, &path_segments)
}

fn split_path(path: &str) -> Vec<&str> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

fn path_segments_match(pattern: &[&str], path: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }

    if pattern[0] == "**" {
        return path_segments_match(&pattern[1..], path)
            || (!path.is_empty() && path_segments_match(pattern, &path[1..]));
    }

    !path.is_empty()
        && segment_matches(pattern[0], path[0])
        && path_segments_match(&pattern[1..], &path[1..])
}

fn segment_matches(pattern: &str, text: &str) -> bool {
    let pattern = pattern.chars().collect::<Vec<_>>();
    let text = text.chars().collect::<Vec<_>>();
    let mut memo = HashMap::new();

    segment_matches_at(&pattern, &text, 0, 0, &mut memo)
}

fn segment_matches_at(
    pattern: &[char],
    text: &[char],
    pattern_index: usize,
    text_index: usize,
    memo: &mut HashMap<(usize, usize), bool>,
) -> bool {
    if let Some(result) = memo.get(&(pattern_index, text_index)) {
        return *result;
    }

    let result = if pattern_index == pattern.len() {
        text_index == text.len()
    } else {
        match pattern[pattern_index] {
            '*' => {
                segment_matches_at(pattern, text, pattern_index + 1, text_index, memo)
                    || (text_index < text.len()
                        && segment_matches_at(pattern, text, pattern_index, text_index + 1, memo))
            }
            '?' => {
                text_index < text.len()
                    && segment_matches_at(pattern, text, pattern_index + 1, text_index + 1, memo)
            }
            '[' => match character_class_matches(pattern, pattern_index, text.get(text_index)) {
                Some((matches, next_pattern_index)) => {
                    matches
                        && segment_matches_at(
                            pattern,
                            text,
                            next_pattern_index,
                            text_index + 1,
                            memo,
                        )
                }
                None => {
                    text.get(text_index) == Some(&'[')
                        && segment_matches_at(
                            pattern,
                            text,
                            pattern_index + 1,
                            text_index + 1,
                            memo,
                        )
                }
            },
            '\\' => {
                let literal = pattern.get(pattern_index + 1).unwrap_or(&'\\');
                text.get(text_index) == Some(literal)
                    && segment_matches_at(
                        pattern,
                        text,
                        pattern_index + usize::from(pattern_index + 1 < pattern.len()) + 1,
                        text_index + 1,
                        memo,
                    )
            }
            literal => {
                text.get(text_index) == Some(&literal)
                    && segment_matches_at(pattern, text, pattern_index + 1, text_index + 1, memo)
            }
        }
    };

    memo.insert((pattern_index, text_index), result);
    result
}

fn character_class_matches(
    pattern: &[char],
    pattern_index: usize,
    text: Option<&char>,
) -> Option<(bool, usize)> {
    let text = text?;
    let mut index = pattern_index + 1;
    let negated = matches!(pattern.get(index), Some('!') | Some('^'));
    if negated {
        index += 1;
    }

    let mut matched = false;
    let mut previous = None;

    while index < pattern.len() {
        match pattern[index] {
            ']' if previous.is_some() => return Some((matched != negated, index + 1)),
            '-' if previous.is_some() && pattern.get(index + 1).is_some_and(|ch| *ch != ']') => {
                let start = previous.unwrap();
                let end = pattern[index + 1];
                if start <= *text && *text <= end {
                    matched = true;
                }
                previous = Some(end);
                index += 2;
            }
            ch => {
                if ch == *text {
                    matched = true;
                }
                previous = Some(ch);
                index += 1;
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::write_file;
    use std::{
        fs,
        ops::Deref,
        path::{Path, PathBuf},
    };

    #[test]
    fn respects_gitignore_anchored_paths_and_negation() {
        let root = test_root("anchored_paths_and_negation");
        write_file(
            &root.join(".gitignore"),
            "/root.log\n*.tmp\n!important.tmp\n",
        );

        assert!(is_ignored(&root, "root.log", &[]));
        assert!(!is_ignored(&root, "nested/root.log", &[]));
        assert!(is_ignored(&root, "ignored.tmp", &[]));
        assert!(is_ignored(&root, "nested/ignored.tmp", &[]));
        assert!(!is_ignored(&root, "important.tmp", &[]));
        assert!(!is_ignored(&root, "nested/important.tmp", &[]));
    }

    #[test]
    fn respects_gitignore_directory_patterns() {
        let root = test_root("directory_patterns");
        write_file(&root.join(".gitignore"), "build/\n/only-root/\n");

        assert!(is_ignored(&root, "build/output.txt", &[]));
        assert!(is_ignored(&root, "nested/build/output.txt", &[]));
        assert!(is_ignored(&root, "only-root/output.txt", &[]));
        assert!(!is_ignored(&root, "nested/only-root/output.txt", &[]));
    }

    #[test]
    fn combines_explicit_excludes_with_gitignore() {
        let root = test_root("explicit_excludes");
        write_file(&root.join(".gitignore"), "*.log\n!keep.log\n");
        let exclude_patterns = ["*.rs"];

        assert!(is_ignored(&root, "drop.log", &exclude_patterns));
        assert!(!is_ignored(&root, "keep.log", &exclude_patterns));
        assert!(is_ignored(&root, "main.rs", &exclude_patterns));
        assert!(!is_ignored(&root, "README.md", &exclude_patterns));
    }

    #[test]
    fn supports_escaped_gitignore_wildcards() {
        let root = test_root("escaped_wildcards");
        write_file(&root.join(".gitignore"), "\\*.literal\n");

        assert!(is_ignored(&root, "*.literal", &[]));
        assert!(!is_ignored(&root, "actual.literal", &[]));
    }

    fn is_ignored(root: &Path, relative_path: &str, exclude_patterns: &[&str]) -> bool {
        let relative_path = Path::new(relative_path);
        let absolute_path = root.join(relative_path);
        let exclude_patterns = exclude_patterns
            .iter()
            .map(|pattern| pattern.to_string())
            .collect::<Vec<_>>();
        let mut ignore = Ignore::new(root, exclude_patterns);

        ignore.contains(relative_path, &absolute_path).unwrap()
    }

    struct TestRoot(PathBuf);

    impl Deref for TestRoot {
        type Target = Path;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn test_root(name: &str) -> TestRoot {
        let root = std::env::current_dir()
            .unwrap()
            .join("target")
            .join(format!("metsuke-ignore-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        TestRoot(root)
    }
}

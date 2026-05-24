use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::fs::check_path_location;
use crate::{AgentTool, tool_description};
use anyhow::{Context, anyhow};
use chrono::{DateTime, Utc};
use genai::chat::Tool;
use glob::glob;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub struct ListFiles;

#[derive(Debug, Deserialize, JsonSchema)]
struct ListFilesArgs {
    #[schemars(
        required,
        description = "The pattern to match files against. '**' can be used to match any number of directories, '*' can be used to match any number of characters in a file or directory name."
    )]
    pattern: String,
    #[schemars(
        required,
        description = "The pattern to exclude files against. '**' can be used to match any number of directories, '*' can be used to match any number of characters in a file or directory name."
    )]
    exclude_patterns: Option<Vec<String>>,
    #[schemars(required, description = "The root directory to start the search from.")]
    root_dir: Option<String>,
    #[schemars(required, description = "The maximum number of files to return.")]
    max_files: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListFilesResult {
    files: Vec<ListedFile>,
    total_matched_files: usize,
    returned_files: usize,
    truncated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListedFile {
    path: String,
    size: usize,
    modified_at: DateTime<Utc>,
}

#[async_trait::async_trait]
impl AgentTool for ListFiles {
    fn tool(&self) -> Tool {
        tool_description::<ListFilesArgs>("list_files", "Lists files in a directory.")
    }

    async fn run(&self, args: &Value) -> anyhow::Result<String> {
        let args: ListFilesArgs = serde_json::from_value(args.clone())?;

        tokio::task::spawn_blocking(move || list_files(args)).await?
    }
}

fn list_files(args: ListFilesArgs) -> anyhow::Result<String> {
    let root_dir = args.root_dir.unwrap_or_else(|| ".".to_string());
    check_path_location(&root_dir)?;

    let root_dir = fs::canonicalize(&root_dir)
        .with_context(|| format!("failed to resolve root directory: {root_dir}"))?;

    let search_pattern = search_pattern(&root_dir, &args.pattern);
    let exclude_patterns = args.exclude_patterns.unwrap_or_default();
    let max_files = args.max_files.unwrap_or(usize::MAX);
    let mut gitignore_cache = HashMap::new();
    let mut files = Vec::new();

    for entry in
        glob(&search_pattern).with_context(|| format!("invalid pattern: {search_pattern}"))?
    {
        let path = entry.with_context(|| format!("failed to read glob entry: {search_pattern}"))?;
        let metadata = fs::metadata(&path)
            .with_context(|| format!("failed to read metadata for {}", path.display()))?;

        if !metadata.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(&root_dir)
            .with_context(|| {
                format!(
                    "matched path {} is outside root directory {}",
                    path.display(),
                    root_dir.display()
                )
            })?
            .to_path_buf();

        if relative_path
            .file_name()
            .is_some_and(|name| name == ".gitignore")
            || is_excluded_by_patterns(&relative_path, &path, &exclude_patterns)
            || is_excluded_by_gitignore(&root_dir, &relative_path, &mut gitignore_cache)?
        {
            continue;
        }

        files.push(ListedFile {
            path: normalize_path(&relative_path),
            size: usize::try_from(metadata.len())
                .with_context(|| format!("file is too large: {}", path.display()))?,
            modified_at: DateTime::<Utc>::from(
                metadata.modified().with_context(|| {
                    format!("failed to read modified time for {}", path.display())
                })?,
            ),
        });
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    let total_matched_files = files.len();
    let truncated = files.len() > max_files;
    files.truncate(max_files);

    let result = ListFilesResult {
        returned_files: files.len(),
        files,
        total_matched_files,
        truncated,
    };

    serde_json::to_string(&result).context("failed to serialize listFiles result")
}

pub(super) fn search_pattern(root_dir: &Path, pattern: &str) -> String {
    let pattern_path = Path::new(pattern);

    if pattern_path.is_absolute() {
        pattern.to_string()
    } else {
        root_dir.join(pattern_path).to_string_lossy().into_owned()
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

fn is_excluded_by_gitignore(
    root_dir: &Path,
    relative_path: &Path,
    cache: &mut HashMap<PathBuf, Vec<GitignoreRule>>,
) -> anyhow::Result<bool> {
    let mut ignored = false;

    for directory in gitignore_directories(root_dir, relative_path) {
        let rules = gitignore_rules(root_dir, &directory, cache)?;

        for rule in rules {
            if rule.matches(relative_path) {
                ignored = !rule.negated;
            }
        }
    }

    Ok(ignored)
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

fn normalize_path(path: &Path) -> String {
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
    use std::ops::Deref;

    #[test]
    fn respects_gitignore_anchored_paths_and_negation() {
        let root = test_root("anchored_paths_and_negation");
        write_file(
            &root.join(".gitignore"),
            "/root.log\n*.tmp\n!important.tmp\n",
        );
        write_file(&root.join("root.log"), "ignored");
        write_file(&root.join("nested/root.log"), "included");
        write_file(&root.join("ignored.tmp"), "ignored");
        write_file(&root.join("nested/ignored.tmp"), "ignored");
        write_file(&root.join("important.tmp"), "included");
        write_file(&root.join("nested/important.tmp"), "included");

        let files = run_list_files(&root);

        assert_eq!(
            paths(files),
            vec!["important.tmp", "nested/important.tmp", "nested/root.log"]
        );
    }

    #[test]
    fn respects_gitignore_directory_patterns() {
        let root = test_root("directory_patterns");
        write_file(&root.join(".gitignore"), "build/\n/only-root/\n");
        write_file(&root.join("build/output.txt"), "ignored");
        write_file(&root.join("nested/build/output.txt"), "ignored");
        write_file(&root.join("only-root/output.txt"), "ignored");
        write_file(&root.join("nested/only-root/output.txt"), "included");

        let files = run_list_files(&root);

        assert_eq!(paths(files), vec!["nested/only-root/output.txt"]);
    }

    #[test]
    fn combines_explicit_excludes_with_gitignore() {
        let root = test_root("explicit_excludes");
        write_file(&root.join(".gitignore"), "*.log\n!keep.log\n");
        write_file(&root.join("drop.log"), "ignored");
        write_file(&root.join("keep.log"), "included");
        write_file(&root.join("main.rs"), "excluded");
        write_file(&root.join("README.md"), "included");

        let args = ListFilesArgs {
            pattern: "**/*".to_string(),
            exclude_patterns: Some(vec!["*.rs".to_string()]),
            root_dir: Some(root.to_string_lossy().into_owned()),
            max_files: None,
        };
        let result: ListFilesResult = serde_json::from_str(&list_files(args).unwrap()).unwrap();

        assert_eq!(paths(result.files), vec!["README.md", "keep.log"]);
        assert_eq!(result.total_matched_files, 2);
        assert!(!result.truncated);
    }

    #[test]
    fn supports_escaped_gitignore_wildcards() {
        let root = test_root("escaped_wildcards");
        write_file(&root.join(".gitignore"), "\\*.literal\n");
        write_file(&root.join("*.literal"), "ignored");
        write_file(&root.join("actual.literal"), "included");

        let files = run_list_files(&root);

        assert_eq!(paths(files), vec!["actual.literal"]);
    }

    #[test]
    fn truncates_results_after_counting_matches() {
        let root = test_root("truncates");
        write_file(&root.join("a.txt"), "a");
        write_file(&root.join("b.txt"), "b");

        let args = ListFilesArgs {
            pattern: "**/*".to_string(),
            exclude_patterns: None,
            root_dir: Some(root.to_string_lossy().into_owned()),
            max_files: Some(1),
        };
        let result: ListFilesResult = serde_json::from_str(&list_files(args).unwrap()).unwrap();

        assert_eq!(paths(result.files), vec!["a.txt"]);
        assert_eq!(result.total_matched_files, 2);
        assert_eq!(result.returned_files, 1);
        assert!(result.truncated);
    }

    fn run_list_files(root: &Path) -> Vec<ListedFile> {
        let args = ListFilesArgs {
            pattern: "**/*".to_string(),
            exclude_patterns: None,
            root_dir: Some(root.to_string_lossy().into_owned()),
            max_files: None,
        };
        let result: ListFilesResult = serde_json::from_str(&list_files(args).unwrap()).unwrap();

        result.files
    }

    fn paths(files: Vec<ListedFile>) -> Vec<String> {
        files.into_iter().map(|file| file.path).collect()
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
            .join(format!("metsuke-list-files-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        TestRoot(root)
    }

    fn write_file(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
}

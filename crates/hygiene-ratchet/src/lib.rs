//! Production-source hygiene budgets for Faber workspace crates.
//!
//! Scans non-test Rust sources for banned patterns (`.unwrap()`, `panic!(`, and
//! similar) and optional structural test-boundary violations.

use std::fs;
use std::path::{Path, PathBuf};

/// Monotonic ceilings for banned production patterns.
#[derive(Debug, Clone, Copy)]
pub struct Budgets {
    pub unwrap: usize,
    pub expect: usize,
    pub panic: usize,
    pub unreachable: usize,
    pub todo: usize,
    pub unimplemented: usize,
    pub let_underscore: usize,
    pub inline_test_modules: usize,
    pub test_attr_in_production: usize,
}

/// Observed production-only counts from one scan pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Counts {
    pub unwrap: usize,
    pub expect: usize,
    pub panic: usize,
    pub unreachable: usize,
    pub todo: usize,
    pub unimplemented: usize,
    pub let_underscore: usize,
    pub inline_test_modules: usize,
    pub test_attr_in_production: usize,
}

/// Scanner configuration for one crate integration test.
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub source_roots: Vec<PathBuf>,
    pub exclude_path_suffixes: Vec<String>,
    pub subtract_self_expect: bool,
}

#[derive(Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub scrubbed: String,
}

/// Collect non-test Rust source files from the configured source roots.
///
/// Skips `tests/` directories, files ending in `_test.rs` or `.test.rs`,
/// files under `test_support` paths, and files matching `exclude_path_suffixes`.
pub fn collect_production_files(config: &ScanConfig) -> Vec<SourceFile> {
    let mut files = Vec::new();
    for root in &config.source_roots {
        collect_rs_files(root, config, &mut files);
    }
    files
}

fn collect_rs_files(dir: &Path, config: &ScanConfig, out: &mut Vec<SourceFile>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "tests") {
                continue;
            }
            collect_rs_files(&path, config, out);
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if name.ends_with("_test.rs") || name.ends_with(".test.rs") {
            continue;
        }
        if is_test_support_path(&path) {
            continue;
        }
        if config
            .exclude_path_suffixes
            .iter()
            .any(|suffix| path.ends_with(suffix))
        {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let scrubbed = scrub_rust_source(&content);
        out.push(SourceFile {
            path,
            content,
            scrubbed,
        });
    }
}

/// Count banned patterns in scrubbed production sources.
///
/// Tracks `.unwrap()`, `.expect(`, `panic!(`, `unreachable!(`, `todo!(`,
/// `unimplemented!(`, `let _ =`, inline `#[cfg(test)] mod tests {` blocks,
/// and `#[test]` attributes.
pub fn count_budgets(files: &[SourceFile], subtract_self_expect: bool) -> Counts {
    let mut counts = Counts::default();
    for file in files {
        counts.unwrap += count_substring(&file.scrubbed, ".unwrap()");
        counts.expect += count_expect(&file.scrubbed, subtract_self_expect);
        counts.panic += count_substring(&file.scrubbed, "panic!(");
        counts.unreachable += count_substring(&file.scrubbed, "unreachable!(");
        counts.todo += count_substring(&file.scrubbed, "todo!(");
        counts.unimplemented += count_substring(&file.scrubbed, "unimplemented!(");
        counts.let_underscore += count_let_underscore(&file.scrubbed);
        if file.scrubbed.contains("#[cfg(test)]") && file.scrubbed.contains("mod tests {") {
            counts.inline_test_modules += 1;
        }
        if file.scrubbed.contains("#[test]") {
            counts.test_attr_in_production += 1;
        }
    }
    counts
}

/// Panic if any observed count exceeds its budget ceiling.
pub fn assert_budgets(counts: Counts, budgets: Budgets) {
    assert_budget(".unwrap()", counts.unwrap, budgets.unwrap);
    assert_budget(".expect(", counts.expect, budgets.expect);
    assert_budget("panic!(", counts.panic, budgets.panic);
    assert_budget("unreachable!(", counts.unreachable, budgets.unreachable);
    assert_budget("todo!(", counts.todo, budgets.todo);
    assert_budget(
        "unimplemented!(",
        counts.unimplemented,
        budgets.unimplemented,
    );
    assert_budget("let _ =", counts.let_underscore, budgets.let_underscore);
    assert_budget(
        "inline #[cfg(test)] mod tests {",
        counts.inline_test_modules,
        budgets.inline_test_modules,
    );
    assert_budget(
        "#[test] in production files",
        counts.test_attr_in_production,
        budgets.test_attr_in_production,
    );
}

/// Verify that every source file with a companion `_test.rs` file declares
/// it via the `#[cfg(test)] #[path = "..."] mod tests;` convention.
pub fn assert_companion_tests_use_cfg_path_module_convention(files: &[SourceFile]) {
    for file in files {
        let Some(companion) = companion_test_path(&file.path) else {
            continue;
        };
        if !companion.exists() {
            continue;
        }

        let companion_name = companion.file_name().unwrap_or_default().to_string_lossy();
        let expected = format!("#[cfg(test)]\n#[path = \"{companion_name}\"]\nmod tests;");
        assert!(
            file.content.contains(&expected),
            "{} has companion test {}, but is missing the repo convention:\n{}",
            file.path.display(),
            companion.display(),
            expected
        );
    }
}

fn is_test_support_path(path: &Path) -> bool {
    if path
        .file_name()
        .is_some_and(|name| name == "test_support.rs")
    {
        return true;
    }
    path.components()
        .any(|component| component.as_os_str() == "test_support")
}

fn companion_test_path(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_string_lossy();
    Some(path.with_file_name(format!("{stem}_test.rs")))
}

fn assert_budget(name: &str, observed: usize, budget: usize) {
    assert!(
        observed <= budget,
        "{name} budget exceeded: found {observed}, max {budget}."
    );
}

fn count_substring(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

fn count_expect(haystack: &str, subtract_self_expect: bool) -> usize {
    let mut count = count_substring(haystack, ".expect(");
    if subtract_self_expect {
        count = count.saturating_sub(count_substring(haystack, "self.expect("));
    }
    count
}

fn count_let_underscore(haystack: &str) -> usize {
    haystack
        .lines()
        .filter(|line| line.contains("let _ ="))
        .count()
}

/// Replace comments, strings, char literals, and lifetimes with spaces.
///
/// Preserves newlines so line-oriented counting stays accurate.
pub fn scrub_rust_source(source: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        LineComment,
        BlockComment,
        String,
        Char,
    }

    let mut out = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    let mut state = State::Code;

    while let Some(ch) = chars.next() {
        match state {
            State::Code => match ch {
                '/' if chars.peek() == Some(&'/') => {
                    out.push(' ');
                    out.push(' ');
                    chars.next();
                    state = State::LineComment;
                }
                '/' if chars.peek() == Some(&'*') => {
                    out.push(' ');
                    out.push(' ');
                    chars.next();
                    state = State::BlockComment;
                }
                '"' => {
                    out.push(' ');
                    state = State::String;
                }
                '\'' => {
                    // Distinguish lifetimes ('ident) from char literals ('x' / '\x').
                    // A lifetime starts with ' followed by an identifier character.
                    out.push(' ');
                    if let Some(&next) = chars.peek() {
                        if next.is_ascii_alphabetic() || next == '_' {
                            // Lifetime: skip the identifier but keep emitting spaces
                            while let Some(&c) = chars.peek() {
                                if c.is_ascii_alphanumeric() || c == '_' {
                                    chars.next();
                                    out.push(' ');
                                } else {
                                    break;
                                }
                            }
                        } else {
                            state = State::Char;
                        }
                    }
                }
                _ => out.push(ch),
            },
            State::LineComment => {
                if ch == '\n' {
                    out.push('\n');
                    state = State::Code;
                } else {
                    out.push(' ');
                }
            }
            State::BlockComment => {
                if ch == '*' && chars.peek() == Some(&'/') {
                    out.push(' ');
                    out.push(' ');
                    chars.next();
                    state = State::Code;
                } else if ch == '\n' {
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            }
            State::String => {
                if ch == '\\' {
                    out.push(' ');
                    if let Some(escaped) = chars.next() {
                        out.push(if escaped == '\n' { '\n' } else { ' ' });
                    }
                } else if ch == '"' {
                    out.push(' ');
                    state = State::Code;
                } else if ch == '\n' {
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            }
            State::Char => {
                if ch == '\\' {
                    out.push(' ');
                    if let Some(escaped) = chars.next() {
                        out.push(if escaped == '\n' { '\n' } else { ' ' });
                    }
                } else if ch == '\'' {
                    out.push(' ');
                    state = State::Code;
                } else if ch == '\n' {
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_substring_counts_occurrences() {
        assert_eq!(count_substring("hello world hello", "hello"), 2);
        assert_eq!(count_substring("no match here", "xyz"), 0);
        assert_eq!(count_substring("", "a"), 0);
        assert_eq!(count_substring("aaa", "aa"), 2); // overlapping
    }

    #[test]
    fn count_expect_without_subtract() {
        let src = "x.expect(\"a\"); self.expect(\"b\");";
        assert_eq!(count_expect(src, false), 2);
    }

    #[test]
    fn count_expect_subtracts_self_expect() {
        let src = "x.expect(\"a\"); self.expect(\"b\");";
        assert_eq!(count_expect(src, true), 1);
    }

    #[test]
    fn count_expect_empty() {
        assert_eq!(count_expect("", false), 0);
        assert_eq!(count_expect("no dot expect here", false), 0);
    }

    #[test]
    fn count_let_underscore_counts_lines() {
        let src = "let _ = foo();\nbar();\nlet _ = baz();";
        assert_eq!(count_let_underscore(src), 2);
    }

    #[test]
    fn count_let_underscore_ignores_inline_occurrences() {
        // Only counts one per line even if multiple `let _ =` appear on same line.
        assert_eq!(count_let_underscore("let _ = a(); let _ = b();"), 1);
    }

    #[test]
    fn count_let_underscore_empty() {
        assert_eq!(count_let_underscore(""), 0);
    }

    #[test]
    fn assert_budget_does_not_panic_on_match() {
        assert_budget(".unwrap()", 0, 0); // exactly at budget
        assert_budget(".unwrap()", 1, 5); // within budget
    }

    #[test]
    #[should_panic(expected = "budget exceeded")]
    fn assert_budget_panics_when_exceeded() {
        assert_budget("test", 3, 1);
    }

    #[test]
    fn is_test_support_path_matches_exact_names() {
        assert!(is_test_support_path(Path::new("src/test_support.rs")));
        assert!(is_test_support_path(Path::new("test_support.rs")));
        assert!(is_test_support_path(Path::new("src/test_support/mod.rs")));
    }

    #[test]
    fn is_test_support_path_rejects_other_paths() {
        assert!(!is_test_support_path(Path::new("src/lib.rs")));
        assert!(!is_test_support_path(Path::new("")));
    }

    #[test]
    fn companion_test_path_generates_expected_name() {
        assert_eq!(
            companion_test_path(Path::new("src/lib.rs")),
            Some(PathBuf::from("src/lib_test.rs"))
        );
        assert_eq!(
            companion_test_path(Path::new("foo/bar/baz.rs")),
            Some(PathBuf::from("foo/bar/baz_test.rs"))
        );
    }

    #[test]
    fn companion_test_path_returns_none_without_stem() {
        assert_eq!(companion_test_path(Path::new("")), None);
    }

    #[test]
    fn scrub_rust_source_preserves_code_characters() {
        let src = "fn foo() -> i32 { 42 }";
        assert_eq!(scrub_rust_source(src), src);
    }

    #[test]
    fn scrub_rust_source_replaces_line_comments_with_spaces() {
        let src = "abc // comment\nxyz";
        let expected = "abc           \nxyz";
        assert_eq!(scrub_rust_source(src), expected);
    }

    #[test]
    fn scrub_rust_source_replaces_block_comments_with_spaces() {
        let src = "abc /* inner */ xyz";
        let expected = "abc             xyz";
        assert_eq!(scrub_rust_source(src), expected);
    }

    #[test]
    fn scrub_rust_source_replaces_string_literals_with_spaces() {
        let src = "x = \"hello\";";
        let expected = "x =        ;";
        assert_eq!(scrub_rust_source(src), expected);
    }

    #[test]
    fn scrub_rust_source_replaces_char_literals_with_spaces() {
        let src = "let ch = 'x';";
        let expected = "let ch =   ;";
        assert_eq!(scrub_rust_source(src), expected);
    }

    #[test]
    fn scrub_rust_source_preserves_lifetimes() {
        let src = "fn foo<'a>(x: &'a str) -> &'a str";
        let expected = "fn foo   (x: &  str) -> &  str";
        assert_eq!(scrub_rust_source(src), expected);
    }

    #[test]
    fn scrub_rust_source_handles_empty_input() {
        assert_eq!(scrub_rust_source(""), "");
    }
}

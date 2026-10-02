use super::{Input, projectform::expand_home};
use std::{
    fs,
    path::{self, MAIN_SEPARATOR, PathBuf},
};

/// Windows and macOS file systems ignore case by default, so matching does too.
const IGNORE_CASE: bool = cfg!(any(windows, target_os = "macos"));

/// The folders listed after Tab finds several matches in the directory field.
pub struct Completion {
    /// The typed text up to and including the last separator; matches go after it.
    base: String,
    pub matches: Vec<String>,
    /// The match Tab has cycled to, if any.
    pub selected: Option<usize>,
}

impl Input {
    /// Completes the directory being typed, like a shell. A single matching folder
    /// is filled in with a trailing separator. Several matches fill in the prefix
    /// they share and get listed; pressing Tab again cycles through them
    /// (`backwards` for Shift+Tab). Hidden folders only match a prefix starting
    /// with `.`. Matching ignores case on Windows and macOS, and completions use
    /// each folder's real capitalization.
    pub fn complete_directory(&mut self, backwards: bool) {
        if let Some(completion) = &mut self.completion {
            let count = completion.matches.len();
            let next = match (completion.selected, backwards) {
                (None, false) => 0,
                (None, true) => count - 1,
                (Some(i), false) => (i + 1) % count,
                (Some(i), true) => (i + count - 1) % count,
            };
            completion.selected = Some(next);
            let text = with_separator(&completion.base, &completion.matches[next]);
            self.set_input(text);
            return;
        }

        if self.input == "~" {
            self.set_input("~/".to_string());
            return;
        }

        let split = self.input.rfind(path::is_separator).map_or(0, |i| i + 1);
        let base = self.input[..split].to_string();
        let matches = subdirectories(&base, &self.input[split..]);
        match matches.as_slice() {
            [] => {}
            [only] => self.set_input(with_separator(&base, only)),
            _ => {
                self.set_input(format!("{base}{}", common_prefix(&matches)));
                self.completion = Some(Completion {
                    base,
                    matches,
                    selected: None,
                });
            }
        }
    }

    fn set_input(&mut self, text: String) {
        self.input = text;
        self.character_index = self.input.chars().count();
    }
}

/// `base` + `name` + a separator, reusing the separator style already typed.
fn with_separator(base: &str, name: &str) -> String {
    let separator = base
        .chars()
        .rev()
        .find(|&c| path::is_separator(c))
        .unwrap_or(MAIN_SEPARATOR);
    format!("{base}{name}{separator}")
}

/// Sorted names of the folders in `base` (the current folder if empty) that start
/// with `prefix`. Unreadable folders have no matches.
fn subdirectories(base: &str, prefix: &str) -> Vec<String> {
    let dir = if base.is_empty() {
        PathBuf::from(".")
    } else {
        match expand_home(base) {
            Ok(dir) => dir,
            Err(_) => return Vec::new(),
        }
    };
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let show_hidden = prefix.starts_with('.');
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| starts_with(name, prefix) && (show_hidden || !name.starts_with('.')))
        .collect();
    names.sort();
    names
}

/// The longest prefix every name shares. `names` must not be empty.
fn common_prefix(names: &[String]) -> &str {
    let first = &names[0];
    let end = names[1..].iter().fold(first.len(), |end, name| {
        let mut other = name.chars();
        first[..end]
            .char_indices()
            .find(|&(_, a)| !other.next().is_some_and(|b| same_char(a, b)))
            .map_or(end, |(i, _)| i)
    });
    &first[..end]
}

fn same_char(a: char, b: char) -> bool {
    if IGNORE_CASE {
        a.to_lowercase().eq(b.to_lowercase())
    } else {
        a == b
    }
}

fn starts_with(name: &str, prefix: &str) -> bool {
    let mut chars = name.chars();
    prefix
        .chars()
        .all(|p| chars.next().is_some_and(|n| same_char(n, p)))
}

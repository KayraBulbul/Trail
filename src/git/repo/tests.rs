use super::*;
use std::{fs, path::PathBuf};

struct TempRepo {
    path: PathBuf,
}

impl TempRepo {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("trail-git-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        let repo = TempRepo { path };
        repo.run(&["init", "-q", "-b", "main"]);
        repo
    }

    fn run(&self, args: &[&str]) -> String {
        let mut full = vec![
            "-c",
            "user.name=Trail Test",
            "-c",
            "user.email=test@trail.invalid",
            "-c",
            "commit.gpgsign=false",
        ];
        full.extend_from_slice(args);
        git(&self.path, &full).unwrap()
    }

    fn commit_file(&self, name: &str, contents: &str, message: &str) {
        fs::write(self.path.join(name), contents).unwrap();
        self.run(&["add", name]);
        self.run(&["commit", "-q", "-m", message]);
    }
}

impl Drop for TempRepo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn detect_returns_none_outside_a_repo() {
    let dir = std::env::temp_dir().join(format!("trail-plain-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();

    assert!(detect(&dir).is_none());

    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn detect_returns_none_for_missing_directory() {
    assert!(detect(Path::new("/definitely/not/a/real/dir")).is_none());
}

#[test]
fn detect_finds_branch_in_repo_without_commits() {
    let repo = TempRepo::new();

    let info = detect(&repo.path).unwrap();

    assert_eq!(info.head, Head::Branch("main".to_string()));
}

#[test]
fn detect_works_from_a_subdirectory() {
    let repo = TempRepo::new();
    repo.commit_file("README.md", "hi", "init");
    let sub = repo.path.join("pkg");
    fs::create_dir_all(&sub).unwrap();

    assert!(detect(&sub).is_some());
}

#[test]
fn detect_reports_detached_head() {
    let repo = TempRepo::new();
    repo.commit_file("README.md", "hi", "init");
    let sha = repo
        .run(&["rev-parse", "--short", "HEAD"])
        .trim()
        .to_string();
    repo.run(&["checkout", "-q", "--detach"]);

    assert_eq!(detect(&repo.path).unwrap().head, Head::Detached(sha));
}

#[test]
fn summary_returns_none_for_no_commits() {
    let repo = TempRepo::new();
    let sum = summary(&repo.path).unwrap();

    assert!(sum.last_commit.is_none());
}

#[test]
fn summary_includes_last_commit() {
    let repo = TempRepo::new();
    repo.commit_file("README.md", "hi", "init");
    let sum = summary(&repo.path).unwrap();

    assert!(sum.last_commit.is_some());
}

#[test]
fn summary_files_changed_count_includes_untracked() {
    let repo = TempRepo::new();
    repo.commit_file("README.md", "hi", "init");
    fs::write(repo.path.join("README.md"), "changed").unwrap();
    fs::write(repo.path.join("test.txt"), "hello").unwrap();
    let sum = summary(&repo.path).unwrap();

    assert_eq!(sum.files_changed, 2);
}

#[test]
fn summary_files_changed_scoped_to_subdirectory() {
    let repo = TempRepo::new();

    let sub_directory = repo.path.join("sub/");
    fs::create_dir_all(&sub_directory).unwrap();

    repo.commit_file("README.md", "hi", "init");
    fs::write(repo.path.join("README.md"), "changed").unwrap();
    fs::write(repo.path.join("test.txt"), "hello").unwrap();
    fs::write(sub_directory.join("inner_test.txt"), "hello from subdir").unwrap();

    let sum_outer = summary(&repo.path).unwrap();
    let sum_inner = summary(&sub_directory).unwrap();

    assert_eq!(sum_outer.files_changed, 3);
    assert_eq!(sum_inner.files_changed, 1);
}

#[test]
fn commits_are_newest_first_and_keep_tabs_in_subject() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a", "first");
    repo.commit_file("b.txt", "b", "second\twith tab");
    let head = repo.run(&["rev-parse", "HEAD"]).trim().to_string();

    let commits = commits(&repo.path, "main", COMMIT_LIMIT).unwrap();

    assert_eq!(commits.len(), 2);
    assert_eq!(commits[0].sha, head);
    assert!(head.starts_with(&commits[0].short_sha));
    assert_eq!(commits[0].subject, "second\twith tab");
    assert_eq!(commits[1].subject, "first");
}

#[test]
fn commits_respects_limit() {
    let repo = TempRepo::new();
    for name in ["a", "b", "c"] {
        repo.commit_file(name, name, name);
    }

    assert_eq!(commits(&repo.path, "main", 2).unwrap().len(), 2);
}

#[test]
fn commits_reads_other_branches_without_checking_out() {
    let repo = TempRepo::new();
    repo.commit_file("a.txt", "a", "on main");
    repo.run(&["checkout", "-q", "-b", "feature"]);
    repo.commit_file("b.txt", "b", "on feature");
    repo.run(&["checkout", "-q", "main"]);

    let commits = commits(&repo.path, "feature", COMMIT_LIMIT).unwrap();

    assert_eq!(commits[0].subject, "on feature");
    assert_eq!(
        detect(&repo.path).unwrap().head,
        Head::Branch("main".to_string())
    );
}

#[test]
fn commits_scoped_to_subdirectory() {
    let repo = TempRepo::new();
    fs::create_dir_all(repo.path.join("sub")).unwrap();
    repo.commit_file("root.txt", "root", "root change");
    repo.commit_file("sub/inner.txt", "inner", "sub change");

    let commits = commits(&repo.path.join("sub"), "main", COMMIT_LIMIT).unwrap();

    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].subject, "sub change");
}

#[test]
fn commit_diff_has_stat_and_added_lines() {
    let repo = TempRepo::new();
    repo.commit_file("notes.txt", "hello trail\n", "add notes");
    let sha = repo.run(&["rev-parse", "HEAD"]).trim().to_string();

    let diff = commit_diff(&repo.path, &sha).unwrap();

    assert!(diff.stat.contains("notes.txt"));
    assert!(diff.patch.iter().any(|line| line == "+hello trail"));
    assert!(!diff.patch.iter().any(|line| line.starts_with("commit ")));
    assert!(!diff.truncated);
}

#[test]
fn commit_diff_scoped_to_subdirectory() {
    let repo = TempRepo::new();
    fs::create_dir_all(repo.path.join("sub")).unwrap();
    fs::write(repo.path.join("root.txt"), "root\n").unwrap();
    fs::write(repo.path.join("sub/inner.txt"), "inner\n").unwrap();
    repo.run(&["add", "."]);
    repo.run(&["commit", "-q", "-m", "both"]);
    let sha = repo.run(&["rev-parse", "HEAD"]).trim().to_string();

    let diff = commit_diff(&repo.path.join("sub"), &sha).unwrap();

    assert!(diff.stat.contains("inner.txt"));
    assert!(!diff.stat.contains("root.txt"));
    assert!(!diff.patch.iter().any(|line| line.contains("root.txt")));
}

#[test]
fn cap_lines_keeps_short_text_whole() {
    assert_eq!(
        cap_lines("a\nb\n"),
        (vec!["a".to_string(), "b".to_string()], false)
    );
}

#[test]
fn cap_lines_at_limit_is_not_truncated() {
    let (lines, truncated) = cap_lines(&"x\n".repeat(DIFF_LINE_LIMIT));

    assert_eq!(lines.len(), DIFF_LINE_LIMIT);
    assert!(!truncated);
}

#[test]
fn cap_lines_over_limit_is_truncated() {
    let (lines, truncated) = cap_lines(&"x\n".repeat(DIFF_LINE_LIMIT + 1));

    assert_eq!(lines.len(), DIFF_LINE_LIMIT);
    assert!(truncated);
}

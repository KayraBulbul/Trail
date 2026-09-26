#![allow(dead_code)]

use chrono::{DateTime, Utc};
use std::{fmt, io, path::Path, process::Command};

pub const COMMIT_LIMIT: usize = 50;
pub const DIFF_LINE_LIMIT: usize = 5000;

#[derive(Debug, PartialEq)]
pub enum Head {
    Branch(String),
    Detached(String),
}

#[derive(Debug)]
pub struct GitInfo {
    pub head: Head,
}

#[derive(Debug)]
pub struct Summary {
    pub head: Head,
    pub files_changed: usize,
    pub last_commit: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub struct Branch {
    pub name: String,
    pub is_head: bool,
    pub last_commit: DateTime<Utc>,
}

#[derive(Debug)]
pub struct Commit {
    pub sha: String,
    pub short_sha: String,
    pub subject: String,
    pub time: DateTime<Utc>,
}

#[derive(Debug, Default)]
pub struct Diff {
    pub stat: String,
    pub patch: Vec<String>,
    pub untracked: Vec<String>,
    pub truncated: bool,
}

#[derive(Debug)]
pub enum GitError {
    NotInstalled,
    Io(io::Error),
    Failed(String),
}

impl fmt::Display for GitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GitError::NotInstalled => write!(f, "git is not installed"),
            GitError::Io(error) => write!(f, "couldn't run git: {error}"),
            GitError::Failed(stderr) => write!(f, "git failed: {stderr}"),
        }
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| match error.kind() {
            io::ErrorKind::NotFound => GitError::NotInstalled,
            _ => GitError::Io(error),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(GitError::Failed(stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Returns `None` if git isn't installed, `dir` doesn't exist, or `dir` isn't
/// inside a repo. Works for subdirectories of a repo and for repos with no commits.
pub fn detect(dir: &Path) -> Option<GitInfo> {
    if !dir.is_dir() {
        return None;
    }
    git(dir, &["rev-parse", "--is-inside-work-tree"]).ok()?;

    let head = match git(dir, &["symbolic-ref", "--short", "-q", "HEAD"]) {
        Ok(branch) => Head::Branch(branch.trim().to_string()),
        Err(_) => Head::Detached(
            git(dir, &["rev-parse", "--short", "HEAD"])
                .ok()?
                .trim()
                .to_string(),
        ),
    };

    Some(GitInfo { head })
}

pub fn summary(dir: &Path) -> Result<Summary, GitError> {
    let Some(info) = detect(dir) else {
        return Err(GitError::Failed("not a git repository".into()));
    };

    let files_changed = git(dir, &["status", "--porcelain", "--", "."])?
        .lines()
        .count();

    let last_commit = git(dir, &["log", "-1", "--format=%ct", "--", "."])
        .ok()
        .and_then(|out| out.trim().parse::<i64>().ok())
        .and_then(|secs| DateTime::from_timestamp(secs, 0));

    Ok(Summary {
        head: info.head,
        files_changed,
        last_commit,
    })
}

pub fn branches(dir: &Path) -> Result<Vec<Branch>, GitError> {
    let branches = git(
        dir,
        &[
            "for-each-ref",
            "--sort=-committerdate",
            "--format=%(HEAD)%09%(refname:short)%09%(committerdate:unix)",
            "refs/heads",
        ],
    )?;

    let mut vec: Vec<Branch> = Vec::new();

    for branch in branches.lines() {
        let split: Vec<&str> = branch.split('\t').collect();
        let [head, name, time] = split[..] else {
            continue;
        };
        let is_head = head == "*";
        let Some(last_commit) = time
            .parse::<i64>()
            .ok()
            .and_then(|secs| DateTime::from_timestamp(secs, 0))
        else {
            continue;
        };

        vec.push(Branch {
            name: name.to_string(),
            is_head,
            last_commit,
        });
    }

    Ok(vec)
}

pub fn commits(dir: &Path, branch: &str, limit: usize) -> Result<Vec<Commit>, GitError> {
    let commits = git(
        dir,
        &[
            "log",
            branch,
            "-n",
            limit.to_string().as_str(),
            "--format=%H%x09%h%x09%ct%x09%s",
            "--",
            ".",
        ],
    )?;

    let mut vec: Vec<Commit> = Vec::new();

    for commit in commits.lines() {
        let log: Vec<&str> = commit.splitn(4, '\t').collect();
        let [sha, short_sha, unix_time, subject] = log[..] else {
            continue;
        };

        let Some(time) = unix_time
            .parse::<i64>()
            .ok()
            .and_then(|secs| DateTime::from_timestamp(secs, 0))
        else {
            continue;
        };

        vec.push(Commit {
            sha: sha.to_string(),
            short_sha: short_sha.to_string(),
            subject: subject.to_string(),
            time,
        });
    }

    Ok(vec)
}

/// The diff a single commit introduced.
///
/// Hints:
/// - `stat`: `git show --stat --format= <sha> -- .`
/// - `patch`: `git show --format= <sha> -- .`, split into lines, capped with `cap_lines`.
///   (`--format=` with nothing after it hides the commit header.)
pub fn commit_diff(dir: &Path, sha: &str) -> Result<Diff, GitError> {
    let stat = git(dir, &["show", "--stat", "--format=", sha, "--", "."])?;
    let output = git(dir, &["show", "--format=", sha, "--", "."])?;

    let (patch, truncated) = cap_lines(&output);
    Ok(Diff {
        stat,
        patch,
        truncated,
        ..Default::default()
    })
}

/// Staged + unstaged changes, plus the names of untracked files.
///
/// Hints:
/// - `stat` / `patch`: like `commit_diff`, but `git diff --stat HEAD -- .` and `git diff HEAD -- .`
///   (this fails when there are no commits yet; decide what to return then).
/// - `untracked`: `git ls-files --others --exclude-standard -- .`
pub fn uncommitted_diff(dir: &Path) -> Result<Diff, GitError> {
    todo!()
}

/// Splits `text` into at most `DIFF_LINE_LIMIT` lines. The bool says whether it cut anything off.
fn cap_lines(text: &str) -> (Vec<String>, bool) {
    (
        text.lines()
            .take(DIFF_LINE_LIMIT)
            .map(|line| line.to_string())
            .collect(),
        text.lines().count() > DIFF_LINE_LIMIT,
    )
}

#[cfg(test)]
mod tests;

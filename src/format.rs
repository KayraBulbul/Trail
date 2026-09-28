//! Text formatting shared by the TUI and the CLI, so both describe things the same way.

use chrono::{DateTime, Utc};

pub fn relative_time(time: DateTime<Utc>) -> String {
    let seconds = (Utc::now() - time).num_seconds().max(0);
    match seconds {
        0..60 => "just now".to_string(),
        60..3600 => format!("{}m ago", seconds / 60),
        3600..86400 => format!("{}h ago", seconds / 3600),
        86400..2_592_000 => format!("{}d ago", seconds / 86400),
        2_592_000..31_536_000 => format!("{}mo ago", seconds / 2_592_000),
        _ => format!("{}y ago", seconds / 31_536_000),
    }
}

/// `main @ 9c06ef6`, `9c06ef6` when detached, or `main` before the first commit.
pub fn written_on(branch: Option<&str>, commit_sha: Option<&str>) -> Option<String> {
    let short_sha = commit_sha.map(|sha| &sha[..sha.len().min(7)]);
    match (branch, short_sha) {
        (Some(branch), Some(sha)) => Some(format!("{branch} @ {sha}")),
        (Some(branch), None) => Some(branch.to_string()),
        (None, Some(sha)) => Some(sha.to_string()),
        (None, None) => None,
    }
}

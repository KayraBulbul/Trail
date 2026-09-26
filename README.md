# Trail

A TUI built with rust. Helps Students and devs keep track of what they were last working on.

## Install

Linux / macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/KayraBulbul/Trail/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/KayraBulbul/Trail/main/install.ps1 | iex
```

## Git

If a project's directory is inside a git repository, Trail detects it automatically. You need `git` installed; without it, Trail works as usual without the git features.

- Git projects are marked with `git` in the Projects list.
- The Latest Update pane shows the current branch, how many files have changed, and when the last commit was made.
- Press `g` on an open git project to browse its branches and commits and view their diffs. This is read-only: Trail never checks out branches or changes your repository.

If the project directory is a subfolder of a repository, Trail only shows commits and changes inside that folder.

Git view keys:

| Key | Action |
| --- | --- |
| `j` / `k` | Move through branches or commits, or scroll the diff |
| `Enter` | Open a branch's commits, or a commit's diff |
| `Ctrl+h` / `Ctrl+l` | Focus the list / the diff |
| `r` | Refresh |
| `Esc` | Go back (diff → commits → branches → close) |
| `q` | Quit |

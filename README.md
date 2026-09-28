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

## Command line

Run `trail` on its own to open the TUI. The commands below work from a project's folder (or any subfolder), so AI coding agents can read and write updates too.

| Command | What it does |
| --- | --- |
| `trail status` | The project for this folder, its git state, and the latest update |
| `trail log [-n 5]` | The most recent updates, newest first |
| `trail add --title "..." --body "..." [--next "..."]` | Write an update. Records the current git branch and commit. Use `--body -` to read the body from stdin |
| `trail projects` | List all projects |
| `trail init [--name ...]` | Make the current folder a project (named after the folder by default) |
| `trail agents` | Print instructions for AI agents |
| `trail update` | Install the latest version |

- `-p <name>` on `status`, `log` and `add` picks a project by name instead of by folder.
- `--json` on any command prints JSON instead of text. Missing values are `null`.
- If a folder is inside more than one project, the deepest one is used.

### Using Trail with AI agents

Agents forget everything between sessions. Add Trail's instructions to your project's `AGENTS.md` or `CLAUDE.md`, and the agent will check `trail status` when it starts and record an update when it finishes:

```sh
trail agents >> AGENTS.md
```

The instructions tell the agent to never edit or delete updates. That stays in the TUI, with you.

# Trail

Trail keeps a log of where you left off on each of your projects: what you did, and what to do next.

Your AI coding agents (Claude Code, Codex, ...) read and write the same log from the command line. Switch from one agent to another, or come back to a project after a week, and the context is already there. No re-explaining, no ever-growing markdown file. You read the history in a terminal UI, one place for all your projects.

## Install

Linux / macOS:

```sh
curl -fsSL https://raw.githubusercontent.com/KayraBulbul/Trail/main/install.sh | sh
```

Windows (PowerShell):

```powershell
irm https://raw.githubusercontent.com/KayraBulbul/Trail/main/install.ps1 | iex
```

The installer downloads the latest release and checks it against the release's SHA256 checksums before installing. Run `trail update` to install a newer version later. The TUI also lets you know when one is out.

## Getting started

Run `trail` to open the TUI.

1. Press `A` to add a project. Give it a name and the folder it lives in.
2. Press `Enter` to open it.
3. Press `a` to write an update: a title, what you did, and what to do next. `Shift+Enter` starts a new line.

The Latest Update pane always shows the newest update. Press `u` to see all of them, and `?` for every key.

| Key | Action |
| --- | --- |
| `A` | New project |
| `Enter` | Open the selected project |
| `a` | New update |
| `e` | Edit the update being shown |
| `u` | All updates for the open project |
| `g` | Git view (git projects only) |
| `d` | Delete the selected project or update |
| `j` / `k` | Move or scroll |
| `Ctrl+h` / `Ctrl+l` | Focus Projects / Latest Update |
| `?` | Help |
| `q` | Quit |

## Using Trail with AI agents

Agents forget everything between sessions. Add Trail's instructions to your project's `AGENTS.md` or `CLAUDE.md`, and the agent will check `trail status` when it starts and record an update when it finishes:

```sh
trail agents >> AGENTS.md
```

The instructions tell the agent to never edit or delete updates. That stays in the TUI, with you.

If the folder isn't a Trail project yet, run `trail init` in it (or add it in the TUI).

## Command line

The commands below work from a project's folder (or any subfolder).

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

## Git

If a project's directory is inside a git repository, Trail detects it automatically. You need `git` installed; without it, Trail works as usual without the git features.

- Git projects are marked with `git` in the Projects list.
- The Latest Update pane shows the current branch, how many files have changed, and when the last commit was made.
- Every new update records the branch and commit it was written on.
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

## Your data

Everything is stored locally in a single SQLite database. Nothing is sent anywhere. The only network requests are the update check and `trail update`, which talk to GitHub releases.

| OS | Database |
| --- | --- |
| Linux | `~/.local/share/trail/trail.db` |
| macOS | `~/Library/Application Support/trail/trail.db` |
| Windows | `%LOCALAPPDATA%\trail\trail.db` |

To uninstall, delete the `trail` binary (`~/.local/bin/trail` on Linux and macOS, `%LOCALAPPDATA%\Programs\trail` on Windows) and, if you want to remove your data too, the `trail` folder above.

## Contributing

Bug reports and ideas are welcome in [issues](https://github.com/KayraBulbul/Trail/issues). Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## License

[MIT](LICENSE)

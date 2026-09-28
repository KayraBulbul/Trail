## Trail

This project keeps a log of where work left off in [Trail](https://github.com/KayraBulbul/Trail).

- At the start of a session, run `trail status` to see the latest update and what to do next. Use `trail log` for earlier updates.
- At the end of a session, record what you did: `trail add --title "<short title>" --body "<what you did>" --next "<what to do next>"`. For a long body, pass `--body -` and pipe it in on stdin.
- Add `--json` to any command for machine-readable output.
- Never edit or delete updates. If `trail status` says no project covers this folder, ask the user before running `trail init`.

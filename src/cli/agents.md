## Trail

This project keeps a log of where work left off in [Trail](https://github.com/KayraBulbul/Trail).

- At the start of a session, run `trail status` to see the latest update, what to do next, and the open plan (the work still to do). Work from the plan. Use `trail log` for earlier updates.
- At the end of a session, record what you did: `trail add --title "<short title>" --body "<what you did>" --next "<what to do next>"`. For a long body, pass `--body -` and pipe it in on stdin.
- `--next` is the immediate next step. Work that lasts longer belongs in the plan.
- Plan items are pieces of work that take one or more sessions, not small TODOs. Add one only when the user agrees to the work, or when it's left over from this session: `trail plan add "<item>"`.
- Close the plan items a session finished with `trail add ... --done <id>,<id>`. If the user cuts an item, `trail plan drop <id>` and say why in the update.
- Add `--json` to any command for machine-readable output.
- Never edit or delete updates. If `trail status` says no project covers this folder, ask the user before running `trail init`.

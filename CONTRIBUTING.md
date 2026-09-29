# Contributing to Trail

Thanks for wanting to help! Bug reports, ideas and pull requests are all welcome.

## Before you open a pull request

Please open an issue first for anything bigger than a small fix, and wait until we've agreed on the approach. A few reasons:

- Trail is also how I'm learning Rust, so I sometimes want to write a feature myself even when the idea is good.
- Some things are already in progress or planned:
  - An MCP server (`trail mcp`) is being built on the `mcp-server` branch.
  - A visual redesign of the TUI is planned for v1.0, so larger UI changes may clash with it.
- Some things are left out on purpose. For example, editing and deleting updates stays in the TUI, so agents can't rewrite history from the command line.

Typo fixes, doc improvements and small, obvious bug fixes can go straight to a pull request.

## Development

You need a recent stable Rust toolchain.

```sh
cargo run             # open the TUI
cargo run -- status   # run a CLI command
cargo test
```

`cargo run` uses your real Trail database, so be careful when trying out deletes. The tests use their own temporary databases.

Before pushing, make sure these pass. CI runs them on every pull request:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

## Pull requests

- Keep each pull request to one change, and add tests for new behavior where you can.
- Don't bump the version in `Cargo.toml`. That happens when a release is tagged.
- By contributing, you agree that your contributions are licensed under the [MIT License](LICENSE).

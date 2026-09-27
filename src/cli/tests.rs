use super::*;
use clap::CommandFactory;

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("trail").chain(args.iter().copied()))
}

#[test]
fn definitions_are_valid() {
    Cli::command().debug_assert();
}

#[test]
fn no_command_opens_the_tui() {
    let cli = parse(&[]).unwrap();
    assert!(cli.command.is_none());
    assert!(!cli.version);
}

#[test]
fn version_flag_keeps_short_lowercase_v() {
    assert!(parse(&["-v"]).unwrap().version);
    assert!(parse(&["--version"]).unwrap().version);
}

#[test]
fn log_defaults_to_five_and_json_works_after_the_command() {
    let cli = parse(&["log", "--json"]).unwrap();
    assert!(cli.json);
    let Some(Command::Log { count, target }) = cli.command else {
        panic!("expected log");
    };
    assert_eq!(count, 5);
    assert_eq!(target.project, None);

    let Some(Command::Log { count, .. }) = parse(&["log", "-n", "2"]).unwrap().command else {
        panic!("expected log");
    };
    assert_eq!(count, 2);
}

#[test]
fn add_requires_title_and_body_and_accepts_stdin_body() {
    assert!(parse(&["add", "--title", "T"]).is_err());

    let cli = parse(&["add", "-p", "Trail", "--title", "T", "--body", "-"]).unwrap();
    let Some(Command::Add {
        target,
        title,
        body,
        next,
    }) = cli.command
    else {
        panic!("expected add");
    };
    assert_eq!(target.project.as_deref(), Some("Trail"));
    assert_eq!((title.as_str(), body.as_str(), next), ("T", "-", None));
}

#[test]
fn unknown_commands_are_rejected() {
    assert!(parse(&["frobnicate"]).is_err());
}

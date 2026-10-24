//! Checks of the command-line definition.

use clap::{CommandFactory, Parser};

use super::*;

#[test]
fn the_definition_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn providers_can_be_repeated_or_comma_separated() {
    let cli = Cli::try_parse_from([
        "mul-tor",
        "upload",
        "a.zip",
        "-p",
        "gofile,oshi",
        "--provider",
        "pixeldrain",
    ])
    .unwrap();
    let Some(Command::Upload(args)) = cli.command else {
        panic!("not an upload command");
    };
    assert_eq!(args.providers, ["gofile", "oshi", "pixeldrain"]);
}

#[test]
fn upload_needs_a_path() {
    assert!(Cli::try_parse_from(["mul-tor", "upload"]).is_err());
}

#[test]
fn no_command_means_interactive() {
    assert!(Cli::try_parse_from(["mul-tor"]).unwrap().command.is_none());
}

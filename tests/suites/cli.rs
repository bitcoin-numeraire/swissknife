use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    // Help and argument errors must work without deployment configuration.
    let directory = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_swissknife"))
        .current_dir(directory.path())
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_lists_recovery_without_loading_config() {
    let result = run(&["--help"]);
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(help.contains("recover-local-login"));
    assert!(help.contains("--version"));
}

#[test]
fn recovery_help_describes_the_account_argument() {
    let result = run(&["recover-local-login", "--help"]);
    assert!(result.status.success());
    let help = String::from_utf8(result.stdout).unwrap();
    assert!(help.contains("<ACCOUNT_ID>"));
    assert!(help.contains("without connecting to a Lightning provider"));
}

#[test]
fn version_works_without_loading_config() {
    let result = run(&["--version"]);
    assert!(result.status.success());
    assert!(String::from_utf8(result.stdout)
        .unwrap()
        .contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn invalid_arguments_exit_with_a_usage_error() {
    for args in [
        vec!["unknown-command"],
        vec!["recover-local-login"],
        vec!["recover-local-login", "not-a-uuid"],
        vec!["recover-local-login", "00000000-0000-0000-0000-000000000001", "extra"],
    ] {
        let result = run(&args);
        assert_eq!(result.status.code(), Some(2), "{args:?}");
        let error = String::from_utf8(result.stderr).unwrap();
        assert!(error.contains("error:"), "{error}");
        assert!(!error.contains("Failed to load config"));
    }
}

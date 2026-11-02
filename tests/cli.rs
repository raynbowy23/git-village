use std::process::Command;
fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_git-village"))
        .args(arguments)
        .output()
        .unwrap()
}
#[test]
fn help_version_and_usage_exit_codes() {
    assert!(run(&["--help"]).status.success());
    assert!(
        String::from_utf8(run(&["--help"]).stdout)
            .unwrap()
            .contains("--output-dir")
    );
    assert!(
        String::from_utf8(run(&["--help"]).stdout)
            .unwrap()
            .contains("--setting")
    );
    assert!(run(&["--version"]).status.success());
    for arguments in [
        vec![],
        vec!["render"],
        vec!["collect", "--bad"],
        vec!["render", "x", "--output-dir", "x", "--setting"],
    ] {
        assert_eq!(run(&arguments).status.code(), Some(2));
    }
    let unknown = run(&["render", "x", "--output-dir", "x", "--setting", "venus"]);
    assert_eq!(unknown.status.code(), Some(1));
    assert!(
        String::from_utf8(unknown.stderr)
            .unwrap()
            .contains("unknown setting venus")
    );
    let result = run(&["collect", "owner/repo"]);
    assert_eq!(result.status.code(), Some(1));
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("one person's account")
    );
}
#[test]
fn failed_render_preserves_yesterdays_files() {
    let directory = std::env::temp_dir().join(format!("profile-cli-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("village.gif"), b"yesterday").unwrap();
    std::fs::write(directory.join("invalid.json"), br#"{"schema_version":99}"#).unwrap();
    let result = run(&[
        "render",
        directory.join("invalid.json").to_str().unwrap(),
        "--output-dir",
        directory.to_str().unwrap(),
    ]);
    assert_eq!(result.status.code(), Some(1));
    assert_eq!(
        std::fs::read(directory.join("village.gif")).unwrap(),
        b"yesterday"
    );
    assert!(
        String::from_utf8(result.stderr)
            .unwrap()
            .contains("upgrade")
    );
    std::fs::remove_dir_all(directory).unwrap();
}

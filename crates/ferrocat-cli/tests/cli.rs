use std::fs;
use std::process::Command;

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ferrocat"))
}

#[test]
fn binary_preserves_reports_and_parser_contracts() {
    let dir = tempfile::tempdir().expect("CLI test operation");
    let source = dir.path().join("-source.po");
    let target = dir.path().join("target.po");
    fs::write(&source, "msgid \"Hello\"\nmsgstr \"Hello\"\n").expect("CLI test operation");
    fs::write(&target, "msgid \"Hello\"\nmsgstr \"Hallo\"\n").expect("CLI test operation");
    let target_arg = format!("de={}", target.display());
    let run = |extra: &[&str]| {
        cli()
            .current_dir(dir.path())
            .args([
                "audit",
                "--source-locale",
                "wrong",
                "--source-locale",
                "en",
                "--source",
                "-source.po",
                "--target",
                &target_arg,
                "--locale",
                "de",
            ])
            .args(extra)
            .output()
            .expect("CLI test operation")
    };
    let text = run(&[]);
    assert!(text.status.success());
    assert_eq!(
        String::from_utf8(text.stdout).expect("CLI test operation"),
        "Catalog audit: 1 source messages, 1 target locales, 0 diagnostics (0 errors, 0 warnings, 0 info)\nNo catalog audit diagnostics.\n"
    );
    let legacy = run(&["--format", "text", "--format", "json"]);
    let shorthand = run(&["--json", "--format", "text"]);
    assert!(legacy.status.success() && shorthand.status.success());
    assert_eq!(legacy.stdout, shorthand.stdout);
    let json: serde_json::Value =
        serde_json::from_slice(&legacy.stdout).expect("CLI test operation");
    assert_eq!(json["summary"]["target_locales"], 1);
    for args in [
        &[][..],
        &["--help"][..],
        &["help"][..],
        &["audit", "--help"][..],
        &["audit", "--target", "--help"][..],
        &["audit", "--source-locale", "-h"][..],
    ] {
        let output = cli().args(args).output().expect("CLI test operation");
        assert!(output.status.success(), "{args:?}");
        assert!(!output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    let usage = cli()
        .args(["audit", "--source-locale"])
        .output()
        .expect("CLI test operation");
    assert_eq!(usage.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&usage.stderr).contains("Usage:"));
    fs::write(target, "").expect("CLI test operation");
    assert_eq!(run(&[]).status.code(), Some(1));
}

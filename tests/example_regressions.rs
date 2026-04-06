use std::fs;
use std::path::Path;
use std::process::Command;

fn run_script(script: &str, args: &[&str]) -> (String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nimble"));
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command.arg("run").arg(script);
    if !args.is_empty() {
        command.arg("--");
        command.args(args);
    }

    let output = command.output().expect("failed to run nimble binary");
    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");

    assert!(
        output.status.success(),
        "script {} failed\nstdout:\n{}\nstderr:\n{}",
        script,
        stdout,
        stderr
    );
    assert!(
        !stdout.contains("[ERROR]") && !stderr.contains("[ERROR]"),
        "script {} emitted a runtime diagnostic\nstdout:\n{}\nstderr:\n{}",
        script,
        stdout,
        stderr
    );

    (stdout, stderr)
}

fn check_script(script: &str) -> (String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_nimble"));
    command.current_dir(env!("CARGO_MANIFEST_DIR"));
    command.arg("check").arg(script);

    let output = command.output().expect("failed to run nimble binary");
    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");

    assert!(
        output.status.success(),
        "check {} failed\nstdout:\n{}\nstderr:\n{}",
        script,
        stdout,
        stderr
    );
    assert!(
        !stdout.contains("[ERROR]") && !stderr.contains("[ERROR]"),
        "check {} emitted a diagnostic\nstdout:\n{}\nstderr:\n{}",
        script,
        stdout,
        stderr
    );

    (stdout, stderr)
}

fn collect_examples(root: &Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = fs::read_dir(root)
        .expect("failed to read examples dir")
        .map(|entry| entry.expect("failed to read dir entry"))
        .collect();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_examples(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("nmb") {
            let relative = path
                .strip_prefix(env!("CARGO_MANIFEST_DIR"))
                .expect("example path should be inside repo")
                .to_string_lossy()
                .replace('\\', "/");
            out.push(relative);
        }
    }
}

#[test]
fn regex_examples_match_real_patterns() {
    let (stdout, _) = run_script("examples/stdlib/regex/extract.nmb", &[]);
    assert!(
        stdout.contains("found matches: true"),
        "stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("first match: nimble 101"),
        "stdout was:\n{stdout}"
    );
}

#[test]
fn json_roundtrip_preserves_nested_values() {
    let (stdout, _) = run_script("examples/stdlib/json/roundtrip.nmb", &[]);
    assert!(
        stdout.contains("\"features\":[\"fast\",\"fun\"]"),
        "stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("\"version\":\"0.2.0\""),
        "stdout was:\n{stdout}"
    );
    assert!(
        stdout.contains("\"features\": [\n    \"fast\",\n    \"fun\"\n  ]"),
        "stdout was:\n{stdout}"
    );
}

#[test]
fn os_args_exposes_script_args_only() {
    let script = "examples/stdlib/os/args.nmb";
    assert!(Path::new(script).exists(), "missing test script: {script}");

    let (stdout, _) = run_script(script, &["alpha", "beta"]);
    assert!(stdout.contains("arg: alpha"), "stdout was:\n{stdout}");
    assert!(stdout.contains("arg: beta"), "stdout was:\n{stdout}");
    assert!(!stdout.contains("arg: run"), "stdout was:\n{stdout}");
    assert!(!stdout.contains(script), "stdout was:\n{stdout}");
}

#[test]
fn shipped_examples_typecheck() {
    let mut examples = Vec::new();
    collect_examples(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .as_path(),
        &mut examples,
    );
    assert!(!examples.is_empty(), "no examples found");

    for example in examples {
        check_script(&example);
    }
}

#[test]
fn shipped_non_network_examples_run() {
    let mut examples = Vec::new();
    collect_examples(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples")
            .as_path(),
        &mut examples,
    );

    for example in examples {
        if example.contains("/stdlib/net/") {
            continue;
        }
        run_script(&example, &[]);
    }
}

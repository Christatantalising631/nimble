use std::process::Command;

fn run_script(script: &str) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_nimble"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .arg("run")
        .arg(script)
        .output()
        .expect("failed to run nimble binary");

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
        "script {} emitted a diagnostic\nstdout:\n{}\nstderr:\n{}",
        script,
        stdout,
        stderr
    );

    stdout
}

#[test]
fn ffi_strlen_example_runs() {
    let stdout = run_script("examples/stdlib/ffi/strlen.nmb");
    assert!(stdout.contains("loaded "), "stdout was:\n{stdout}");
    assert!(stdout.contains("strlen('nimble ffi') = 10"), "stdout was:\n{stdout}");
}

#[test]
fn ffi_abs_example_runs() {
    let stdout = run_script("examples/stdlib/ffi/abs.nmb");
    assert!(stdout.contains("abs(-42) = 42"), "stdout was:\n{stdout}");
}

#[test]
fn ffi_open_any_example_runs() {
    let stdout = run_script("examples/stdlib/ffi/open_any.nmb");
    assert!(stdout.contains("open_any abs(-7) = 7"), "stdout was:\n{stdout}");
}

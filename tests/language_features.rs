use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_temp_dir(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock went backwards")
        .as_nanos();
    std::env::temp_dir().join(format!("nimble-{name}-{unique}"))
}

fn write_tree(root: &Path, files: &[(&str, &str)]) {
    for (relative, contents) in files {
        let path = root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create temp parent dir");
        }
        fs::write(path, contents).expect("failed to write temp script");
    }
}

fn run_temp_script(
    mode: &str,
    main_source: &str,
    extra_files: &[(&str, &str)],
    args: &[&str],
) -> (String, String) {
    let root = unique_temp_dir(mode);
    fs::create_dir_all(&root).expect("failed to create temp dir");

    write_tree(&root, extra_files);

    let main_path = root.join("main.nmb");
    fs::write(&main_path, main_source).expect("failed to write main script");

    let mut command = Command::new(env!("CARGO_BIN_EXE_nimble"));
    command.current_dir(&root);
    command.arg(mode).arg(&main_path);
    if mode == "run" && !args.is_empty() {
        command.arg("--");
        command.args(args);
    }

    let output = command.output().expect("failed to run nimble binary");
    let stdout = String::from_utf8_lossy(&output.stdout).replace('\r', "");
    let stderr = String::from_utf8_lossy(&output.stderr).replace('\r', "");

    assert!(
        output.status.success(),
        "nimble {mode} failed\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );
    assert!(
        !stdout.contains("[ERROR]") && !stderr.contains("[ERROR]"),
        "nimble {mode} emitted a diagnostic\nstdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );

    (stdout, stderr)
}

#[test]
fn named_args_and_step_loops_work() {
    let source = r#"
cls User:
    name str
    age int

fn describe(name str, age int):
    out("{name}:{age}")

user = User(age=30, name="Alice")
describe(age=7, name="Bob")
for i in 0..10 step 3:
    out(i)
out(user.name)
out(user.age)
"#;

    let (stdout, _) = run_temp_script("run", source, &[], &[]);
    assert!(stdout.contains("Bob:7"), "stdout was:\n{stdout}");
    assert!(stdout.contains("0\n3\n6\n9"), "stdout was:\n{stdout}");
    assert!(stdout.contains("Alice\n30"), "stdout was:\n{stdout}");
}

#[test]
fn local_modules_load_relative_to_script() {
    let main = r#"
load helper from "./helper"
out(helper.answer())
"#;
    let helper = r#"
export fn answer():
    return 42
"#;

    let (stdout, _) = run_temp_script("run", main, &[("helper.nmb", helper)], &[]);
    assert!(stdout.contains("42"), "stdout was:\n{stdout}");
}

#[test]
fn documented_stdlib_aliases_work() {
    let source = r#"
load io
load list
load math
load string

path = "alias_test.txt"
io.write_file(path, "hello")?
io.append_file(path, " world")?
out(io.read_file(path)?)
io.delete_file(path)?

items = []
out(list.is_empty(items))
list.push(items, "a")?
list.push(items, "b")?
out(list.index_of(items, "b"))
out(list.sort([3, 1, 2])?)
out(math.add(2, 3))
out(math.sub(9, 4))
out(math.mul(6, 7))
out(string.is_empty(""))
"#;

    let (stdout, _) = run_temp_script("run", source, &[], &[]);
    assert!(stdout.contains("hello world"), "stdout was:\n{stdout}");
    assert!(stdout.contains("true"), "stdout was:\n{stdout}");
    assert!(stdout.contains("1"), "stdout was:\n{stdout}");
    assert!(stdout.contains("[1, 2, 3]"), "stdout was:\n{stdout}");
    assert!(stdout.contains("5"), "stdout was:\n{stdout}");
    assert!(stdout.contains("42"), "stdout was:\n{stdout}");
}

#[test]
fn check_accepts_documented_features() {
    let source = r#"
cls User:
    name str
    age int

fn describe(name str, age int):
    out("{name}:{age}")

load helper from "./helper"
user = User(age=30, name="Alice")
describe(age=7, name=helper.name())

for i in 0..10 step 2:
    out(i)
"#;
    let helper = r#"
export fn name():
    return "Bob"
"#;

    run_temp_script("check", source, &[("helper.nmb", helper)], &[]);
}

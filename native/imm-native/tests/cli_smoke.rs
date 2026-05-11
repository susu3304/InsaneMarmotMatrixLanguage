use std::process::Command;

#[test]
fn version_mentions_native_runtime() {
    let output = Command::new(env!("CARGO_BIN_EXE_imm-native"))
        .arg("--version")
        .output()
        .expect("run imm-native --version");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "insane marmot matrix native 0.1.0\n"
    );
}

#[test]
fn spec_json_is_parseable() {
    let output = Command::new(env!("CARGO_BIN_EXE_imm-native"))
        .args(["spec", "--json"])
        .output()
        .expect("run imm-native spec --json");
    assert!(output.status.success());
    let spec: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("spec output should be JSON");
    assert_eq!(spec["shortName"], "IMM");
    assert!(spec["keywords"]
        .as_array()
        .expect("keywords array")
        .contains(&serde_json::Value::String("marmot".to_string())));
}

#[test]
fn run_executes_native_runtime() {
    let output = Command::new(env!("CARGO_BIN_EXE_imm-native"))
        .args(["run", "../../examples/hello.imm"])
        .output()
        .expect("run imm-native hello");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Hello, insane marmot matrix!\n"
    );
}

#[test]
fn law_runs_shared_law_suite_natively() {
    let output = Command::new(env!("CARGO_BIN_EXE_imm-native"))
        .arg("law")
        .output()
        .expect("run imm-native law");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("core.law.imm"));
    assert!(stdout.contains("matrix.law.imm"));
    assert!(stdout.contains("web.law.imm"));
    assert!(stdout.contains("howl.law.imm"));
}

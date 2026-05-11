use std::process::Command;
use std::{
    fs,
    io::{Read, Write},
    net::TcpListener,
    thread,
};

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

#[test]
fn web_grab_supports_local_http_post() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local test server");
    let addr = listener.local_addr().expect("local addr");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 4096];
        let read = stream.read(&mut buffer).expect("read request");
        let request = String::from_utf8_lossy(&buffer[..read]);
        assert!(request.starts_with("POST /echo HTTP/1.1"));
        assert!(request.contains("x-imm-test: yes"));
        assert!(request.ends_with("native-body"));
        let body = r#"{"method":"POST","body":"native-body"}"#;
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
        .expect("write response");
    });

    let dir = std::env::temp_dir().join(format!("imm-native-web-test-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create temp dir");
    let file = dir.join("web.imm");
    fs::write(
        &file,
        format!(
            r#"use web

marmot main {{
    let res = web.grab({{
        "method": "POST",
        "url": "http://{addr}/echo",
        "headers": {{"x-imm-test": "yes"}},
        "body": "native-body",
        "timeout_ms": 2000
    }})
    squeak res.status
    squeak res.ok
    squeak res.json()["method"]
    squeak res.json()["body"]
}}
"#
        ),
    )
    .expect("write IMM program");

    let output = Command::new(env!("CARGO_BIN_EXE_imm-native"))
        .arg("run")
        .arg(&file)
        .output()
        .expect("run imm-native web program");
    server.join().expect("server thread");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "200\ntrue\nPOST\nnative-body\n"
    );
}

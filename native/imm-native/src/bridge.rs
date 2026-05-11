use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::diagnostics::CliError;

pub fn run_reference(args: &[OsString]) -> Result<i32, CliError> {
    let root = repo_root()?;
    let status = Command::new(python_interpreter())
        .arg(root.join("imm"))
        .args(args)
        .current_dir(root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;
    status.code().ok_or(CliError::MissingExitStatus)
}

fn python_interpreter() -> OsString {
    if let Some(path) = std::env::var_os("IMM_PYTHON") {
        return path;
    }

    for candidate in [
        "python3.13",
        "python3.12",
        "python3.11",
        "python3.10",
        "python3",
    ] {
        let status = Command::new(candidate)
            .args([
                "-c",
                "import sys; raise SystemExit(0 if sys.version_info >= (3, 10) else 1)",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if matches!(status, Ok(status) if status.success()) {
            return OsString::from(candidate);
        }
    }

    OsString::from("python3")
}

pub fn repo_root() -> Result<PathBuf, CliError> {
    if let Some(path) = std::env::var_os("IMM_REPO_ROOT") {
        let root = PathBuf::from(path);
        if is_repo_root(&root) {
            return Ok(root);
        }
    }

    let mut starts = Vec::new();
    starts.push(std::env::current_dir()?);
    if let Ok(exe) = std::env::current_exe() {
        starts.push(exe);
    }

    for start in starts {
        for ancestor in start.ancestors() {
            if is_repo_root(ancestor) {
                return Ok(ancestor.to_path_buf());
            }
        }
    }

    Err(CliError::RepoRoot(std::env::current_dir()?))
}

fn is_repo_root(path: &Path) -> bool {
    path.join("imm").is_file() && path.join("imm_lang").join("cli.py").is_file()
}

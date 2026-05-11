use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use clap::{CommandFactory, Parser, Subcommand};

use crate::diagnostics::{Category, Diagnostic};
use crate::parser::parse_source;
use crate::runtime::Runtime;
use crate::spec;

#[derive(Parser)]
#[command(
    name = "imm-native",
    about = "insane marmot matrix native runtime",
    disable_version_flag = true
)]
struct Args {
    #[arg(long, help = "show version")]
    version: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    Run {
        file: PathBuf,
        #[arg(long)]
        trace: bool,
    },
    Check {
        file: PathBuf,
    },
    Fmt {
        #[arg(long)]
        check: bool,
        file: PathBuf,
    },
    Probe {
        files: Vec<PathBuf>,
    },
    Law,
    Pack {
        file: PathBuf,
        #[arg(long = "crate")]
        crate_path: Option<PathBuf>,
        #[arg(long, value_parser = ["python", "native"])]
        pelt: Option<String>,
    },
    Spec {
        #[arg(long)]
        json: bool,
    },
}

pub fn run<I, T>(args: I) -> i32
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Args::try_parse_from(args) {
        Ok(parsed) => run_parsed(parsed),
        Err(err) => {
            let _ = err.print();
            err.exit_code()
        }
    }
}

pub fn run_embedded(entry: &str, sources: BTreeMap<String, String>, trace: bool) -> i32 {
    let Some(source) = sources.get(entry).cloned() else {
        eprintln!("pack error: embedded entry not found: {entry}");
        return 1;
    };
    match parse_source(0, &source) {
        Ok(program) => {
            let mut runtime = Runtime::with_embedded_sources(entry, sources);
            runtime.set_trace_enabled(trace);
            match runtime.run(&program, true) {
                Ok(()) => {
                    emit_runtime_output(&runtime);
                    0
                }
                Err(err) => {
                    emit_runtime_output(&runtime);
                    eprintln!("{err}");
                    1
                }
            }
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn run_parsed(args: Args) -> i32 {
    if args.version {
        println!("{}", spec::VERSION_TEXT);
        return 0;
    }

    let Some(command) = args.command else {
        let mut cmd = Args::command();
        let _ = cmd.print_help();
        eprintln!();
        return 2;
    };

    let result = match command {
        Command::Run { file, trace } => command_run(&file, trace),
        Command::Check { file } => command_check(&file),
        Command::Fmt { check, file } => command_fmt(&file, check),
        Command::Probe { files } => command_probe(&files),
        Command::Law => command_law(),
        Command::Pack {
            file,
            crate_path,
            pelt,
        } => command_pack(&file, crate_path.as_deref(), pelt.as_deref()),
        Command::Spec { json } => command_spec(json),
    };

    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn command_run(file: &Path, trace: bool) -> Result<i32, Diagnostic> {
    let path = absolutize(file)?;
    let source = fs::read_to_string(&path).map_err(io_error)?;
    let program = parse_source(0, &source)?;
    let mut runtime = Runtime::new(Some(path));
    runtime.set_trace_enabled(trace);
    runtime.run(&program, true)?;
    emit_runtime_output(&runtime);
    Ok(0)
}

fn command_check(file: &Path) -> Result<i32, Diagnostic> {
    let path = absolutize(file)?;
    let source = fs::read_to_string(&path).map_err(io_error)?;
    let program = parse_source(0, &source)?;
    let mut runtime = Runtime::new(Some(path));
    runtime.check(&program)?;
    println!("OK");
    Ok(0)
}

fn command_fmt(file: &Path, check: bool) -> Result<i32, Diagnostic> {
    let path = absolutize(file)?;
    let source = fs::read_to_string(&path).map_err(io_error)?;
    parse_source(0, &source)?;
    let formatted = simple_format(&source);
    if check {
        if source.replace("\r\n", "\n").replace('\r', "\n") != formatted {
            eprintln!("{} is not formatted", path.display());
            return Ok(1);
        }
        println!("OK");
    } else {
        fs::write(&path, formatted).map_err(io_error)?;
        println!("{}", path.display());
    }
    Ok(0)
}

fn command_probe(files: &[PathBuf]) -> Result<i32, Diagnostic> {
    let paths = if files.is_empty() {
        discover_probe_files(&repo_root()?)
    } else {
        files
            .iter()
            .map(PathBuf::as_path)
            .map(absolutize)
            .collect::<Result<Vec<_>, _>>()?
    };
    run_probe_paths(&paths, "probe")
}

fn command_law() -> Result<i32, Diagnostic> {
    let root = repo_root()?;
    let law_root = root.join("laws");
    let mut paths = fs::read_dir(&law_root)
        .map_err(io_error)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "imm"))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with(".law.imm"))
        })
        .collect::<Vec<_>>();
    paths.sort();
    if paths.is_empty() {
        println!("law: no law files found");
        return Ok(1);
    }
    run_probe_paths(&paths, "law")
}

fn command_pack(
    file: &Path,
    crate_path: Option<&Path>,
    pelt: Option<&str>,
) -> Result<i32, Diagnostic> {
    let entry = absolutize(file)?;
    let pelt = pelt.unwrap_or("native");
    if pelt != "native" {
        return Err(Diagnostic::new(
            Category::Pack,
            "imm-native only builds --pelt native artifacts",
        ));
    }
    let out = crate_path
        .map(absolutize)
        .transpose()?
        .unwrap_or_else(|| entry.with_extension(""));
    build_native_artifact(&entry, &out)?;
    println!("{}", out.display());
    Ok(0)
}

fn command_spec(json: bool) -> Result<i32, Diagnostic> {
    if json {
        println!(
            "{}",
            spec::render_json().map_err(|err| Diagnostic::new(Category::Io, err.to_string()))?
        );
    } else {
        println!("insane marmot matrix {}", spec::VERSION);
    }
    Ok(0)
}

fn run_probe_paths(paths: &[PathBuf], label: &str) -> Result<i32, Diagnostic> {
    let mut passed = 0;
    let mut failed = 0;
    for path in paths {
        let source = fs::read_to_string(path).map_err(io_error)?;
        let program = parse_source(0, &source)?;
        let mut runtime = Runtime::new(Some(path.clone()));
        runtime.check(&program)?;
        let results = runtime.run_probe_blocks(&program)?;
        for (name, ok, message) in results {
            if ok {
                passed += 1;
                println!("PASS {}: {}", path.display(), name);
            } else {
                failed += 1;
                eprintln!(
                    "FAIL {}: {}: {}",
                    path.display(),
                    name,
                    message.unwrap_or_default()
                );
            }
        }
    }
    println!("{label}: {passed} passed, {failed} failed");
    Ok(if failed == 0 { 0 } else { 1 })
}

fn emit_runtime_output(runtime: &Runtime) {
    for line in runtime.output_lines() {
        println!("{line}");
    }
    for line in runtime.trace_lines() {
        eprintln!("{line}");
    }
}

fn discover_probe_files(root: &Path) -> Vec<PathBuf> {
    let probe_root = root.join("tests").join("imm");
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(probe_root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "imm") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    paths
}

fn repo_root() -> Result<PathBuf, Diagnostic> {
    let mut current = std::env::current_dir().map_err(io_error)?;
    loop {
        if current.join("imm").is_file() && current.join("laws").is_dir() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(Diagnostic::new(
                Category::Io,
                "could not locate IMM repository root",
            ));
        }
    }
}

fn absolutize(path: &Path) -> Result<PathBuf, Diagnostic> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir().map_err(io_error)?.join(path))
    }
}

fn io_error(err: std::io::Error) -> Diagnostic {
    Diagnostic::new(Category::Io, err.to_string())
}

fn simple_format(source: &str) -> String {
    let mut indent = 0_usize;
    let mut out = String::new();
    for raw in source.replace("\r\n", "\n").replace('\r', "\n").lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('}') {
            indent = indent.saturating_sub(1);
        }
        out.push_str(&"    ".repeat(indent));
        out.push_str(line);
        out.push('\n');
        if line.ends_with('{') {
            indent += 1;
        }
    }
    out
}

fn build_native_artifact(entry: &Path, out: &Path) -> Result<(), Diagnostic> {
    let source_root = entry.parent().unwrap_or_else(|| Path::new("."));
    let mut sources = BTreeMap::new();
    for entry in fs::read_dir(source_root).map_err(io_error)? {
        let path = entry.map_err(io_error)?.path();
        if path.extension().is_some_and(|ext| ext == "imm") {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            sources.insert(name, fs::read_to_string(&path).map_err(io_error)?);
        }
    }
    let entry_name = entry
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let crate_dir = std::env::temp_dir().join(format!(
        "imm-native-pack-{}-{}",
        std::process::id(),
        SystemTimeId::now()
    ));
    fs::create_dir_all(crate_dir.join("src")).map_err(io_error)?;
    let imm_native_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"imm-packed-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nimm-native = {{ path = {:?} }}\n",
            imm_native_path
        ),
    )
    .map_err(io_error)?;
    fs::write(
        crate_dir.join("src").join("main.rs"),
        packed_main_source(&entry_name, &sources),
    )
    .map_err(io_error)?;
    let status = ProcessCommand::new("cargo")
        .args(["build", "--release"])
        .current_dir(&crate_dir)
        .status()
        .map_err(io_error)?;
    if !status.success() {
        return Err(Diagnostic::new(
            Category::Pack,
            "cargo build failed for native artifact",
        ));
    }
    let built = crate_dir
        .join("target")
        .join("release")
        .join(if cfg!(windows) {
            "imm-packed-app.exe"
        } else {
            "imm-packed-app"
        });
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    fs::copy(&built, out).map_err(io_error)?;
    Ok(())
}

fn packed_main_source(entry: &str, sources: &BTreeMap<String, String>) -> String {
    let mut inserts = String::new();
    for (name, source) in sources {
        inserts.push_str(&format!(
            "    sources.insert({name:?}.to_string(), {source:?}.to_string());\n"
        ));
    }
    format!(
        "use std::collections::BTreeMap;\n\nfn main() {{\n    let mut sources = BTreeMap::new();\n{inserts}    let trace = std::env::args().any(|arg| arg == \"--trace\");\n    std::process::exit(imm_native::cli::run_embedded({entry:?}, sources, trace));\n}}\n"
    )
}

struct SystemTimeId;

impl SystemTimeId {
    fn now() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default()
    }
}

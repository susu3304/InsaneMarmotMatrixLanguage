use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use clap::{CommandFactory, Parser, Subcommand};

use crate::bridge;
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

    match command {
        Command::Spec { json } => run_spec(json),
        other => {
            let cwd = match std::env::current_dir() {
                Ok(cwd) => cwd,
                Err(err) => {
                    eprintln!("IO error: {err}");
                    return 1;
                }
            };
            let reference_args = reference_args(other, &cwd);
            match bridge::run_reference(&reference_args) {
                Ok(code) => code,
                Err(err) => {
                    eprintln!("{err}");
                    1
                }
            }
        }
    }
}

fn run_spec(json: bool) -> i32 {
    if json {
        match spec::render_json() {
            Ok(rendered) => {
                println!("{rendered}");
                0
            }
            Err(err) => {
                eprintln!("JSON error: {err}");
                1
            }
        }
    } else {
        println!("insane marmot matrix {}", spec::VERSION);
        0
    }
}

fn reference_args(command: Command, cwd: &Path) -> Vec<OsString> {
    match command {
        Command::Run { file, trace } => {
            let mut args = vec![os("run"), absolutize(file, cwd).into_os_string()];
            if trace {
                args.push(os("--trace"));
            }
            args
        }
        Command::Check { file } => vec![os("check"), absolutize(file, cwd).into_os_string()],
        Command::Fmt { check, file } => {
            let mut args = vec![os("fmt")];
            if check {
                args.push(os("--check"));
            }
            args.push(absolutize(file, cwd).into_os_string());
            args
        }
        Command::Probe { files } => {
            let mut args = vec![os("probe")];
            args.extend(
                files
                    .into_iter()
                    .map(|path| absolutize(path, cwd).into_os_string()),
            );
            args
        }
        Command::Law => vec![os("law")],
        Command::Pack {
            file,
            crate_path,
            pelt,
        } => {
            let mut args = vec![os("pack"), absolutize(file, cwd).into_os_string()];
            if let Some(crate_path) = crate_path {
                args.push(os("--crate"));
                args.push(absolutize(crate_path, cwd).into_os_string());
            }
            if let Some(pelt) = pelt {
                args.push(os("--pelt"));
                args.push(OsString::from(pelt));
            }
            args
        }
        Command::Spec { .. } => unreachable!("spec is handled natively"),
    }
}

fn absolutize(path: PathBuf, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    }
}

fn os(value: &str) -> OsString {
    OsStr::new(value).to_os_string()
}

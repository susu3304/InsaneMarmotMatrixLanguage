import argparse
import json
import shutil
import sys
import tempfile
import zipapp
from pathlib import Path

from . import VERSION
from . import nodes as n
from .errors import ImmError
from .formatter import format_source
from .parser import parse
from .runtime import Runtime
from .tokenizer import tokenize


def load_program(path):
    source = Path(path).read_text(encoding="utf-8")
    return parse(tokenize(source))


def command_run(args):
    program = load_program(args.file)
    Runtime(args.file, trace_enabled=args.trace).run(program)
    return 0


def command_check(args):
    program = load_program(args.file)
    Runtime(args.file, output=lambda _value: None, input_func=lambda: "", check_only=True).check(program)
    print("OK")
    return 0


def command_fmt(args):
    path = Path(args.file)
    source = path.read_text(encoding="utf-8")
    formatted = format_source(source)
    parse(tokenize(formatted))
    if args.check:
        if source.replace("\r\n", "\n").replace("\r", "\n") != formatted:
            print(f"{path} is not formatted", file=sys.stderr)
            return 1
        print("OK")
        return 0
    path.write_text(formatted, encoding="utf-8")
    print(str(path))
    return 0


def command_spec(args):
    spec = {
        "name": "insane marmot matrix",
        "shortName": "IMM",
        "version": VERSION,
        "extension": ".imm",
        "commands": ["run", "check", "fmt", "probe", "law", "pack", "spec"],
        "entrypoints": ["marmot main", "insane marmot main", "howl marmot main", "insane howl marmot main"],
        "keywords": [
            "marmot",
            "insane",
            "dig",
            "let",
            "stash",
            "return",
            "if",
            "else",
            "for",
            "in",
            "while",
            "break",
            "continue",
            "true",
            "false",
            "null",
            "matrix",
            "burrow",
            "use",
            "squeak",
            "sniff",
            "panic",
            "try",
            "catch",
            "tunnel",
            "den",
            "hatch",
            "self",
            "init",
            "fur",
            "fang",
            "mask",
            "wear",
            "under",
            "web",
            "fetch",
            "grab",
            "howl",
            "wait",
            "scatter",
            "nest",
            "nap",
            "tick",
            "pack",
            "crate",
            "pelt",
            "probe",
            "law",
            "expect",
            "trace",
        ],
        "libraries": ["core", "math", "matrix", "path", "chaser", "store", "web", "tick"],
        "objectModel": ["den", "hatch", "self", "fur", "fang", "mask", "wear", "under"],
    }
    if args.json:
        print(json.dumps(spec, ensure_ascii=False, indent=2))
    else:
        print(f"{spec['name']} {spec['version']}")
    return 0


def command_probe(args):
    paths = [Path(path) for path in args.files] if args.files else discover_probe_files(Path.cwd())
    return run_probe_paths(paths, "probe")


def command_law(args):
    root = Path.cwd() / "laws"
    paths = sorted(root.glob("*.law.imm"))
    if not paths:
        print("law: no law files found")
        return 1
    return run_probe_paths(paths, "law")


def run_probe_paths(paths, label):
    passed = 0
    failed = 0
    for path in paths:
        program = load_program(path)
        Runtime(path, output=lambda _value: None, input_func=lambda: "", check_only=True).check(program)
        runtime = Runtime(path)
        results = runtime.run_probe_blocks(program)
        for name, ok, message in results:
            if ok:
                passed += 1
                print(f"PASS {path}: {name}")
            else:
                failed += 1
                print(f"FAIL {path}: {name}: {message}", file=sys.stderr)
        if not results:
            print(f"SKIP {path}: no probes")
    print(f"{label}: {passed} passed, {failed} failed")
    return 1 if failed else 0


def discover_probe_files(root):
    probe_root = root / "tests" / "imm"
    if not probe_root.exists():
        return []
    candidates = sorted(probe_root.glob("*.probe.imm")) + sorted(probe_root.glob("*.imm"))
    seen = set()
    result = []
    for path in candidates:
        if path in seen:
            continue
        seen.add(path)
        text = path.read_text(encoding="utf-8")
        if path.name.endswith(".probe.imm") or "probe " in text:
            result.append(path)
    return result


def command_pack(args):
    target = Path(args.file)
    if not target.exists():
        raise OSError(f"entry file not found: {target}")
    program = load_program(target)
    config = pack_config_from_program(program)
    base_dir = target.parent.resolve()
    entry = Path(args.file)
    if config.get("entry"):
        entry = Path(config["entry"])
        if not entry.is_absolute():
            entry = base_dir / entry
    if not entry.exists():
        raise OSError(f"entry file not found: {entry}")
    pelt = args.pelt or config.get("pelt") or "python"
    if args.crate:
        crate = Path(args.crate)
    elif config.get("crate"):
        crate = Path(config["crate"])
        if not crate.is_absolute():
            crate = base_dir / crate
    else:
        crate = entry.with_suffix(".pyz")
    if pelt != "python":
        raise ImmError(f"unsupported pelt {pelt!r}; supported pelts: python")
    build_python_pelt(entry.resolve(), crate)
    print(str(crate))
    return 0


def pack_config_from_program(program):
    config = {"entry": None, "crate": None, "pelt": None}
    for item in program.items:
        if isinstance(item, n.PackDef):
            config["entry"] = item.entry
            config["crate"] = item.crate
            config["pelt"] = item.pelt
    return config


def build_python_pelt(entry, crate):
    root = Path(__file__).resolve().parents[1]
    source_root = entry.parent
    crate = crate.resolve()
    crate.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory() as tmp:
        app_root = Path(tmp) / "app"
        app_root.mkdir()
        shutil.copytree(root / "imm_lang", app_root / "imm_lang")
        sources_root = app_root / "imm_sources"
        sources_root.mkdir()
        for source in source_root.rglob("*.imm"):
            destination = sources_root / source.relative_to(source_root)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, destination)
        entry_rel = entry.relative_to(source_root)
        (app_root / "__main__.py").write_text(
            f"""import sys
import tempfile
import zipfile
from pathlib import Path

from imm_lang.cli import main

with tempfile.TemporaryDirectory() as tmp:
    tmp_path = Path(tmp)
    with zipfile.ZipFile(sys.argv[0]) as archive:
        for name in archive.namelist():
            if name.startswith("imm_sources/") and not name.endswith("/"):
                target = tmp_path / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.read(name))
    raise SystemExit(main(["run", str(tmp_path / "imm_sources" / {str(entry_rel)!r}), *sys.argv[1:]]))
""",
            encoding="utf-8",
        )
        zipapp.create_archive(app_root, target=crate, interpreter="/usr/bin/env python3")


def build_parser():
    parser = argparse.ArgumentParser(prog="imm", description="insane marmot matrix interpreter")
    parser.add_argument("--version", action="store_true", help="show version")
    subparsers = parser.add_subparsers(dest="command")

    run = subparsers.add_parser("run", help="run an .imm file")
    run.add_argument("file")
    run.add_argument("--trace", action="store_true", help="write trace statements to stderr")
    run.set_defaults(func=command_run)

    check = subparsers.add_parser("check", help="check syntax")
    check.add_argument("file")
    check.set_defaults(func=command_check)

    fmt = subparsers.add_parser("fmt", help="format an .imm file")
    fmt.add_argument("--check", action="store_true", help="fail if the file is not formatted")
    fmt.add_argument("file")
    fmt.set_defaults(func=command_fmt)

    probe = subparsers.add_parser("probe", help="run IMM probe blocks")
    probe.add_argument("files", nargs="*")
    probe.set_defaults(func=command_probe)

    law = subparsers.add_parser("law", help="run IMM conformance law probes")
    law.set_defaults(func=command_law)

    pack = subparsers.add_parser("pack", help="pack an IMM program")
    pack.add_argument("file")
    pack.add_argument("--crate", help="output artifact path")
    pack.add_argument("--pelt", help="runtime bundle strategy", default=None)
    pack.set_defaults(func=command_pack)

    spec = subparsers.add_parser("spec", help="print machine-readable language metadata")
    spec.add_argument("--json", action="store_true", help="print JSON")
    spec.set_defaults(func=command_spec)
    return parser


def main(argv=None):
    parser = build_parser()
    args = parser.parse_args(argv)
    if args.version:
        print(f"insane marmot matrix {VERSION}")
        return 0
    if not hasattr(args, "func"):
        parser.print_help()
        return 2
    try:
        return args.func(args)
    except ImmError as err:
        file_prefix = f"{args.file}: " if hasattr(args, "file") else ""
        print(f"{file_prefix}{err}", file=sys.stderr)
        return 1
    except OSError as err:
        file_prefix = f"{args.file}: " if hasattr(args, "file") else ""
        print(f"{file_prefix}{err}", file=sys.stderr)
        return 1

import argparse
import json
import sys
from pathlib import Path

from . import VERSION
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
    Runtime(args.file).run(program)
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
        "commands": ["run", "check", "fmt", "spec"],
        "entrypoints": ["marmot main", "insane marmot main"],
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
        ],
        "libraries": ["core", "math", "matrix", "path", "chaser", "store"],
        "objectModel": ["den", "hatch", "self", "fur", "fang", "mask", "wear", "under"],
    }
    if args.json:
        print(json.dumps(spec, ensure_ascii=False, indent=2))
    else:
        print(f"{spec['name']} {spec['version']}")
    return 0


def build_parser():
    parser = argparse.ArgumentParser(prog="imm", description="insane marmot matrix interpreter")
    parser.add_argument("--version", action="store_true", help="show version")
    subparsers = parser.add_subparsers(dest="command")

    run = subparsers.add_parser("run", help="run an .imm file")
    run.add_argument("file")
    run.set_defaults(func=command_run)

    check = subparsers.add_parser("check", help="check syntax")
    check.add_argument("file")
    check.set_defaults(func=command_check)

    fmt = subparsers.add_parser("fmt", help="format an .imm file")
    fmt.add_argument("--check", action="store_true", help="fail if the file is not formatted")
    fmt.add_argument("file")
    fmt.set_defaults(func=command_fmt)

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

#!/usr/bin/env python3
import subprocess
import sys
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
IMM = ROOT / "imm"


def run_imm(*args):
    return subprocess.run(
        [str(IMM), *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def run_python(*args):
    return subprocess.run(
        [sys.executable, *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def run_artifact(path, *args):
    return subprocess.run(
        [str(path), *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def assert_ok(name, result, stdout=None):
    if result.returncode != 0:
        fail(name, f"expected success, got {result.returncode}\nstderr:\n{result.stderr}")
    if stdout is not None and result.stdout != stdout:
        fail(name, f"unexpected stdout\nexpected:\n{stdout!r}\nactual:\n{result.stdout!r}")
    print(f"ok {name}")


def assert_fail(name, result, stderr_contains=None):
    if result.returncode == 0:
        fail(name, f"expected failure\nstdout:\n{result.stdout}")
    if stderr_contains is not None and stderr_contains not in result.stderr:
        fail(name, f"stderr did not contain {stderr_contains!r}\nstderr:\n{result.stderr}")
    print(f"ok {name}")


def fail(name, message):
    print(f"not ok {name}: {message}", file=sys.stderr)
    raise SystemExit(1)


def write_temp(source):
    tmp = tempfile.NamedTemporaryFile("w", suffix=".imm", encoding="utf-8", delete=False)
    with tmp:
        tmp.write(source)
    return tmp.name


def write_temp_project(files, main_name="main.imm"):
    tmp = tempfile.TemporaryDirectory()
    root = Path(tmp.name)
    for name, source in files.items():
        (root / name).write_text(source, encoding="utf-8")
    return tmp, str(root / main_name)


def main():
    assert_ok("version", run_imm("--version"), "insane marmot matrix 0.1.0\n")
    assert_ok("hello", run_imm("run", "examples/hello.imm"), "Hello, insane marmot matrix!\n")
    assert_ok("matrix", run_imm("run", "examples/matrix.imm"), "[(1,0), (2,1), (1,2), (0,1)]\n")
    assert_ok("path", run_imm("run", "examples/path.imm"), "[(0,0), (1,0), (2,0), (2,1), (2,2)]\n")
    assert_ok("module", run_imm("run", "examples/use_module.imm"), "21\n")
    assert_ok("objects", run_imm("run", "examples/objects.imm"), "marmot digs UP\nsusu: 80\n")
    assert_ok("chaser runtime", run_imm("run", "examples/chaser_runtime.imm"))
    assert_ok("store example", run_imm("run", "examples/store.imm"), "susu: 80\n1\n")
    assert_ok("check", run_imm("check", "examples/chaser.imm"), "OK\n")
    assert_ok("spec json", run_imm("spec", "--json"))

    howl_async = write_temp(
        """howl dig load() -> String {
    return "ok"
}

howl marmot main {
    let task = scatter load()
    squeak wait task
}
"""
    )
    assert_ok("check howl async", run_imm("check", howl_async), "OK\n")
    assert_ok("run howl async", run_imm("run", howl_async), "ok\n")

    bad_wait = write_temp(
        """marmot main {
    wait nap(10)
}
"""
    )
    assert_fail("check wait outside howl", run_imm("check", bad_wait), "wait can only be used inside howl context")

    nest_nap = write_temp(
        """howl dig work(x: Int) -> Int {
    wait nap(1)
    return x * 2
}

howl marmot main {
    let group = nest {
        scatter work(1)
        scatter work(2)
    }
    squeak wait group
    squeak tick.now() > 0
}
"""
    )
    assert_ok("nest nap tick", run_imm("run", nest_nap), "[2, 4]\ntrue\n")

    web_grab = write_temp(
        """use web

marmot main {
    let res = web.grab("data:application/json,%7B%22name%22%3A%22marmot%22%7D")
    squeak res.status
    squeak res.ok
    squeak res.json()["name"]
}
"""
    )
    assert_ok("web grab response", run_imm("run", web_grab), "200\ntrue\nmarmot\n")

    web_options = write_temp(
        """use web

marmot main {
    let res = web.grab({
        "url": "data:text/plain,option-ok",
        "timeout_ms": 1000
    })
    squeak res.text()
}
"""
    )
    assert_ok("web grab options", run_imm("run", web_options), "option-ok\n")

    web_fetch = write_temp(
        """use web

howl marmot main {
    let res = wait web.fetch("data:text/plain,async-ok")
    squeak res.status
    squeak res.text()
}
"""
    )
    assert_ok("web fetch task", run_imm("run", web_fetch), "200\nasync-ok\n")

    trace_file = write_temp(
        """marmot main {
    let x = 10
    trace x
    squeak x
}
"""
    )
    assert_ok("trace disabled stdout", run_imm("run", trace_file), "10\n")
    traced = run_imm("run", trace_file, "--trace")
    assert_ok("trace enabled stdout", traced, "10\n")
    if "[trace] 10" not in traced.stderr:
        fail("trace enabled stderr", f"missing trace output\nstderr:\n{traced.stderr}")

    probe_file = write_temp(
        """probe "add" {
    expect 1 + 1 == 2
}
"""
    )
    assert_ok("probe pass", run_imm("probe", probe_file), f"PASS {probe_file}: add\nprobe: 1 passed, 0 failed\n")

    failing_probe = write_temp(
        """probe "fail" {
    expect false
}
"""
    )
    assert_fail("probe fail", run_imm("probe", failing_probe), "FAIL")
    assert_ok("law suite", run_imm("law"))

    with tempfile.TemporaryDirectory() as pack_tmp:
        artifact = str(Path(pack_tmp) / "hello.pyz")
        assert_ok("pack python pelt", run_imm("pack", "examples/hello.imm", "--crate", artifact, "--pelt", "python"), f"{artifact}\n")
        assert_ok("run packed python pelt", run_python(artifact), "Hello, insane marmot matrix!\n")

    with tempfile.TemporaryDirectory() as pack_tmp:
        artifact = str(Path(pack_tmp) / "hello-native")
        assert_ok("pack native pelt", run_imm("pack", "examples/hello.imm", "--crate", artifact, "--pelt", "native"), f"{artifact}\n")
        assert_ok("run packed native pelt", run_artifact(artifact), "Hello, insane marmot matrix!\n")

    pack_config_tmp, pack_config_file = write_temp_project(
        {
            "pack.imm": """pack {
    entry "main.imm"
    crate "app.pyz"
    pelt "python"
}
""",
            "main.imm": """marmot main {
    squeak "packed-config"
}
""",
        },
        main_name="pack.imm",
    )
    try:
        artifact = str(Path(pack_config_file).with_name("app.pyz").resolve())
        assert_ok("pack config", run_imm("pack", pack_config_file), f"{artifact}\n")
        assert_ok("run pack config artifact", run_python(artifact), "packed-config\n")
    finally:
        pack_config_tmp.cleanup()

    bad_array = write_temp('let nums: Array<Int> = [1, "x"]\n')
    assert_fail("check generic array type", run_imm("check", bad_array), "nums[1] must be Int")

    bad_matrix = write_temp(
        """marmot main {
    let field: Matrix<Int> = matrix [
        [1, 2],
        [3, "x"]
    ]
}
"""
    )
    assert_fail("run generic matrix type", run_imm("run", bad_matrix), "field[1, 1] must be Int")

    bad_stash = write_temp(
        """marmot main {
    stash LIMIT = 5
    LIMIT = 6
}
"""
    )
    assert_fail("stash assignment", run_imm("run", bad_stash), "LIMIT is a stash constant")

    choose_empty = write_temp(
        """marmot main {
    let values = []
    squeak insane choose values
}
"""
    )
    assert_ok("insane choose empty", run_imm("run", choose_empty), "null\n")

    private_field = write_temp(
        """den Player {
    fang let hp: Int = 100
}

marmot main {
    let p = hatch Player()
    squeak p.hp
}
"""
    )
    assert_fail("private field access", run_imm("run", private_field), "Player.hp is private")

    missing_mask_method = write_temp(
        """mask Movable {
    dig move(dir: String) -> Void
}

den Player wear Movable {
    fur let name: String = "marmot"
}
"""
    )
    assert_fail("wear missing method", run_imm("check", missing_mask_method), "does not implement move")

    uninitialized_field = write_temp(
        """den BadBox {
    fur let x: Int
}

marmot main {
    let b = hatch BadBox()
}
"""
    )
    assert_fail("uninitialized den field", run_imm("run", uninitialized_field), "BadBox.x is not initialized")

    object_type = write_temp(
        """den Player {
    fur let name: String = "marmot"
}

den Boss under Player {
    fur let phase: Int = 1
}

marmot main {
    let p: Player = hatch Boss()
    squeak p.name
}
"""
    )
    assert_ok("object subtype annotation", run_imm("run", object_type), "marmot\n")

    mask_view = write_temp(
        """mask Movable {
    dig move(dir: String) -> Void
}

den Player wear Movable {
    fur dig move(dir: String) -> Void {
        squeak dir
    }

    fur dig status() {
        squeak "visible only as Player"
    }
}

marmot main {
    let m: Movable = hatch Player()
    m.status()
}
"""
    )
    assert_fail("mask typed view restriction", run_imm("run", mask_view), "mask Movable has no member status")

    top_level_panic = write_temp(
        """panic "check must not execute this"

marmot main {
    squeak "run would execute main"
}
"""
    )
    assert_ok("check does not execute top-level statements", run_imm("check", top_level_panic), "OK\n")

    cycle_tmp, cycle_main = write_temp_project(
        {
            "main.imm": "use a\n\nmarmot main {}\n",
            "a.imm": "use b\nburrow a\n",
            "b.imm": "use a\nburrow b\n",
        }
    )
    try:
        assert_fail("module cycle detection", run_imm("check", cycle_main), "cyclic module import")
    finally:
        cycle_tmp.cleanup()

    bad_if = write_temp(
        """marmot main {
    if 1 {
        squeak "bad"
    }
}
"""
    )
    assert_fail("check non-bool if", run_imm("check", bad_if), "if condition must be Bool")

    bad_return = write_temp(
        """dig f() -> Int {
    return "bad"
}

marmot main {}
"""
    )
    assert_fail("check return type", run_imm("check", bad_return), "return value must be Int")

    bad_method_private = write_temp(
        """den Player {
    fang let hp: Int = 100
}

den Viewer {
    fur dig show(p: Player) {
        squeak p.hp
    }
}

marmot main {}
"""
    )
    assert_fail("check private object member", run_imm("check", bad_method_private), "Player.hp is private")

    mask_view_check = write_temp(
        """mask Movable {
    dig move(dir: String) -> Void
}

den Player wear Movable {
    fur dig move(dir: String) -> Void {}
    fur dig status() {}
}

marmot main {
    let m: Movable = hatch Player()
    m.status()
}
"""
    )
    assert_fail("check mask typed view restriction", run_imm("check", mask_view_check), "mask Movable has no member status")

    unformatted = write_temp(
        """marmot main {
squeak "hi"
if true {
squeak "nested"
}
}
"""
    )
    assert_fail("fmt check detects unformatted", run_imm("fmt", "--check", unformatted), "is not formatted")
    assert_ok("fmt rewrites file", run_imm("fmt", unformatted))
    assert_ok("fmt check passes formatted", run_imm("fmt", "--check", unformatted), "OK\n")
    formatted_text = Path(unformatted).read_text(encoding="utf-8")
    expected_formatted = 'marmot main {\n    squeak "hi"\n    if true {\n        squeak "nested"\n    }\n}\n'
    if formatted_text != expected_formatted:
        fail("fmt output", f"unexpected formatted text\n{formatted_text!r}")

    insane_oob = write_temp(
        """marmot main {
    let values = [1, 2, 3]
    insane {
        squeak values[99]
        values[99] = 100
    }
    squeak values.len()
}
"""
    )
    assert_ok("insane out-of-bounds is relaxed", run_imm("run", insane_oob), "null\n3\n")

    chaser_direction = write_temp(
        """use chaser

marmot main {
    squeak chaser.direction(@point(1, 1), @point(2, 1))
    squeak chaser.step(@point(1, 1), "UP")
}
"""
    )
    assert_ok("chaser helpers", run_imm("run", chaser_direction), "RIGHT\n(1,0)\n")

    store_tmp, store_main = write_temp_project(
        {
            "main.imm": """use store

den Player {
    fur let name: String
    fang let hp: Int = 100

    fur dig init(name: String) {
        self.name = name
    }

    fur dig damage(amount: Int) {
        self.hp = self.hp - amount
    }

    fur dig status() {
        squeak self.name + ": " + str(self.hp)
    }
}

marmot main {
    let db = store.open("players.immstore")
    store.clear(db, Player)

    let p = hatch Player("susu")
    p.damage(20)
    let id = store.save(db, p)

    let loaded: Player = store.load(db, Player, id)
    loaded.status()
    squeak store.count(db, Player)

    let found = store.get(db, Player, "name", "susu")
    found.status()

    loaded.damage(10)
    store.save(db, loaded)

    let again: Player = store.load(db, Player, id)
    again.status()

    squeak store.delete(db, Player, id)
    squeak store.count(db, Player)
}
""",
        }
    )
    try:
        assert_ok("store persistence lifecycle", run_imm("run", store_main), "susu: 80\n1\nsusu: 80\nsusu: 70\ntrue\n0\n")
        assert_ok("store check", run_imm("check", store_main), "OK\n")
    finally:
        store_tmp.cleanup()

    print("all tests passed")


if __name__ == "__main__":
    main()

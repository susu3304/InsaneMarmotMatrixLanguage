import asyncio
import contextvars
import heapq
import json
import math as py_math
import random
import sys
import time
import urllib.error
import urllib.request
from collections import deque
from dataclasses import dataclass
from pathlib import Path

from . import nodes as n
from .errors import ImmRuntimeError
from .parser import parse
from .tokenizer import tokenize


@dataclass(frozen=True)
class Point:
    x: int
    y: int

    def __add__(self, other):
        if not isinstance(other, Point):
            return NotImplemented
        return Point(self.x + other.x, self.y + other.y)

    def __str__(self):
        return f"({self.x},{self.y})"


class Matrix:
    def __init__(self, rows):
        if not isinstance(rows, list):
            raise ImmRuntimeError("matrix literal requires rows")
        width = None
        copied = []
        for row in rows:
            if not isinstance(row, list):
                raise ImmRuntimeError("matrix row must be an array")
            if width is None:
                width = len(row)
            elif len(row) != width:
                raise ImmRuntimeError("matrix rows must have the same length")
            copied.append(list(row))
        self.rows = copied

    def width(self):
        if not self.rows:
            return 0
        return len(self.rows[0])

    def height(self):
        return len(self.rows)

    def in_bounds(self, point):
        self._require_point(point)
        return 0 <= point.x < self.width() and 0 <= point.y < self.height()

    def get(self, args, unsafe=False):
        y, x = self._coords(args)
        if not (0 <= y < self.height() and 0 <= x < self.width()):
            if unsafe:
                return None
            raise ImmRuntimeError(f"matrix index out of bounds: [{y}, {x}]")
        return self.rows[y][x]

    def set(self, args, value, unsafe=False):
        y, x = self._coords(args)
        if not (0 <= y < self.height() and 0 <= x < self.width()):
            if unsafe:
                return value
            raise ImmRuntimeError(f"matrix index out of bounds: [{y}, {x}]")
        self.rows[y][x] = value
        return value

    def points(self):
        return [Point(x, y) for y in range(self.height()) for x in range(self.width())]

    def neighbors4(self, point):
        self._require_point(point)
        candidates = [
            Point(point.x, point.y - 1),
            Point(point.x + 1, point.y),
            Point(point.x, point.y + 1),
            Point(point.x - 1, point.y),
        ]
        return [p for p in candidates if self.in_bounds(p)]

    def neighbors8(self, point):
        self._require_point(point)
        candidates = [
            Point(point.x, point.y - 1),
            Point(point.x + 1, point.y - 1),
            Point(point.x + 1, point.y),
            Point(point.x + 1, point.y + 1),
            Point(point.x, point.y + 1),
            Point(point.x - 1, point.y + 1),
            Point(point.x - 1, point.y),
            Point(point.x - 1, point.y - 1),
        ]
        return [p for p in candidates if self.in_bounds(p)]

    def find(self, value):
        for point in self.points():
            if self.rows[point.y][point.x] == value:
                return point
        return None

    def find_all(self, value):
        return [point for point in self.points() if self.rows[point.y][point.x] == value]

    def _coords(self, args):
        if len(args) == 1 and isinstance(args[0], Point):
            return args[0].y, args[0].x
        if len(args) == 2:
            return self._to_int(args[0], "matrix y index"), self._to_int(args[1], "matrix x index")
        raise ImmRuntimeError("matrix index must be [y, x] or [point]")

    @staticmethod
    def _to_int(value, label):
        if type(value) is not int:
            raise ImmRuntimeError(f"{label} must be Int")
        return value

    @staticmethod
    def _require_point(value):
        if not isinstance(value, Point):
            raise ImmRuntimeError("expected Point")

    def __str__(self):
        return "matrix " + format_value(self.rows)


class Namespace:
    def __init__(self, name, values):
        self.name = name
        self.values = dict(values)

    def get(self, name):
        if name not in self.values:
            raise ImmRuntimeError(f"{self.name}.{name} is not defined")
        return self.values[name]


class BuiltinFunction:
    def __init__(self, name, func, needs_runtime=False):
        self.name = name
        self.func = func
        self.needs_runtime = needs_runtime

    def call(self, runtime, args):
        if self.needs_runtime:
            return self.func(runtime, *args)
        return self.func(*args)

    def __str__(self):
        return f"<builtin {self.name}>"


class BoundMethod:
    def __init__(self, name, func):
        self.name = name
        self.func = func

    def call(self, runtime, args):
        return self.func(*args)

    def __str__(self):
        return f"<method {self.name}>"


class Response:
    def __init__(self, status, headers, body, url):
        self.status = 200 if status is None else int(status)
        self.headers = dict(headers)
        self.body = body
        self.url = url
        self.ok = 200 <= self.status < 400

    def text(self):
        return self.body

    def json(self):
        try:
            return json.loads(self.body)
        except json.JSONDecodeError as err:
            raise ImmRuntimeError(f"invalid JSON response: {err}") from err

    def __str__(self):
        return f"<Response {self.status} {self.url}>"


class ImmTask:
    def __init__(self, factory, name="<task>", scheduled=False):
        self.factory = factory
        self.name = name
        self._task = None
        self._done = False
        self._result = None
        if scheduled:
            self.start()

    def start(self):
        if self._task is None and not self._done:
            try:
                loop = asyncio.get_running_loop()
            except RuntimeError as err:
                raise ImmRuntimeError("scatter requires a running howl context") from err
            self._task = loop.create_task(self.factory())
        return self

    async def wait(self):
        if self._done:
            return self._result
        if self._task is not None:
            result = await self._task
        else:
            result = await self.factory()
        self._result = result
        self._done = True
        return result

    def cancel(self):
        if self._task is not None:
            self._task.cancel()
            return True
        return False

    def __str__(self):
        return f"<task {self.name}>"


class TaskGroup:
    def __init__(self, tasks):
        self.tasks = list(tasks)

    async def wait(self):
        try:
            return [await task.wait() for task in self.tasks]
        except Exception:
            for task in self.tasks:
                task.cancel()
            raise

    def __str__(self):
        return f"<task-group {len(self.tasks)}>"


class UserFunction:
    def __init__(self, name, params, return_type, body, closure):
        self.name = name
        self.params = params
        self.return_type = return_type
        self.body = body
        self.closure = closure

    def call(self, runtime, args):
        if len(args) != len(self.params):
            raise ImmRuntimeError(f"{self.name} expects {len(self.params)} arguments, got {len(args)}")
        env = Environment(self.closure)
        for param, value in zip(self.params, args):
            check_type(value, param.type_name, f"parameter {param.name}", runtime)
            env.define(param.name, value, type_name=param.type_name, type_context=runtime)
        try:
            runtime.execute_block(self.body, env)
        except ReturnSignal as signal:
            check_type(signal.value, self.return_type, f"return value of {self.name}", runtime)
            return signal.value
        check_type(None, self.return_type, f"return value of {self.name}", runtime)
        return None

    def __str__(self):
        return f"<dig {self.name}>"


class HowlFunction(UserFunction):
    def call(self, runtime, args):
        if len(args) != len(self.params):
            raise ImmRuntimeError(f"{self.name} expects {len(self.params)} arguments, got {len(args)}")
        for param, value in zip(self.params, args):
            check_type(value, param.type_name, f"parameter {param.name}", runtime)

        async def runner():
            env = Environment(self.closure)
            for param, value in zip(self.params, args):
                env.define(param.name, value, type_name=param.type_name, type_context=runtime)
            try:
                await runtime.async_execute_block(self.body, env)
            except ReturnSignal as signal:
                check_type(signal.value, self.return_type, f"return value of {self.name}", runtime)
                return signal.value
            check_type(None, self.return_type, f"return value of {self.name}", runtime)
            return None

        return ImmTask(runner, name=self.name)

    def __str__(self):
        return f"<howl dig {self.name}>"


class LambdaFunction:
    def __init__(self, params, body, is_block, closure):
        self.params = params
        self.body = body
        self.is_block = is_block
        self.closure = closure

    def call(self, runtime, args):
        if len(args) != len(self.params):
            raise ImmRuntimeError(f"lambda expects {len(self.params)} arguments, got {len(args)}")
        env = Environment(self.closure)
        for name, value in zip(self.params, args):
            env.define(name, value)
        if self.is_block:
            try:
                runtime.execute_block(self.body, env)
            except ReturnSignal as signal:
                return signal.value
            return None
        previous = runtime.env
        runtime.env = env
        try:
            return runtime.evaluate(self.body)
        finally:
            runtime.env = previous

    def __str__(self):
        return "<lambda>"


UNINITIALIZED = object()


@dataclass
class FieldSpec:
    name: str
    type_name: str | None
    expr: object | None
    access: str
    owner: object


@dataclass
class MethodSpec:
    name: str
    params: list
    return_type: str | None
    body: list
    access: str
    owner: object
    closure: object


class MaskType:
    def __init__(self, name, methods):
        self.name = name
        self.methods = {method.name: method for method in methods}
        if len(self.methods) != len(methods):
            raise ImmRuntimeError(f"duplicate method in mask {name}")

    def __str__(self):
        return f"<mask {self.name}>"


class DenType:
    def __init__(self, name, parent_name, mask_names):
        self.name = name
        self.parent_name = parent_name
        self.mask_names = list(mask_names)
        self.parent = None
        self.fields = {}
        self.methods = {}
        self.local_fields = {}
        self.local_methods = {}
        self.validated = False

    def is_a(self, name):
        current = self
        while current is not None:
            if current.name == name:
                return True
            current = current.parent
        return False

    def wears(self, name):
        current = self
        while current is not None:
            if name in current.mask_names:
                return True
            current = current.parent
        return False

    def field_order(self):
        fields = []
        seen = set()
        current = self
        chain = []
        while current is not None:
            chain.append(current)
            current = current.parent
        for den in reversed(chain):
            for field in den.local_fields.values():
                if field.name not in seen:
                    fields.append(field)
                    seen.add(field.name)
        return fields

    def find_field(self, name):
        return self.fields.get(name)

    def find_method(self, name):
        return self.methods.get(name)

    def find_parent_method(self, name):
        if self.parent is None:
            return None
        return self.parent.find_method(name)

    def __str__(self):
        return f"<den {self.name}>"


class ObjectInstance:
    def __init__(self, den_type):
        self.den_type = den_type
        self.fields = {}
        self.store_ids = {}

    def __str__(self):
        return f"<{self.den_type.name} object>"


class ObjectView:
    def __init__(self, instance, mask):
        self.instance = instance
        self.mask = mask

    def __str__(self):
        return f"<{self.mask.name} view of {self.instance.den_type.name}>"


class ObjectBoundMethod:
    def __init__(self, instance, method):
        self.instance = instance
        self.method = method

    def call(self, runtime, args):
        return runtime._call_method(self.instance, self.method, args)

    def __str__(self):
        return f"<method {self.method.owner.name}.{self.method.name}>"


class UnderProxy:
    def __init__(self, instance, parent_type):
        self.instance = instance
        self.parent_type = parent_type

    def get(self, name):
        if self.parent_type is None:
            raise ImmRuntimeError("under has no parent den")
        if name == "init":
            method = self.parent_type.local_methods.get("init")
        else:
            method = self.parent_type.find_method(name)
        if method is None:
            raise ImmRuntimeError(f"parent den has no method {name}")
        return ObjectBoundMethod(self.instance, method)


class StoreDatabase:
    FORMAT = "IMM_STORE_V1"

    def __init__(self, path):
        self.path = Path(path).resolve()
        self.data = self._read()

    def _read(self):
        if not self.path.exists():
            return {"format": self.FORMAT, "next_id": {}, "records": {}}
        try:
            data = json.loads(self.path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as err:
            raise ImmRuntimeError(f"invalid store file {self.path}: {err}") from err
        if data.get("format") != self.FORMAT:
            raise ImmRuntimeError(f"unsupported store format in {self.path}")
        data.setdefault("next_id", {})
        data.setdefault("records", {})
        return data

    def flush(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        tmp = self.path.with_suffix(self.path.suffix + ".tmp")
        tmp.write_text(json.dumps(self.data, ensure_ascii=False, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        tmp.replace(self.path)

    def next_id(self, den_name):
        current = int(self.data["next_id"].get(den_name, 1))
        self.data["next_id"][den_name] = current + 1
        return current

    def records_for(self, den_name):
        return self.data["records"].setdefault(den_name, {})

    def __str__(self):
        return f"<store {self.path}>"


@dataclass
class Cell:
    value: object
    const: bool = False
    type_name: str | None = None


class Environment:
    def __init__(self, parent=None):
        self.parent = parent
        self.values = {}

    def define(self, name, value, const=False, type_name=None, type_context=None):
        check_type(value, type_name, name, type_context)
        self.values[name] = Cell(value, const, type_name)

    def assign(self, name, value, type_context=None):
        env = self._find_env(name)
        if env is None:
            raise ImmRuntimeError(f"{name} is not defined")
        cell = env.values[name]
        if cell.const:
            raise ImmRuntimeError(f"{name} is a stash constant")
        check_type(value, cell.type_name, name, type_context)
        cell.value = value
        return value

    def get(self, name):
        return self.get_cell(name).value

    def get_cell(self, name):
        env = self._find_env(name)
        if env is None:
            raise ImmRuntimeError(f"{name} is not defined")
        return env.values[name]

    def _find_env(self, name):
        if name in self.values:
            return self
        if self.parent is not None:
            return self.parent._find_env(name)
        return None


class ReturnSignal(Exception):
    def __init__(self, value):
        self.value = value


class BreakSignal(Exception):
    pass


class ContinueSignal(Exception):
    pass


class Runtime:
    def __init__(
        self,
        source_path=None,
        output=print,
        input_func=input,
        check_only=False,
        module_cache=None,
        module_stack=None,
        trace_enabled=False,
        trace_output=None,
    ):
        self.source_path = Path(source_path).resolve() if source_path else None
        self.output = output
        self.input_func = input_func
        self.check_only = check_only
        self.module_cache = module_cache if module_cache is not None else {}
        self.module_stack = module_stack if module_stack is not None else []
        self.trace_enabled = trace_enabled
        self.trace_output = trace_output if trace_output is not None else (lambda value: print(value, file=sys.stderr))
        self._env_var = contextvars.ContextVar(f"imm_env_{id(self)}")
        self._howl_depth_var = contextvars.ContextVar(f"imm_howl_depth_{id(self)}", default=0)
        self._probe_stack_var = contextvars.ContextVar(f"imm_probe_stack_{id(self)}", default=())
        self.env = Environment()
        self.insane_depth = 0
        self.dens = {}
        self.masks = {}
        self.current_den = []
        self._install_core()

    @property
    def env(self):
        return self._env_var.get()

    @env.setter
    def env(self, value):
        self._env_var.set(value)

    @property
    def howl_depth(self):
        return self._howl_depth_var.get()

    @howl_depth.setter
    def howl_depth(self, value):
        self._howl_depth_var.set(value)

    @property
    def probe_stack(self):
        return self._probe_stack_var.get()

    @probe_stack.setter
    def probe_stack(self, value):
        self._probe_stack_var.set(tuple(value))

    def prepare(self, program):
        main_def = None
        howl_main_def = None
        for item in program.items:
            if isinstance(item, n.UseStmt):
                self.env.define(item.name, self._load_namespace(item.name), const=True)
        for item in program.items:
            if isinstance(item, n.MaskDef):
                self._register_mask(item)
        for item in program.items:
            if isinstance(item, n.DenDef):
                self._register_den(item)
        self._validate_dens()
        for item in program.items:
            if isinstance(item, n.FunctionDef):
                self.env.define(item.name, UserFunction(item.name, item.params, item.return_type, item.body, self.env), const=True)
            elif isinstance(item, n.HowlFunctionDef):
                self.env.define(item.name, HowlFunction(item.name, item.params, item.return_type, item.body, self.env), const=True)
            elif isinstance(item, n.MainDef):
                if main_def is not None:
                    raise ImmRuntimeError("duplicate marmot main")
                main_def = item
            elif isinstance(item, n.HowlMainDef):
                if howl_main_def is not None:
                    raise ImmRuntimeError("duplicate howl marmot main")
                howl_main_def = item
        if main_def is not None and howl_main_def is not None:
            raise ImmRuntimeError("cannot define both marmot main and howl marmot main")
        return howl_main_def if howl_main_def is not None else main_def

    def check(self, program):
        self.prepare(program)
        StaticChecker(self).check(program.items)

    def run(self, program, run_main=True):
        main_def = self.prepare(program)

        for item in program.items:
            if isinstance(
                item,
                (
                    n.FunctionDef,
                    n.HowlFunctionDef,
                    n.MainDef,
                    n.HowlMainDef,
                    n.UseStmt,
                    n.ModuleDef,
                    n.DenDef,
                    n.MaskDef,
                    n.ProbeDef,
                    n.PackDef,
                ),
            ):
                continue
            self.execute(item)

        if run_main:
            if main_def is None:
                raise ImmRuntimeError("marmot main is not defined")
            if isinstance(main_def, n.HowlMainDef):
                asyncio.run(self._run_howl_main(main_def))
            else:
                if main_def.insane:
                    self.insane_depth += 1
                try:
                    self.execute_block(main_def.body, Environment(self.env))
                finally:
                    if main_def.insane:
                        self.insane_depth -= 1

    def run_probe_blocks(self, program):
        self.prepare(program)
        for item in program.items:
            if isinstance(
                item,
                (
                    n.FunctionDef,
                    n.HowlFunctionDef,
                    n.MainDef,
                    n.HowlMainDef,
                    n.UseStmt,
                    n.ModuleDef,
                    n.DenDef,
                    n.MaskDef,
                    n.ProbeDef,
                    n.PackDef,
                ),
            ):
                continue
            self.execute(item)

        results = []
        for item in program.items:
            if not isinstance(item, n.ProbeDef):
                continue
            self.probe_stack = (*self.probe_stack, item.name)
            try:
                self.execute_block(item.body, Environment(self.env))
            except ImmRuntimeError as err:
                results.append((item.name, False, str(err)))
            else:
                results.append((item.name, True, None))
            finally:
                self.probe_stack = self.probe_stack[:-1]
        return results

    def _expect_message(self):
        if self.probe_stack:
            return f"expect failed in probe {self.probe_stack[-1]}"
        return "expect failed"

    async def _run_howl_main(self, main_def):
        if main_def.insane:
            self.insane_depth += 1
        self.howl_depth = self.howl_depth + 1
        try:
            await self.async_execute_block(main_def.body, Environment(self.env))
        finally:
            self.howl_depth = self.howl_depth - 1
            if main_def.insane:
                self.insane_depth -= 1

    async def async_execute_block(self, statements, env):
        previous = self.env
        self.env = env
        try:
            for stmt in statements:
                await self.async_execute(stmt)
        finally:
            self.env = previous

    async def async_execute(self, stmt):
        if isinstance(stmt, n.LetStmt):
            self.env.define(stmt.name, await self.async_evaluate(stmt.expr), stmt.const, stmt.type_name, type_context=self)
            return
        if isinstance(stmt, n.ExprStmt):
            await self.async_evaluate(stmt.expr)
            return
        if isinstance(stmt, n.SqueakStmt):
            values = [format_value(await self.async_evaluate(expr)) for expr in stmt.exprs]
            self.output(" ".join(values))
            return
        if isinstance(stmt, n.PanicStmt):
            raise ImmRuntimeError(format_value(await self.async_evaluate(stmt.expr)))
        if isinstance(stmt, n.ExpectStmt):
            value = await self.async_evaluate(stmt.expr)
            if type(value) is not bool:
                raise ImmRuntimeError("expect expression must be Bool")
            if not value:
                raise ImmRuntimeError(self._expect_message())
            return
        if isinstance(stmt, n.TraceStmt):
            if self.trace_enabled:
                values = [format_value(await self.async_evaluate(expr)) for expr in stmt.exprs]
                payload = " ".join(values)
                self.trace_output(f"[trace] {payload}" if payload else "[trace]")
            return
        if isinstance(stmt, n.IfStmt):
            if self._require_bool(await self.async_evaluate(stmt.condition), "if condition"):
                await self.async_execute_block(stmt.then_body, Environment(self.env))
            elif stmt.else_body is not None:
                if isinstance(stmt.else_body, n.IfStmt):
                    await self.async_execute(stmt.else_body)
                else:
                    await self.async_execute_block(stmt.else_body, Environment(self.env))
            return
        if isinstance(stmt, n.WhileStmt):
            while self._require_bool(await self.async_evaluate(stmt.condition), "while condition"):
                try:
                    await self.async_execute_block(stmt.body, Environment(self.env))
                except ContinueSignal:
                    continue
                except BreakSignal:
                    break
            return
        if isinstance(stmt, n.ForStmt):
            iterable = await self.async_evaluate(stmt.iterable)
            if iterable is None:
                raise ImmRuntimeError("cannot iterate null")
            if stmt.insane:
                self.insane_depth += 1
            try:
                values = list(iterable)
                if stmt.insane:
                    random.shuffle(values)
                for value in values:
                    loop_env = Environment(self.env)
                    loop_env.define(stmt.name, value, type_context=self)
                    try:
                        await self.async_execute_block(stmt.body, loop_env)
                    except ContinueSignal:
                        continue
                    except BreakSignal:
                        break
            finally:
                if stmt.insane:
                    self.insane_depth -= 1
            return
        if isinstance(stmt, n.ReturnStmt):
            raise ReturnSignal(await self.async_evaluate(stmt.expr) if stmt.expr is not None else None)
        if isinstance(stmt, n.BreakStmt):
            raise BreakSignal()
        if isinstance(stmt, n.ContinueStmt):
            raise ContinueSignal()
        if isinstance(stmt, n.TryStmt):
            try:
                await self.async_execute_block(stmt.body, Environment(self.env))
            except ImmRuntimeError as err:
                if stmt.insane and stmt.catch_body is None:
                    return
                if stmt.catch_body is None:
                    raise
                catch_env = Environment(self.env)
                catch_env.define(stmt.catch_name, str(err), type_context=self)
                await self.async_execute_block(stmt.catch_body, catch_env)
            return
        if isinstance(stmt, n.InsaneBlock):
            self.insane_depth += 1
            try:
                await self.async_execute_block(stmt.body, Environment(self.env))
            finally:
                self.insane_depth -= 1
            return
        if isinstance(
            stmt,
            (
                n.UseStmt,
                n.ModuleDef,
                n.FunctionDef,
                n.HowlFunctionDef,
                n.MainDef,
                n.HowlMainDef,
                n.ProbeDef,
                n.PackDef,
            ),
        ):
            return
        raise ImmRuntimeError(f"unknown statement {type(stmt).__name__}")

    async def async_evaluate(self, expr):
        if isinstance(expr, n.Literal):
            return expr.value
        if isinstance(expr, n.Var):
            return self._cell_value(self.env.get_cell(expr.name))
        if isinstance(expr, n.ArrayLiteral):
            return [await self.async_evaluate(item) for item in expr.items]
        if isinstance(expr, n.MapLiteral):
            result = {}
            for key_expr, value_expr in expr.pairs:
                key = await self.async_evaluate(key_expr)
                if not isinstance(key, str):
                    raise ImmRuntimeError("map literal keys must be String")
                result[key] = await self.async_evaluate(value_expr)
            return result
        if isinstance(expr, n.MatrixLiteral):
            return Matrix([await self.async_evaluate(row) for row in expr.rows])
        if isinstance(expr, n.PointLiteral):
            x = await self.async_evaluate(expr.x)
            y = await self.async_evaluate(expr.y)
            if type(x) is not int or type(y) is not int:
                raise ImmRuntimeError("@point requires Int x and y")
            return Point(x, y)
        if isinstance(expr, n.HatchExpr):
            args = [await self.async_evaluate(arg) for arg in expr.args]
            return self._hatch(expr.name, args)
        if isinstance(expr, n.SniffExpr):
            return self.input_func()
        if isinstance(expr, n.Unary):
            value = await self.async_evaluate(expr.expr)
            if expr.op == "-":
                return -value
            if expr.op == "!":
                return not self._require_bool(value, "! operand")
            raise ImmRuntimeError(f"unknown unary operator {expr.op}")
        if isinstance(expr, n.Binary):
            return await self._async_binary(expr)
        if isinstance(expr, n.RangeExpr):
            start = await self.async_evaluate(expr.start)
            end = await self.async_evaluate(expr.end)
            if type(start) is not int or type(end) is not int:
                raise ImmRuntimeError("range bounds must be Int")
            return range(start, end)
        if isinstance(expr, n.Call):
            callee = await self.async_evaluate(expr.callee)
            args = [await self.async_evaluate(arg) for arg in expr.args]
            return await self.async_call_value(callee, args)
        if isinstance(expr, n.Index):
            target = await self.async_evaluate(expr.target)
            args = [await self.async_evaluate(arg) for arg in expr.args]
            return self._get_index(target, args)
        if isinstance(expr, n.Member):
            return self._get_member(await self.async_evaluate(expr.target), expr.name)
        if isinstance(expr, n.Assign):
            value = await self.async_evaluate(expr.value)
            return self._assign_async_target(expr.target, value)
        if isinstance(expr, n.LambdaExpr):
            return LambdaFunction(expr.params, expr.body, expr.is_block, self.env)
        if isinstance(expr, n.TunnelExpr):
            value = await self.async_evaluate(expr.left)
            return await self._async_eval_tunnel(value, expr.right)
        if isinstance(expr, n.InsaneChoose):
            values = await self.async_evaluate(expr.expr)
            if values is None or len(values) == 0:
                return None
            return random.choice(list(values))
        if isinstance(expr, n.WaitExpr):
            return await self._await_value(await self.async_evaluate(expr.expr))
        if isinstance(expr, n.ScatterExpr):
            return self._scatter(expr.expr, expr.insane)
        if isinstance(expr, n.NestExpr):
            return TaskGroup([self._scatter(item.expr, item.insane) for item in expr.items])
        raise ImmRuntimeError(f"unknown expression {type(expr).__name__}")

    async def async_call_value(self, callee, args):
        return self.call_value(callee, args)

    async def _await_value(self, value):
        if isinstance(value, TaskGroup):
            return await value.wait()
        if isinstance(value, ImmTask):
            return await value.wait()
        raise ImmRuntimeError(f"wait expects Task, got {type_name(value)}")

    def _scatter(self, expr, insane=False):
        captured_env = self.env

        async def runner():
            previous = self.env
            self.env = captured_env
            if insane:
                self.insane_depth += 1
            try:
                value = await self.async_evaluate(expr)
                if isinstance(value, (ImmTask, TaskGroup)):
                    return await self._await_value(value)
                return value
            finally:
                if insane:
                    self.insane_depth -= 1
                self.env = previous

        return ImmTask(runner, name="scatter", scheduled=True)

    async def _async_binary(self, expr):
        if expr.op == "&&":
            left = self._require_bool(await self.async_evaluate(expr.left), "left side of &&")
            if not left:
                return False
            return self._require_bool(await self.async_evaluate(expr.right), "right side of &&")
        if expr.op == "||":
            left = self._require_bool(await self.async_evaluate(expr.left), "left side of ||")
            if left:
                return True
            return self._require_bool(await self.async_evaluate(expr.right), "right side of ||")
        left = await self.async_evaluate(expr.left)
        right = await self.async_evaluate(expr.right)
        return self._apply_binary(expr.op, left, right)

    async def _async_eval_tunnel(self, value, right):
        if isinstance(right, n.Call):
            callee = await self.async_evaluate(right.callee)
            args = [value] + [await self.async_evaluate(arg) for arg in right.args]
            return await self.async_call_value(callee, args)
        callee = await self.async_evaluate(right)
        return await self.async_call_value(callee, [value])

    def _assign_async_target(self, target, value):
        return self._assign(target, value)

    def _register_mask(self, item):
        if item.name in self.masks or item.name in self.dens:
            raise ImmRuntimeError(f"type {item.name} is already defined")
        mask = MaskType(item.name, item.methods)
        self.masks[item.name] = mask
        self.env.define(item.name, mask, const=True)

    def _register_den(self, item):
        if item.name in self.dens or item.name in self.masks:
            raise ImmRuntimeError(f"type {item.name} is already defined")
        den_type = DenType(item.name, item.parent, item.masks)
        for member in item.members:
            if isinstance(member, n.FieldDef):
                if member.name in den_type.local_fields:
                    raise ImmRuntimeError(f"duplicate field {item.name}.{member.name}")
                den_type.local_fields[member.name] = FieldSpec(
                    member.name,
                    member.type_name,
                    member.expr,
                    member.access,
                    den_type,
                )
            elif isinstance(member, n.MethodDef):
                if member.name in den_type.local_methods:
                    raise ImmRuntimeError(f"duplicate method {item.name}.{member.name}")
                if member.name == "init" and member.return_type is not None:
                    raise ImmRuntimeError(f"{item.name}.init cannot declare a return type")
                den_type.local_methods[member.name] = MethodSpec(
                    member.name,
                    member.params,
                    member.return_type,
                    member.body,
                    member.access,
                    den_type,
                    self.env,
                )
        self.dens[item.name] = den_type
        self.env.define(item.name, den_type, const=True)

    def _validate_dens(self):
        for den_type in self.dens.values():
            if den_type.parent_name is not None:
                if den_type.parent_name not in self.dens:
                    raise ImmRuntimeError(f"parent den {den_type.parent_name} for {den_type.name} is not defined")
                den_type.parent = self.dens[den_type.parent_name]
            for mask_name in den_type.mask_names:
                if mask_name not in self.masks:
                    raise ImmRuntimeError(f"mask {mask_name} for {den_type.name} is not defined")
        for den_type in self.dens.values():
            self._validate_den(den_type, [])

    def _validate_den(self, den_type, seen):
        if den_type.validated:
            return
        if den_type.name in seen:
            raise ImmRuntimeError(f"cyclic den inheritance involving {den_type.name}")
        if den_type.parent is not None:
            self._validate_den(den_type.parent, seen + [den_type.name])
            den_type.fields = dict(den_type.parent.fields)
            den_type.methods = dict(den_type.parent.methods)
            for name, method in den_type.local_methods.items():
                parent_method = den_type.parent.find_method(name)
                if name != "init" and parent_method is not None and not same_signature(method, parent_method):
                    raise ImmRuntimeError(f"{den_type.name}.{name} does not match parent method signature")
        for name, field in den_type.local_fields.items():
            if den_type.parent is not None and name in den_type.parent.fields:
                raise ImmRuntimeError(f"{den_type.name}.{name} cannot redeclare inherited field")
            den_type.fields[name] = field
        for name, method in den_type.local_methods.items():
            if name == "init":
                continue
            den_type.methods[name] = method
        for mask_name in den_type.mask_names:
            mask = self.masks[mask_name]
            for name, required in mask.methods.items():
                method = den_type.find_method(name)
                if method is None:
                    raise ImmRuntimeError(f"{den_type.name} wears {mask_name} but does not implement {name}")
                if not same_signature(method, required):
                    raise ImmRuntimeError(f"{den_type.name}.{name} does not match mask {mask_name}.{name}")
        den_type.validated = True

    def execute_block(self, statements, env):
        previous = self.env
        self.env = env
        try:
            for stmt in statements:
                self.execute(stmt)
        finally:
            self.env = previous

    def execute(self, stmt):
        if isinstance(stmt, n.LetStmt):
            self.env.define(stmt.name, self.evaluate(stmt.expr), stmt.const, stmt.type_name, type_context=self)
            return
        if isinstance(stmt, n.ExprStmt):
            self.evaluate(stmt.expr)
            return
        if isinstance(stmt, n.SqueakStmt):
            values = [format_value(self.evaluate(expr)) for expr in stmt.exprs]
            self.output(" ".join(values))
            return
        if isinstance(stmt, n.PanicStmt):
            raise ImmRuntimeError(format_value(self.evaluate(stmt.expr)))
        if isinstance(stmt, n.ExpectStmt):
            value = self.evaluate(stmt.expr)
            if type(value) is not bool:
                raise ImmRuntimeError("expect expression must be Bool")
            if not value:
                raise ImmRuntimeError(self._expect_message())
            return
        if isinstance(stmt, n.TraceStmt):
            if self.trace_enabled:
                values = [format_value(self.evaluate(expr)) for expr in stmt.exprs]
                payload = " ".join(values)
                self.trace_output(f"[trace] {payload}" if payload else "[trace]")
            return
        if isinstance(stmt, n.IfStmt):
            if self._require_bool(self.evaluate(stmt.condition), "if condition"):
                self.execute_block(stmt.then_body, Environment(self.env))
            elif stmt.else_body is not None:
                if isinstance(stmt.else_body, n.IfStmt):
                    self.execute(stmt.else_body)
                else:
                    self.execute_block(stmt.else_body, Environment(self.env))
            return
        if isinstance(stmt, n.WhileStmt):
            while self._require_bool(self.evaluate(stmt.condition), "while condition"):
                try:
                    self.execute_block(stmt.body, Environment(self.env))
                except ContinueSignal:
                    continue
                except BreakSignal:
                    break
            return
        if isinstance(stmt, n.ForStmt):
            iterable = self.evaluate(stmt.iterable)
            if iterable is None:
                raise ImmRuntimeError("cannot iterate null")
            if stmt.insane:
                self.insane_depth += 1
            try:
                values = list(iterable)
                if stmt.insane:
                    random.shuffle(values)
                for value in values:
                    loop_env = Environment(self.env)
                    loop_env.define(stmt.name, value, type_context=self)
                    try:
                        self.execute_block(stmt.body, loop_env)
                    except ContinueSignal:
                        continue
                    except BreakSignal:
                        break
            finally:
                if stmt.insane:
                    self.insane_depth -= 1
            return
        if isinstance(stmt, n.ReturnStmt):
            raise ReturnSignal(self.evaluate(stmt.expr) if stmt.expr is not None else None)
        if isinstance(stmt, n.BreakStmt):
            raise BreakSignal()
        if isinstance(stmt, n.ContinueStmt):
            raise ContinueSignal()
        if isinstance(stmt, n.TryStmt):
            try:
                self.execute_block(stmt.body, Environment(self.env))
            except ImmRuntimeError as err:
                if stmt.insane and stmt.catch_body is None:
                    return
                if stmt.catch_body is None:
                    raise
                catch_env = Environment(self.env)
                catch_env.define(stmt.catch_name, str(err), type_context=self)
                self.execute_block(stmt.catch_body, catch_env)
            return
        if isinstance(stmt, n.InsaneBlock):
            self.insane_depth += 1
            try:
                self.execute_block(stmt.body, Environment(self.env))
            finally:
                self.insane_depth -= 1
            return
        if isinstance(
            stmt,
            (
                n.UseStmt,
                n.ModuleDef,
                n.FunctionDef,
                n.HowlFunctionDef,
                n.MainDef,
                n.HowlMainDef,
                n.ProbeDef,
                n.PackDef,
            ),
        ):
            return
        raise ImmRuntimeError(f"unknown statement {type(stmt).__name__}")

    def evaluate(self, expr):
        if isinstance(expr, n.Literal):
            return expr.value
        if isinstance(expr, n.Var):
            return self._cell_value(self.env.get_cell(expr.name))
        if isinstance(expr, n.ArrayLiteral):
            return [self.evaluate(item) for item in expr.items]
        if isinstance(expr, n.MapLiteral):
            result = {}
            for key_expr, value_expr in expr.pairs:
                key = self.evaluate(key_expr)
                if not isinstance(key, str):
                    raise ImmRuntimeError("map literal keys must be String")
                result[key] = self.evaluate(value_expr)
            return result
        if isinstance(expr, n.MatrixLiteral):
            return Matrix([self.evaluate(row) for row in expr.rows])
        if isinstance(expr, n.PointLiteral):
            x = self.evaluate(expr.x)
            y = self.evaluate(expr.y)
            if type(x) is not int or type(y) is not int:
                raise ImmRuntimeError("@point requires Int x and y")
            return Point(x, y)
        if isinstance(expr, n.HatchExpr):
            args = [self.evaluate(arg) for arg in expr.args]
            return self._hatch(expr.name, args)
        if isinstance(expr, n.SniffExpr):
            return self.input_func()
        if isinstance(expr, n.Unary):
            value = self.evaluate(expr.expr)
            if expr.op == "-":
                return -value
            if expr.op == "!":
                return not self._require_bool(value, "! operand")
            raise ImmRuntimeError(f"unknown unary operator {expr.op}")
        if isinstance(expr, n.Binary):
            return self._binary(expr)
        if isinstance(expr, n.RangeExpr):
            start = self.evaluate(expr.start)
            end = self.evaluate(expr.end)
            if type(start) is not int or type(end) is not int:
                raise ImmRuntimeError("range bounds must be Int")
            return range(start, end)
        if isinstance(expr, n.Call):
            callee = self.evaluate(expr.callee)
            args = [self.evaluate(arg) for arg in expr.args]
            return self.call_value(callee, args)
        if isinstance(expr, n.Index):
            target = self.evaluate(expr.target)
            args = [self.evaluate(arg) for arg in expr.args]
            return self._get_index(target, args)
        if isinstance(expr, n.Member):
            return self._get_member(self.evaluate(expr.target), expr.name)
        if isinstance(expr, n.Assign):
            value = self.evaluate(expr.value)
            return self._assign(expr.target, value)
        if isinstance(expr, n.LambdaExpr):
            return LambdaFunction(expr.params, expr.body, expr.is_block, self.env)
        if isinstance(expr, n.TunnelExpr):
            value = self.evaluate(expr.left)
            return self._eval_tunnel(value, expr.right)
        if isinstance(expr, n.InsaneChoose):
            values = self.evaluate(expr.expr)
            if values is None or len(values) == 0:
                return None
            return random.choice(list(values))
        if isinstance(expr, n.WaitExpr):
            raise ImmRuntimeError("wait can only be used inside howl context")
        if isinstance(expr, n.ScatterExpr):
            raise ImmRuntimeError("scatter can only be used inside howl context")
        if isinstance(expr, n.NestExpr):
            raise ImmRuntimeError("nest can only be used inside howl context")
        raise ImmRuntimeError(f"unknown expression {type(expr).__name__}")

    def call_value(self, callee, args):
        if hasattr(callee, "call"):
            return callee.call(self, args)
        if callable(callee):
            return callee(*args)
        raise ImmRuntimeError(f"{format_value(callee)} is not callable")

    def _eval_tunnel(self, value, right):
        if isinstance(right, n.Call):
            callee = self.evaluate(right.callee)
            args = [value] + [self.evaluate(arg) for arg in right.args]
            return self.call_value(callee, args)
        callee = self.evaluate(right)
        return self.call_value(callee, [value])

    def _assign(self, target, value):
        if isinstance(target, n.Var):
            return self.env.assign(target.name, value, type_context=self)
        if isinstance(target, n.Index):
            obj = self.evaluate(target.target)
            args = [self.evaluate(arg) for arg in target.args]
            return self._set_index(obj, args, value)
        if isinstance(target, n.Member):
            obj = self.evaluate(target.target)
            return self._set_member(obj, target.name, value)
        raise ImmRuntimeError("invalid assignment target")

    def _cell_value(self, cell):
        return self._apply_type_view(cell.value, cell.type_name)

    def _apply_type_view(self, value, type_name_hint):
        if isinstance(value, ObjectInstance) and type_name_hint is not None:
            type_ref = parse_type_ref(type_name_hint)
            if type_ref.name in self.masks:
                return ObjectView(value, self.masks[type_ref.name])
        return value

    def _hatch(self, name, args):
        if name not in self.dens:
            raise ImmRuntimeError(f"den {name} is not defined")
        den_type = self.dens[name]
        instance = ObjectInstance(den_type)
        self._initialize_fields(instance)
        init_method = den_type.local_methods.get("init")
        if init_method is None:
            if args:
                raise ImmRuntimeError(f"{name} has no init and expects 0 arguments")
        else:
            self._call_method(instance, init_method, args)
        self._check_initialized_fields(instance)
        return instance

    def _initialize_fields(self, instance):
        for field in instance.den_type.field_order():
            instance.fields[field.name] = UNINITIALIZED
            if field.expr is not None:
                env = Environment(self.env)
                env.define("self", instance, const=True, type_context=self)
                previous = self.env
                self.env = env
                self.current_den.append(field.owner)
                try:
                    value = self.evaluate(field.expr)
                finally:
                    self.current_den.pop()
                    self.env = previous
                if field.type_name is None:
                    field.type_name = type_name(value)
                check_type(value, field.type_name, f"{field.owner.name}.{field.name}", self)
                instance.fields[field.name] = value

    def _check_initialized_fields(self, instance):
        for field in instance.den_type.field_order():
            if instance.fields.get(field.name, UNINITIALIZED) is UNINITIALIZED:
                raise ImmRuntimeError(f"{instance.den_type.name}.{field.name} is not initialized")

    def _call_method(self, instance, method, args):
        if len(args) != len(method.params):
            raise ImmRuntimeError(f"{method.owner.name}.{method.name} expects {len(method.params)} arguments, got {len(args)}")
        env = Environment(method.closure)
        env.define("self", instance, const=True, type_context=self)
        if method.owner.parent is not None:
            env.define("under", UnderProxy(instance, method.owner.parent), const=True, type_context=self)
        for param, value in zip(method.params, args):
            check_type(value, param.type_name, f"parameter {param.name}", self)
            env.define(param.name, value, type_name=param.type_name, type_context=self)
        self.current_den.append(method.owner)
        try:
            try:
                self.execute_block(method.body, env)
            except ReturnSignal as signal:
                if method.name == "init":
                    if signal.value is not None:
                        raise ImmRuntimeError(f"{method.owner.name}.init cannot return a value")
                    return None
                check_type(signal.value, method.return_type, f"return value of {method.owner.name}.{method.name}", self)
                return signal.value
            if method.name != "init":
                check_type(None, method.return_type, f"return value of {method.owner.name}.{method.name}", self)
            return None
        finally:
            self.current_den.pop()

    def _can_access(self, member):
        if member.access == "fur":
            return True
        if self.current_den and self.current_den[-1] is member.owner:
            return True
        return False

    def _get_object_member(self, target, name):
        field = target.den_type.find_field(name)
        if field is not None:
            if not self._can_access(field):
                raise ImmRuntimeError(f"{target.den_type.name}.{name} is private")
            value = target.fields.get(name, UNINITIALIZED)
            if value is UNINITIALIZED:
                raise ImmRuntimeError(f"{target.den_type.name}.{name} is not initialized")
            return value
        method = target.den_type.find_method(name)
        if method is not None:
            if not self._can_access(method):
                raise ImmRuntimeError(f"{target.den_type.name}.{name} is private")
            return ObjectBoundMethod(target, method)
        raise ImmRuntimeError(f"{target.den_type.name} has no member {name}")

    def _set_object_member(self, target, name, value):
        field = target.den_type.find_field(name)
        if field is None:
            raise ImmRuntimeError(f"{target.den_type.name} has no field {name}")
        if not self._can_access(field):
            raise ImmRuntimeError(f"{target.den_type.name}.{name} is private")
        check_type(value, field.type_name, f"{field.owner.name}.{field.name}", self)
        target.fields[name] = value
        return value

    def _binary(self, expr):
        if expr.op == "&&":
            left = self._require_bool(self.evaluate(expr.left), "left side of &&")
            if not left:
                return False
            return self._require_bool(self.evaluate(expr.right), "right side of &&")
        if expr.op == "||":
            left = self._require_bool(self.evaluate(expr.left), "left side of ||")
            if left:
                return True
            return self._require_bool(self.evaluate(expr.right), "right side of ||")

        left = self.evaluate(expr.left)
        right = self.evaluate(expr.right)
        return self._apply_binary(expr.op, left, right)

    def _apply_binary(self, op, left, right):
        if op == "+":
            if isinstance(left, Point) and isinstance(right, Point):
                return left + right
            if isinstance(left, str) or isinstance(right, str):
                return format_value(left) + format_value(right)
            return left + right
        if op == "-":
            return left - right
        if op == "*":
            return left * right
        if op == "/":
            return left / right
        if op == "%":
            return left % right
        if op == "==":
            return left == right
        if op == "!=":
            return left != right
        if op == "<":
            return left < right
        if op == "<=":
            return left <= right
        if op == ">":
            return left > right
        if op == ">=":
            return left >= right
        raise ImmRuntimeError(f"unknown operator {op}")

    def _get_index(self, target, args):
        if isinstance(target, Matrix):
            return target.get(args, unsafe=self.insane_depth > 0)
        if isinstance(target, list):
            if len(args) != 1 or type(args[0]) is not int:
                raise ImmRuntimeError("array index must be one Int")
            try:
                return target[args[0]]
            except IndexError as err:
                if self.insane_depth > 0:
                    return None
                raise ImmRuntimeError(f"array index out of bounds: {args[0]}") from err
        if isinstance(target, str):
            if len(args) != 1 or type(args[0]) is not int:
                raise ImmRuntimeError("string index must be one Int")
            try:
                return target[args[0]]
            except IndexError as err:
                if self.insane_depth > 0:
                    return None
                raise ImmRuntimeError(f"string index out of bounds: {args[0]}") from err
        if isinstance(target, dict):
            if len(args) != 1 or not isinstance(args[0], str):
                raise ImmRuntimeError("map index must be one String")
            try:
                return target[args[0]]
            except KeyError as err:
                if self.insane_depth > 0:
                    return None
                raise ImmRuntimeError(f"map key not found: {args[0]}") from err
        raise ImmRuntimeError(f"{type_name(target)} is not indexable")

    def _set_index(self, target, args, value):
        if isinstance(target, Matrix):
            return target.set(args, value, unsafe=self.insane_depth > 0)
        if isinstance(target, list):
            if len(args) != 1 or type(args[0]) is not int:
                raise ImmRuntimeError("array index must be one Int")
            try:
                target[args[0]] = value
            except IndexError as err:
                if self.insane_depth > 0:
                    return value
                raise ImmRuntimeError(f"array index out of bounds: {args[0]}") from err
            return value
        if isinstance(target, dict):
            if len(args) != 1 or not isinstance(args[0], str):
                raise ImmRuntimeError("map index must be one String")
            target[args[0]] = value
            return value
        raise ImmRuntimeError(f"{type_name(target)} is not assignable by index")

    def _get_member(self, target, name):
        if target is None:
            raise ImmRuntimeError("null has no members")
        if isinstance(target, Namespace):
            return target.get(name)
        if isinstance(target, Response):
            if name in ("status", "headers", "body", "url", "ok"):
                return getattr(target, name)
            methods = {
                "json": lambda: target.json(),
                "text": lambda: target.text(),
            }
            if name in methods:
                return BoundMethod(name, methods[name])
        if isinstance(target, ImmTask):
            methods = {
                "done": lambda: target._done or (target._task is not None and target._task.done()),
                "cancel": lambda: target.cancel(),
            }
            if name in methods:
                return BoundMethod(name, methods[name])
        if isinstance(target, UnderProxy):
            return target.get(name)
        if isinstance(target, ObjectView):
            return self._get_object_view_member(target, name)
        if isinstance(target, ObjectInstance):
            return self._get_object_member(target, name)
        if isinstance(target, Point):
            if name == "x":
                return target.x
            if name == "y":
                return target.y
        if isinstance(target, Matrix):
            methods = {
                "width": lambda: target.width(),
                "height": lambda: target.height(),
                "in_bounds": lambda p: target.in_bounds(p),
                "points": lambda: target.points(),
                "neighbors4": lambda p: target.neighbors4(p),
                "neighbors8": lambda p: target.neighbors8(p),
                "find": lambda v: target.find(v),
                "find_all": lambda v: target.find_all(v),
            }
            if name in methods:
                return BoundMethod(name, methods[name])
        if isinstance(target, list):
            methods = {
                "len": lambda: len(target),
            }
            if name in methods:
                return BoundMethod(name, methods[name])
        if isinstance(target, dict):
            methods = {
                "len": lambda: len(target),
            }
            if name in methods:
                return BoundMethod(name, methods[name])
        if isinstance(target, str):
            methods = {
                "len": lambda: len(target),
                "to_int": lambda: int(target),
                "to_float": lambda: float(target),
                "to_bool": lambda: target.lower() in ("true", "1", "yes"),
            }
            if name in methods:
                return BoundMethod(name, methods[name])
        raise ImmRuntimeError(f"{type_name(target)} has no member {name}")

    def _set_member(self, target, name, value):
        if target is None:
            raise ImmRuntimeError("null has no members")
        if isinstance(target, ObjectView):
            raise ImmRuntimeError(f"mask {target.mask.name} has no assignable field {name}")
        if isinstance(target, ObjectInstance):
            return self._set_object_member(target, name, value)
        raise ImmRuntimeError(f"{type_name(target)} has no assignable member {name}")

    def _get_object_view_member(self, target, name):
        if name not in target.mask.methods:
            raise ImmRuntimeError(f"mask {target.mask.name} has no member {name}")
        method = target.instance.den_type.find_method(name)
        if method is None:
            raise ImmRuntimeError(f"{target.instance.den_type.name} has no member {name}")
        if not self._can_access(method):
            raise ImmRuntimeError(f"{target.instance.den_type.name}.{name} is private")
        return ObjectBoundMethod(target.instance, method)

    @staticmethod
    def _require_bool(value, label):
        if type(value) is not bool:
            raise ImmRuntimeError(f"{label} must be Bool")
        return value

    def _install_core(self):
        builtins = {
            "len": BuiltinFunction("len", lambda x: len(x)),
            "type": BuiltinFunction("type", type_name),
            "str": BuiltinFunction("str", format_value),
            "int": BuiltinFunction("int", lambda x: int(x)),
            "float": BuiltinFunction("float", lambda x: float(x)),
            "bool": BuiltinFunction("bool", lambda x: bool(x)),
            "map": BuiltinFunction("map", builtin_map, needs_runtime=True),
            "filter": BuiltinFunction("filter", builtin_filter, needs_runtime=True),
            "reduce": BuiltinFunction("reduce", builtin_reduce, needs_runtime=True),
            "nap": BuiltinFunction("nap", builtin_nap),
        }
        for name, value in builtins.items():
            self.env.define(name, value, const=True)
        self.env.define("math", self._math_namespace(), const=True)
        self.env.define("path", self._path_namespace(), const=True)
        self.env.define("chaser", self._chaser_namespace(), const=True)
        self.env.define("store", self._store_namespace(), const=True)
        self.env.define("web", self._web_namespace(), const=True)
        self.env.define("tick", self._tick_namespace(), const=True)

    def _load_namespace(self, name):
        if name == "math":
            return self._math_namespace()
        if name == "path":
            return self._path_namespace()
        if name == "chaser":
            return self._chaser_namespace()
        if name == "store":
            return self._store_namespace()
        if name == "web":
            return self._web_namespace()
        if name == "tick":
            return self._tick_namespace()
        if self.source_path is None:
            raise ImmRuntimeError(f"cannot resolve module {name}")
        module_path = self.source_path.parent / f"{name}.imm"
        if not module_path.exists():
            raise ImmRuntimeError(f"module {name} not found at {module_path}")
        module_path = module_path.resolve()
        cache_key = ("check" if self.check_only else "run", module_path)
        if cache_key in self.module_cache:
            return self.module_cache[cache_key]
        if module_path in self.module_stack:
            cycle = " -> ".join(path.stem for path in [*self.module_stack, module_path])
            raise ImmRuntimeError(f"cyclic module import: {cycle}")
        source = module_path.read_text(encoding="utf-8")
        runtime = Runtime(
            module_path,
            output=self.output,
            input_func=self.input_func,
            check_only=self.check_only,
            module_cache=self.module_cache,
            module_stack=[*self.module_stack, module_path],
            trace_enabled=self.trace_enabled,
            trace_output=self.trace_output,
        )
        program = parse(tokenize(source))
        if self.check_only:
            runtime.check(program)
        else:
            runtime.run(program, run_main=False)
        hidden = set(core_names()) | {"math", "path", "chaser", "store", "web", "tick"}
        exports = {key: cell.value for key, cell in runtime.env.values.items() if key not in hidden}
        namespace = Namespace(name, exports)
        self.module_cache[cache_key] = namespace
        return namespace

    def _web_namespace(self):
        return Namespace(
            "web",
            {
                "grab": BuiltinFunction("web.grab", web_grab),
                "fetch": BuiltinFunction("web.fetch", web_fetch),
            },
        )

    def _tick_namespace(self):
        return Namespace(
            "tick",
            {
                "now": BuiltinFunction("tick.now", lambda: int(time.time() * 1000)),
            },
        )

    def _math_namespace(self):
        return Namespace(
            "math",
            {
                "abs": BuiltinFunction("math.abs", abs),
                "min": BuiltinFunction("math.min", min),
                "max": BuiltinFunction("math.max", max),
                "sqrt": BuiltinFunction("math.sqrt", py_math.sqrt),
                "floor": BuiltinFunction("math.floor", py_math.floor),
                "ceil": BuiltinFunction("math.ceil", py_math.ceil),
                "random": BuiltinFunction("math.random", random.random),
            },
        )

    def _path_namespace(self):
        return Namespace(
            "path",
            {
                "bfs": BuiltinFunction("path.bfs", path_bfs, needs_runtime=True),
                "astar": BuiltinFunction("path.astar", path_astar, needs_runtime=True),
            },
        )

    def _chaser_namespace(self):
        return Namespace(
            "chaser",
            {
                "direction": BuiltinFunction("chaser.direction", chaser_direction),
                "step": BuiltinFunction("chaser.step", chaser_step),
                "parse_field": BuiltinFunction("chaser.parse_field", chaser_parse_field),
                "safe_moves": BuiltinFunction("chaser.safe_moves", chaser_safe_moves),
                "random_move": BuiltinFunction("chaser.random_move", chaser_random_move),
            },
        )

    def _store_namespace(self):
        return Namespace(
            "store",
            {
                "open": BuiltinFunction("store.open", store_open, needs_runtime=True),
                "save": BuiltinFunction("store.save", store_save, needs_runtime=True),
                "load": BuiltinFunction("store.load", store_load, needs_runtime=True),
                "all": BuiltinFunction("store.all", store_all, needs_runtime=True),
                "find": BuiltinFunction("store.find", store_find, needs_runtime=True),
                "get": BuiltinFunction("store.get", store_get, needs_runtime=True),
                "delete": BuiltinFunction("store.delete", store_delete),
                "count": BuiltinFunction("store.count", store_count),
                "clear": BuiltinFunction("store.clear", store_clear),
            },
        )


@dataclass(frozen=True)
class StaticType:
    name: str
    args: tuple = ()

    def text(self):
        if not self.args:
            return self.name
        return f"{self.name}<" + ", ".join(arg.text() for arg in self.args) + ">"


@dataclass(frozen=True)
class StaticFunction:
    params: tuple
    return_type: StaticType
    name: str = "<function>"
    special: str | None = None


@dataclass(frozen=True)
class StaticLambda:
    params: tuple
    body: object
    is_block: bool
    env: object


@dataclass
class StaticBinding:
    type_info: object
    const: bool = False


class StaticEnv:
    def __init__(self, parent=None):
        self.parent = parent
        self.values = {}

    def define(self, name, type_info, const=False):
        self.values[name] = StaticBinding(type_info, const)

    def get(self, name):
        env = self._find_env(name)
        if env is None:
            raise ImmRuntimeError(f"{name} is not defined")
        return env.values[name]

    def assign(self, name, type_info, checker):
        env = self._find_env(name)
        if env is None:
            raise ImmRuntimeError(f"{name} is not defined")
        binding = env.values[name]
        if binding.const:
            raise ImmRuntimeError(f"{name} is a stash constant")
        checker.require_assignable(type_info, binding.type_info, name)

    def _find_env(self, name):
        if name in self.values:
            return self
        if self.parent is not None:
            return self.parent._find_env(name)
        return None


class StaticChecker:
    ANY = StaticType("Any")
    VOID = StaticType("Void")
    NULL = StaticType("Null")
    INT = StaticType("Int")
    FLOAT = StaticType("Float")
    BOOL = StaticType("Bool")
    STRING = StaticType("String")
    POINT = StaticType("Point")
    RESPONSE = StaticType("Response")

    def __init__(self, runtime):
        self.runtime = runtime
        self.global_env = StaticEnv()
        self.current_return = self.VOID
        self.current_den = None
        self.loop_depth = 0
        self.howl_depth = 0
        self._install_globals()

    def check(self, items):
        for item in items:
            if isinstance(item, n.FunctionDef):
                self._check_function(item)
            elif isinstance(item, n.HowlFunctionDef):
                self._check_howl_function(item)
            elif isinstance(item, n.MainDef):
                self._check_block(item.body, StaticEnv(self.global_env), self.VOID)
            elif isinstance(item, n.HowlMainDef):
                self._check_howl_block(item.body, StaticEnv(self.global_env), self.VOID)
            elif isinstance(item, n.DenDef):
                self._check_den(item)
            elif isinstance(item, n.ProbeDef):
                self._check_block(item.body, StaticEnv(self.global_env), self.VOID)
            elif not isinstance(item, (n.UseStmt, n.ModuleDef, n.MaskDef, n.PackDef)):
                self._check_stmt(item, self.global_env)

    def _install_globals(self):
        self.global_env.define("len", StaticFunction((self.ANY,), self.INT, "len"), const=True)
        self.global_env.define("type", StaticFunction((self.ANY,), self.STRING, "type"), const=True)
        self.global_env.define("str", StaticFunction((self.ANY,), self.STRING, "str"), const=True)
        self.global_env.define("int", StaticFunction((self.ANY,), self.INT, "int"), const=True)
        self.global_env.define("float", StaticFunction((self.ANY,), self.FLOAT, "float"), const=True)
        self.global_env.define("bool", StaticFunction((self.ANY,), self.BOOL, "bool"), const=True)
        self.global_env.define("map", StaticFunction((self.ANY, self.ANY), self.ANY, "map", "map"), const=True)
        self.global_env.define("filter", StaticFunction((self.ANY, self.ANY), self.ANY, "filter", "filter"), const=True)
        self.global_env.define("reduce", StaticFunction((self.ANY, self.ANY, self.ANY), self.ANY, "reduce", "reduce"), const=True)
        self.global_env.define("nap", StaticFunction((self.INT,), StaticType("Task", (self.VOID,)), "nap"), const=True)
        self.global_env.define("math", StaticType("Module", (StaticType("math"),)), const=True)
        self.global_env.define("path", StaticType("Module", (StaticType("path"),)), const=True)
        self.global_env.define("chaser", StaticType("Module", (StaticType("chaser"),)), const=True)
        self.global_env.define("store", StaticType("Module", (StaticType("store"),)), const=True)
        self.global_env.define("web", StaticType("Module", (StaticType("web"),)), const=True)
        self.global_env.define("tick", StaticType("Module", (StaticType("tick"),)), const=True)
        for name, cell in self.runtime.env.values.items():
            value = cell.value
            if isinstance(value, HowlFunction):
                sig = self._function_sig(value.params, value.return_type, name)
                self.global_env.define(name, StaticFunction(sig.params, StaticType("Task", (sig.return_type,)), name), const=True)
            elif isinstance(value, UserFunction):
                self.global_env.define(name, self._function_sig(value.params, value.return_type, name), const=True)
            elif isinstance(value, Namespace):
                self.global_env.define(name, StaticType("Module", (StaticType(value.name),)), const=True)
            elif isinstance(value, DenType):
                self.global_env.define(name, StaticType("Type", (StaticType(value.name),)), const=True)
            elif isinstance(value, MaskType):
                self.global_env.define(name, StaticType("MaskType", (StaticType(value.name),)), const=True)

    def _check_function(self, item):
        env = StaticEnv(self.global_env)
        for param in item.params:
            env.define(param.name, self._ann(param.type_name))
        self._check_block(item.body, env, self._ann(item.return_type))

    def _check_howl_function(self, item):
        env = StaticEnv(self.global_env)
        for param in item.params:
            env.define(param.name, self._ann(param.type_name))
        self._check_howl_block(item.body, env, self._ann(item.return_type))

    def _check_howl_block(self, body, env, return_type):
        self.howl_depth += 1
        try:
            self._check_block(body, env, return_type)
        finally:
            self.howl_depth -= 1

    def _check_den(self, item):
        den_type = self.runtime.dens[item.name]
        previous_den = self.current_den
        self.current_den = den_type
        try:
            for member in item.members:
                if isinstance(member, n.FieldDef) and member.expr is not None:
                    field_env = self._self_env(den_type)
                    expr_type = self._expr_type(member.expr, field_env)
                    target_type = self._ann(member.type_name) if member.type_name else expr_type
                    self._check_expr_against(member.expr, expr_type, target_type, field_env, f"{item.name}.{member.name}")
                elif isinstance(member, n.MethodDef):
                    self._check_method(den_type, member)
        finally:
            self.current_den = previous_den

    def _check_method(self, den_type, method):
        env = self._self_env(den_type)
        for param in method.params:
            env.define(param.name, self._ann(param.type_name))
        self._check_block(method.body, env, self.VOID if method.name == "init" else self._ann(method.return_type))

    def _self_env(self, den_type):
        env = StaticEnv(self.global_env)
        env.define("self", StaticType(den_type.name), const=True)
        if den_type.parent is not None:
            env.define("under", StaticType("Under", (StaticType(den_type.parent.name),)), const=True)
        return env

    def _check_block(self, body, env, return_type):
        previous_return = self.current_return
        self.current_return = return_type
        try:
            for stmt in body:
                self._check_stmt(stmt, env)
        finally:
            self.current_return = previous_return

    def _check_stmt(self, stmt, env):
        if isinstance(stmt, n.LetStmt):
            expr_type = self._expr_type(stmt.expr, env)
            target_type = self._ann(stmt.type_name) if stmt.type_name else expr_type
            self._check_expr_against(stmt.expr, expr_type, target_type, env, stmt.name)
            env.define(stmt.name, target_type, const=stmt.const)
            return
        if isinstance(stmt, n.ExprStmt):
            self._expr_type(stmt.expr, env)
            return
        if isinstance(stmt, n.SqueakStmt):
            for expr in stmt.exprs:
                self._expr_type(expr, env)
            return
        if isinstance(stmt, n.PanicStmt):
            self._expr_type(stmt.expr, env)
            return
        if isinstance(stmt, n.ExpectStmt):
            expr_type = self._expr_type(stmt.expr, env)
            self.require_assignable(expr_type, self.BOOL, "expect expression")
            return
        if isinstance(stmt, n.TraceStmt):
            for expr in stmt.exprs:
                self._expr_type(expr, env)
            return
        if isinstance(stmt, n.IfStmt):
            cond = self._expr_type(stmt.condition, env)
            self.require_assignable(cond, self.BOOL, "if condition")
            self._check_block(stmt.then_body, StaticEnv(env), self.current_return)
            if stmt.else_body is not None:
                if isinstance(stmt.else_body, n.IfStmt):
                    self._check_stmt(stmt.else_body, StaticEnv(env))
                else:
                    self._check_block(stmt.else_body, StaticEnv(env), self.current_return)
            return
        if isinstance(stmt, n.WhileStmt):
            cond = self._expr_type(stmt.condition, env)
            self.require_assignable(cond, self.BOOL, "while condition")
            self.loop_depth += 1
            try:
                self._check_block(stmt.body, StaticEnv(env), self.current_return)
            finally:
                self.loop_depth -= 1
            return
        if isinstance(stmt, n.ForStmt):
            iterable_type = self._expr_type(stmt.iterable, env)
            item_type = self._iter_item_type(iterable_type)
            loop_env = StaticEnv(env)
            loop_env.define(stmt.name, item_type)
            self.loop_depth += 1
            try:
                self._check_block(stmt.body, loop_env, self.current_return)
            finally:
                self.loop_depth -= 1
            return
        if isinstance(stmt, n.ReturnStmt):
            value_type = self.VOID if stmt.expr is None else self._expr_type(stmt.expr, env)
            self.require_assignable(value_type, self.current_return, "return value")
            return
        if isinstance(stmt, n.BreakStmt):
            if self.loop_depth == 0:
                raise ImmRuntimeError("break outside loop")
            return
        if isinstance(stmt, n.ContinueStmt):
            if self.loop_depth == 0:
                raise ImmRuntimeError("continue outside loop")
            return
        if isinstance(stmt, n.TryStmt):
            self._check_block(stmt.body, StaticEnv(env), self.current_return)
            if stmt.catch_body is not None:
                catch_env = StaticEnv(env)
                catch_env.define(stmt.catch_name, self.STRING)
                self._check_block(stmt.catch_body, catch_env, self.current_return)
            return
        if isinstance(stmt, n.InsaneBlock):
            self._check_block(stmt.body, StaticEnv(env), self.current_return)
            return
        if isinstance(
            stmt,
            (
                n.UseStmt,
                n.ModuleDef,
                n.FunctionDef,
                n.HowlFunctionDef,
                n.MainDef,
                n.HowlMainDef,
                n.DenDef,
                n.MaskDef,
                n.ProbeDef,
                n.PackDef,
            ),
        ):
            return
        raise ImmRuntimeError(f"unknown statement {type(stmt).__name__}")

    def _expr_type(self, expr, env):
        if isinstance(expr, n.Literal):
            return self._literal_type(expr.value)
        if isinstance(expr, n.Var):
            return env.get(expr.name).type_info
        if isinstance(expr, n.ArrayLiteral):
            item_types = [self._expr_type(item, env) for item in expr.items]
            return StaticType("Array", (self._unify(item_types),))
        if isinstance(expr, n.MapLiteral):
            for key, value in expr.pairs:
                self.require_assignable(self._expr_type(key, env), self.STRING, "map key")
                self._expr_type(value, env)
            return StaticType("Map", (self.ANY,))
        if isinstance(expr, n.MatrixLiteral):
            row_types = []
            for row in expr.rows:
                row_type = self._expr_type(row, env)
                if row_type.name != "Array":
                    raise ImmRuntimeError("matrix row must be Array")
                row_types.append(row_type.args[0] if row_type.args else self.ANY)
            return StaticType("Matrix", (self._unify(row_types),))
        if isinstance(expr, n.PointLiteral):
            self.require_assignable(self._expr_type(expr.x, env), self.INT, "@point x")
            self.require_assignable(self._expr_type(expr.y, env), self.INT, "@point y")
            return self.POINT
        if isinstance(expr, n.HatchExpr):
            return self._hatch_type(expr, env)
        if isinstance(expr, n.SniffExpr):
            return self.STRING
        if isinstance(expr, n.Unary):
            value_type = self._expr_type(expr.expr, env)
            if expr.op == "-":
                if value_type.name not in ("Int", "Float", "Any"):
                    raise ImmRuntimeError("- operand must be numeric")
                return value_type
            if expr.op == "!":
                self.require_assignable(value_type, self.BOOL, "! operand")
                return self.BOOL
        if isinstance(expr, n.Binary):
            return self._binary_type(expr, env)
        if isinstance(expr, n.RangeExpr):
            self.require_assignable(self._expr_type(expr.start, env), self.INT, "range start")
            self.require_assignable(self._expr_type(expr.end, env), self.INT, "range end")
            return StaticType("Range", (self.INT,))
        if isinstance(expr, n.Call):
            callee_type = self._expr_type(expr.callee, env)
            arg_types = [self._expr_type(arg, env) for arg in expr.args]
            return self._call_type(callee_type, arg_types, expr.args)
        if isinstance(expr, n.Index):
            target_type = self._expr_type(expr.target, env)
            arg_types = [self._expr_type(arg, env) for arg in expr.args]
            return self._index_type(target_type, arg_types)
        if isinstance(expr, n.Member):
            return self._member_type(self._expr_type(expr.target, env), expr.name)
        if isinstance(expr, n.Assign):
            value_type = self._expr_type(expr.value, env)
            self._check_assignment(expr.target, value_type, env)
            return value_type
        if isinstance(expr, n.LambdaExpr):
            return StaticLambda(tuple(expr.params), expr.body, expr.is_block, env)
        if isinstance(expr, n.TunnelExpr):
            left_type = self._expr_type(expr.left, env)
            return self._tunnel_type(left_type, expr.right, env)
        if isinstance(expr, n.InsaneChoose):
            values_type = self._expr_type(expr.expr, env)
            if values_type.name == "Array" and values_type.args:
                return values_type.args[0]
            return self.ANY
        if isinstance(expr, n.WaitExpr):
            if self.howl_depth == 0:
                raise ImmRuntimeError("wait can only be used inside howl context")
            waited = self._expr_type(expr.expr, env)
            if isinstance(waited, StaticType) and waited.name == "Task":
                return waited.args[0] if waited.args else self.ANY
            if isinstance(waited, StaticType) and waited.name == "TaskGroup":
                item = waited.args[0] if waited.args else self.ANY
                return StaticType("Array", (item,))
            if isinstance(waited, StaticType) and waited.name == "Any":
                return self.ANY
            raise ImmRuntimeError(f"wait expects Task, got {self._type_text(waited)}")
        if isinstance(expr, n.ScatterExpr):
            if self.howl_depth == 0:
                raise ImmRuntimeError("scatter can only be used inside howl context")
            result = self._expr_type(expr.expr, env)
            if isinstance(result, StaticType) and result.name == "Task":
                return result
            return StaticType("Task", (result,))
        if isinstance(expr, n.NestExpr):
            if self.howl_depth == 0:
                raise ImmRuntimeError("nest can only be used inside howl context")
            item_types = []
            for item in expr.items:
                result = self._expr_type(item.expr, env)
                if isinstance(result, StaticType) and result.name == "Task":
                    item_types.append(result.args[0] if result.args else self.ANY)
                else:
                    item_types.append(result)
            return StaticType("TaskGroup", (self._unify(item_types),))
        return self.ANY

    def _literal_type(self, value):
        if value is None:
            return self.NULL
        if type(value) is bool:
            return self.BOOL
        if type(value) is int:
            return self.INT
        if type(value) is float:
            return self.FLOAT
        if isinstance(value, str):
            return self.STRING
        return self.ANY

    def _binary_type(self, expr, env):
        if expr.op in ("&&", "||"):
            self.require_assignable(self._expr_type(expr.left, env), self.BOOL, f"left side of {expr.op}")
            self.require_assignable(self._expr_type(expr.right, env), self.BOOL, f"right side of {expr.op}")
            return self.BOOL
        left = self._expr_type(expr.left, env)
        right = self._expr_type(expr.right, env)
        if expr.op in ("==", "!="):
            return self.BOOL
        if expr.op in ("<", "<=", ">", ">="):
            if not (self._is_numeric(left) and self._is_numeric(right)):
                raise ImmRuntimeError(f"{expr.op} operands must be numeric")
            return self.BOOL
        if expr.op == "+":
            if left.name == "String" or right.name == "String":
                return self.STRING
            if left.name == "Point" and right.name == "Point":
                return self.POINT
            return self._numeric_result(left, right, "+")
        if expr.op in ("-", "*", "/", "%"):
            return self._numeric_result(left, right, expr.op)
        return self.ANY

    def _numeric_result(self, left, right, op):
        if not (self._is_numeric(left) and self._is_numeric(right)):
            raise ImmRuntimeError(f"{op} operands must be numeric")
        if op == "/" or left.name == "Float" or right.name == "Float":
            return self.FLOAT
        return self.INT

    def _hatch_type(self, expr, env):
        if expr.name not in self.runtime.dens:
            raise ImmRuntimeError(f"den {expr.name} is not defined")
        den_type = self.runtime.dens[expr.name]
        init = den_type.local_methods.get("init")
        arg_types = [self._expr_type(arg, env) for arg in expr.args]
        if init is None:
            if arg_types:
                raise ImmRuntimeError(f"{expr.name} has no init and expects 0 arguments")
        else:
            self._check_call_args(self._method_sig(init), arg_types)
        return StaticType(expr.name)

    def _call_type(self, callee_type, arg_types, arg_exprs):
        if isinstance(callee_type, StaticFunction):
            if callee_type.special in ("map", "filter", "reduce"):
                return self._special_call_type(callee_type.special, arg_types, arg_exprs)
            self._check_call_args(callee_type, arg_types)
            return callee_type.return_type
        if isinstance(callee_type, StaticLambda):
            return self._lambda_call_type(callee_type, arg_types)
        if isinstance(callee_type, StaticType) and callee_type.name == "Any":
            return self.ANY
        raise ImmRuntimeError(f"{self._type_text(callee_type)} is not callable")

    def _special_call_type(self, special, arg_types, arg_exprs):
        if special == "map":
            if len(arg_types) != 2:
                raise ImmRuntimeError("map expects 2 arguments")
            item_type = self._iter_item_type(arg_types[0])
            result = self._call_type(arg_types[1], [item_type], arg_exprs[1:])
            return StaticType("Array", (result,))
        if special == "filter":
            if len(arg_types) != 2:
                raise ImmRuntimeError("filter expects 2 arguments")
            item_type = self._iter_item_type(arg_types[0])
            result = self._call_type(arg_types[1], [item_type], arg_exprs[1:])
            self.require_assignable(result, self.BOOL, "filter lambda")
            return arg_types[0]
        if special == "reduce":
            if len(arg_types) != 3:
                raise ImmRuntimeError("reduce expects 3 arguments")
            item_type = self._iter_item_type(arg_types[0])
            result = self._call_type(arg_types[2], [arg_types[1], item_type], arg_exprs[2:])
            self.require_assignable(result, arg_types[1], "reduce lambda")
            return arg_types[1]
        return self.ANY

    def _lambda_call_type(self, lambda_type, arg_types):
        if len(arg_types) != len(lambda_type.params):
            raise ImmRuntimeError(f"lambda expects {len(lambda_type.params)} arguments, got {len(arg_types)}")
        env = StaticEnv(lambda_type.env)
        for name, type_info in zip(lambda_type.params, arg_types):
            env.define(name, type_info)
        if lambda_type.is_block:
            self._check_block(lambda_type.body, env, self.ANY)
            return self.ANY
        return self._expr_type(lambda_type.body, env)

    def _index_type(self, target_type, arg_types):
        if target_type.name == "Array":
            if len(arg_types) != 1:
                raise ImmRuntimeError("array index expects one argument")
            self.require_assignable(arg_types[0], self.INT, "array index")
            return target_type.args[0] if target_type.args else self.ANY
        if target_type.name == "Matrix":
            if len(arg_types) == 1:
                self.require_assignable(arg_types[0], self.POINT, "matrix point index")
            elif len(arg_types) == 2:
                self.require_assignable(arg_types[0], self.INT, "matrix y index")
                self.require_assignable(arg_types[1], self.INT, "matrix x index")
            else:
                raise ImmRuntimeError("matrix index must be [y, x] or [point]")
            return target_type.args[0] if target_type.args else self.ANY
        if target_type.name == "String":
            if len(arg_types) != 1:
                raise ImmRuntimeError("string index expects one argument")
            self.require_assignable(arg_types[0], self.INT, "string index")
            return self.STRING
        if target_type.name == "Map":
            if len(arg_types) != 1:
                raise ImmRuntimeError("map index expects one argument")
            self.require_assignable(arg_types[0], self.STRING, "map index")
            return target_type.args[0] if target_type.args else self.ANY
        if target_type.name == "Any":
            return self.ANY
        raise ImmRuntimeError(f"{target_type.text()} is not indexable")

    def _member_type(self, target_type, name):
        if target_type.name == "Point":
            if name in ("x", "y"):
                return self.INT
        if target_type.name == "Matrix":
            item = target_type.args[0] if target_type.args else self.ANY
            methods = {
                "width": StaticFunction((), self.INT, "Matrix.width"),
                "height": StaticFunction((), self.INT, "Matrix.height"),
                "in_bounds": StaticFunction((self.POINT,), self.BOOL, "Matrix.in_bounds"),
                "points": StaticFunction((), StaticType("Array", (self.POINT,)), "Matrix.points"),
                "neighbors4": StaticFunction((self.POINT,), StaticType("Array", (self.POINT,)), "Matrix.neighbors4"),
                "neighbors8": StaticFunction((self.POINT,), StaticType("Array", (self.POINT,)), "Matrix.neighbors8"),
                "find": StaticFunction((item,), self.POINT, "Matrix.find"),
                "find_all": StaticFunction((item,), StaticType("Array", (self.POINT,)), "Matrix.find_all"),
            }
            if name in methods:
                return methods[name]
        if target_type.name == "Array":
            if name == "len":
                return StaticFunction((), self.INT, "Array.len")
        if target_type.name == "String":
            methods = {
                "len": StaticFunction((), self.INT, "String.len"),
                "to_int": StaticFunction((), self.INT, "String.to_int"),
                "to_float": StaticFunction((), self.FLOAT, "String.to_float"),
                "to_bool": StaticFunction((), self.BOOL, "String.to_bool"),
            }
            if name in methods:
                return methods[name]
        if target_type.name == "Map":
            if name == "len":
                return StaticFunction((), self.INT, "Map.len")
        if target_type.name == "Response":
            fields = {
                "status": self.INT,
                "headers": StaticType("Map", (self.STRING,)),
                "body": self.STRING,
                "url": self.STRING,
                "ok": self.BOOL,
                "json": StaticFunction((), self.ANY, "Response.json"),
                "text": StaticFunction((), self.STRING, "Response.text"),
            }
            if name in fields:
                return fields[name]
        if target_type.name == "Task":
            methods = {
                "done": StaticFunction((), self.BOOL, "Task.done"),
                "cancel": StaticFunction((), self.BOOL, "Task.cancel"),
            }
            if name in methods:
                return methods[name]
        if target_type.name == "Module":
            return self._module_member_type(target_type.args[0].name if target_type.args else "", name)
        if target_type.name == "Under":
            return self._under_member_type(target_type.args[0].name, name)
        if target_type.name in self.runtime.masks:
            return self._mask_member_type(target_type.name, name)
        if target_type.name in self.runtime.dens:
            return self._den_member_type(target_type.name, name)
        if target_type.name == "Any":
            return self.ANY
        raise ImmRuntimeError(f"{target_type.text()} has no member {name}")

    def _module_member_type(self, module, name):
        if module == "math":
            returns = {
                "abs": self.FLOAT,
                "min": self.FLOAT,
                "max": self.FLOAT,
                "sqrt": self.FLOAT,
                "floor": self.INT,
                "ceil": self.INT,
                "random": self.FLOAT,
            }
            if name in returns:
                arity = 0 if name == "random" else (2 if name in ("min", "max") else 1)
                return StaticFunction(tuple(self.ANY for _ in range(arity)), returns[name], f"math.{name}")
        if module == "path":
            if name in ("bfs", "astar"):
                return StaticFunction((StaticType("Matrix", (self.ANY,)), self.POINT, self.POINT, self.ANY), StaticType("Array", (self.POINT,)), f"path.{name}")
        if module == "chaser":
            methods = {
                "direction": StaticFunction((self.POINT, self.POINT), self.STRING, "chaser.direction"),
                "step": StaticFunction((self.POINT, self.STRING), self.POINT, "chaser.step"),
                "parse_field": StaticFunction((StaticType("Array", (self.STRING,)),), StaticType("Matrix", (self.STRING,)), "chaser.parse_field"),
                "safe_moves": StaticFunction((StaticType("Matrix", (self.ANY,)), self.POINT, self.ANY), StaticType("Array", (self.POINT,)), "chaser.safe_moves"),
                "random_move": StaticFunction((StaticType("Matrix", (self.ANY,)), self.POINT, self.ANY), self.STRING, "chaser.random_move"),
            }
            if name in methods:
                return methods[name]
        if module == "store":
            store_type = StaticType("Store")
            type_value = self.ANY
            methods = {
                "open": StaticFunction((self.STRING,), store_type, "store.open"),
                "save": StaticFunction((store_type, self.ANY), self.INT, "store.save"),
                "load": StaticFunction((store_type, type_value, self.INT), self.ANY, "store.load"),
                "all": StaticFunction((store_type, type_value), StaticType("Array", (self.ANY,)), "store.all"),
                "find": StaticFunction((store_type, type_value, self.STRING, self.ANY), StaticType("Array", (self.ANY,)), "store.find"),
                "get": StaticFunction((store_type, type_value, self.STRING, self.ANY), self.ANY, "store.get"),
                "delete": StaticFunction((store_type, type_value, self.INT), self.BOOL, "store.delete"),
                "count": StaticFunction((store_type, type_value), self.INT, "store.count"),
                "clear": StaticFunction((store_type, type_value), self.INT, "store.clear"),
            }
            if name in methods:
                return methods[name]
        if module == "web":
            methods = {
                "grab": StaticFunction((self.ANY,), self.RESPONSE, "web.grab"),
                "fetch": StaticFunction((self.ANY,), StaticType("Task", (self.RESPONSE,)), "web.fetch"),
            }
            if name in methods:
                return methods[name]
        if module == "tick":
            if name == "now":
                return StaticFunction((), self.INT, "tick.now")
        return self.ANY

    def _under_member_type(self, parent_name, name):
        den_type = self.runtime.dens[parent_name]
        method = den_type.local_methods.get(name) if name == "init" else den_type.find_method(name)
        if method is None:
            raise ImmRuntimeError(f"parent den has no method {name}")
        return self._method_sig(method)

    def _mask_member_type(self, mask_name, name):
        mask = self.runtime.masks[mask_name]
        if name not in mask.methods:
            raise ImmRuntimeError(f"mask {mask_name} has no member {name}")
        return self._mask_method_sig(mask.methods[name], f"{mask_name}.{name}")

    def _den_member_type(self, den_name, name):
        den_type = self.runtime.dens[den_name]
        field = den_type.find_field(name)
        if field is not None:
            if not self._can_access_static(field):
                raise ImmRuntimeError(f"{den_name}.{name} is private")
            return self._ann(field.type_name) if field.type_name else self.ANY
        method = den_type.find_method(name)
        if method is not None:
            if not self._can_access_static(method):
                raise ImmRuntimeError(f"{den_name}.{name} is private")
            return self._method_sig(method)
        raise ImmRuntimeError(f"{den_name} has no member {name}")

    def _check_assignment(self, target, value_type, env):
        if isinstance(target, n.Var):
            env.assign(target.name, value_type, self)
            return
        if isinstance(target, n.Member):
            target_type = self._expr_type(target.target, env)
            if target_type.name in self.runtime.masks:
                raise ImmRuntimeError(f"mask {target_type.name} has no assignable field {target.name}")
            if target_type.name not in self.runtime.dens:
                raise ImmRuntimeError(f"{target_type.text()} has no assignable member {target.name}")
            den_type = self.runtime.dens[target_type.name]
            field = den_type.find_field(target.name)
            if field is None:
                raise ImmRuntimeError(f"{target_type.name} has no field {target.name}")
            if not self._can_access_static(field):
                raise ImmRuntimeError(f"{target_type.name}.{target.name} is private")
            self.require_assignable(value_type, self._ann(field.type_name), f"{field.owner.name}.{field.name}")
            return
        if isinstance(target, n.Index):
            target_type = self._expr_type(target.target, env)
            item_type = self._index_type(target_type, [self._expr_type(arg, env) for arg in target.args])
            self.require_assignable(value_type, item_type, "indexed assignment")
            return
        raise ImmRuntimeError("invalid assignment target")

    def _tunnel_type(self, left_type, right, env):
        if isinstance(right, n.Call):
            callee_type = self._expr_type(right.callee, env)
            arg_types = [left_type] + [self._expr_type(arg, env) for arg in right.args]
            return self._call_type(callee_type, arg_types, [None] + right.args)
        return self._call_type(self._expr_type(right, env), [left_type], [None])

    def _iter_item_type(self, type_info):
        if isinstance(type_info, StaticType):
            if type_info.name in ("Array", "Range"):
                return type_info.args[0] if type_info.args else self.ANY
            if type_info.name == "String":
                return self.STRING
            if type_info.name == "Any":
                return self.ANY
        raise ImmRuntimeError(f"{self._type_text(type_info)} is not iterable")

    def _check_call_args(self, fn, arg_types):
        if len(arg_types) != len(fn.params):
            raise ImmRuntimeError(f"{fn.name} expects {len(fn.params)} arguments, got {len(arg_types)}")
        for expected, actual in zip(fn.params, arg_types):
            self.require_assignable(actual, expected, f"argument to {fn.name}")

    def require_assignable(self, actual, expected, label):
        if not isinstance(actual, StaticType) or not isinstance(expected, StaticType):
            return
        if expected.name == "Any" or actual.name == "Any":
            return
        if actual.name == "Null" and (expected.name in self.runtime.dens or expected.name in self.runtime.masks or expected.name == "Null"):
            return
        if expected.name == "Float" and actual.name == "Int":
            return
        if expected.name == "Array" and actual.name == "Array":
            self.require_assignable(actual.args[0] if actual.args else self.ANY, expected.args[0] if expected.args else self.ANY, label)
            return
        if expected.name == "Matrix" and actual.name == "Matrix":
            self.require_assignable(actual.args[0] if actual.args else self.ANY, expected.args[0] if expected.args else self.ANY, label)
            return
        if expected.name == "Map" and actual.name == "Map":
            self.require_assignable(actual.args[0] if actual.args else self.ANY, expected.args[0] if expected.args else self.ANY, label)
            return
        if expected.name == "Task" and actual.name == "Task":
            self.require_assignable(actual.args[0] if actual.args else self.ANY, expected.args[0] if expected.args else self.ANY, label)
            return
        if expected.name == "TaskGroup" and actual.name == "TaskGroup":
            self.require_assignable(actual.args[0] if actual.args else self.ANY, expected.args[0] if expected.args else self.ANY, label)
            return
        if expected.name in self.runtime.dens and actual.name in self.runtime.dens and self.runtime.dens[actual.name].is_a(expected.name):
            return
        if expected.name in self.runtime.masks and actual.name in self.runtime.dens and self.runtime.dens[actual.name].wears(expected.name):
            return
        if actual.name == expected.name:
            return
        raise ImmRuntimeError(f"{label} must be {expected.text()}, got {actual.text()}")

    def _check_expr_against(self, expr, actual, expected, env, label):
        if isinstance(expected, StaticType) and expected.name == "Array" and isinstance(expr, n.ArrayLiteral):
            item_expected = expected.args[0] if expected.args else self.ANY
            for index, item in enumerate(expr.items):
                item_type = self._expr_type(item, env)
                self._check_expr_against(item, item_type, item_expected, env, f"{label}[{index}]")
            return
        if isinstance(expected, StaticType) and expected.name == "Matrix" and isinstance(expr, n.MatrixLiteral):
            item_expected = expected.args[0] if expected.args else self.ANY
            for y, row in enumerate(expr.rows):
                if not isinstance(row, n.ArrayLiteral):
                    raise ImmRuntimeError("matrix row must be Array")
                for x, item in enumerate(row.items):
                    item_type = self._expr_type(item, env)
                    self._check_expr_against(item, item_type, item_expected, env, f"{label}[{y}, {x}]")
            return
        self.require_assignable(actual, expected, label)

    def _ann(self, type_name_hint):
        if type_name_hint is None:
            return self.ANY
        type_ref = parse_type_ref(type_name_hint)
        return self._from_type_ref(type_ref)

    def _from_type_ref(self, type_ref):
        return StaticType(type_ref.name, tuple(self._from_type_ref(arg) for arg in type_ref.args))

    def _function_sig(self, params, return_type, name):
        return StaticFunction(tuple(self._ann(param.type_name) for param in params), self._ann(return_type), name)

    def _method_sig(self, method):
        return StaticFunction(tuple(self._ann(param.type_name) for param in method.params), self.VOID if method.name == "init" else self._ann(method.return_type), f"{method.owner.name}.{method.name}")

    def _mask_method_sig(self, method, name):
        return StaticFunction(tuple(self._ann(param.type_name) for param in method.params), self._ann(method.return_type), name)

    def _unify(self, types):
        if not types:
            return self.ANY
        first = types[0]
        for item in types[1:]:
            if not (isinstance(first, StaticType) and isinstance(item, StaticType) and first == item):
                if self._is_numeric(first) and self._is_numeric(item):
                    first = self.FLOAT if first.name == "Float" or item.name == "Float" else self.INT
                else:
                    return self.ANY
        return first

    def _is_numeric(self, type_info):
        return isinstance(type_info, StaticType) and type_info.name in ("Int", "Float", "Any")

    def _can_access_static(self, member):
        if member.access == "fur":
            return True
        return self.current_den is member.owner

    def _type_text(self, type_info):
        if isinstance(type_info, StaticType):
            return type_info.text()
        if isinstance(type_info, StaticFunction):
            return type_info.name
        if isinstance(type_info, StaticLambda):
            return "lambda"
        return str(type_info)


def builtin_map(runtime, values, fn):
    return [runtime.call_value(fn, [value]) for value in values]


def builtin_filter(runtime, values, fn):
    result = []
    for value in values:
        keep = runtime.call_value(fn, [value])
        if type(keep) is not bool:
            raise ImmRuntimeError("filter lambda must return Bool")
        if keep:
            result.append(value)
    return result


def builtin_reduce(runtime, values, initial, fn):
    acc = initial
    for value in values:
        acc = runtime.call_value(fn, [acc, value])
    return acc


def builtin_nap(ms):
    if type(ms) is not int:
        raise ImmRuntimeError("nap expects Int milliseconds")
    if ms < 0:
        raise ImmRuntimeError("nap milliseconds must be >= 0")

    async def runner():
        await asyncio.sleep(ms / 1000)
        return None

    return ImmTask(runner, name="nap")


def web_fetch(options):
    async def runner():
        return await asyncio.to_thread(web_grab, options)

    return ImmTask(runner, name="web.fetch")


def web_grab(options):
    request_options = normalize_web_options(options)
    data = None
    if request_options["body"] is not None:
        data = request_options["body"].encode("utf-8")
    request = urllib.request.Request(
        request_options["url"],
        data=data,
        headers=request_options["headers"],
        method=request_options["method"],
    )
    timeout = request_options["timeout_ms"] / 1000
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            body = response.read().decode(response.headers.get_content_charset() or "utf-8", errors="replace")
            return Response(response.status, dict(response.headers.items()), body, response.geturl())
    except urllib.error.HTTPError as err:
        body = err.read().decode(err.headers.get_content_charset() or "utf-8", errors="replace")
        return Response(err.code, dict(err.headers.items()), body, err.geturl())
    except urllib.error.URLError as err:
        reason = getattr(err, "reason", err)
        raise ImmRuntimeError(f"network request failed: {reason}") from err
    except TimeoutError as err:
        raise ImmRuntimeError("network request timed out") from err
    except ValueError as err:
        raise ImmRuntimeError(f"invalid URL: {err}") from err


def normalize_web_options(options):
    if isinstance(options, str):
        result = {
            "method": "GET",
            "url": options,
            "headers": {},
            "body": None,
            "timeout_ms": 10000,
        }
    elif isinstance(options, dict):
        result = {
            "method": options.get("method", "GET"),
            "url": options.get("url"),
            "headers": options.get("headers", {}),
            "body": options.get("body", None),
            "timeout_ms": options.get("timeout_ms", 10000),
        }
    else:
        raise ImmRuntimeError("web request expects String URL or Map options")
    if not isinstance(result["method"], str):
        raise ImmRuntimeError("web request method must be String")
    result["method"] = result["method"].upper()
    if not isinstance(result["url"], str) or not result["url"]:
        raise ImmRuntimeError("web request url must be String")
    if not isinstance(result["headers"], dict):
        raise ImmRuntimeError("web request headers must be Map")
    headers = {}
    for key, value in result["headers"].items():
        if not isinstance(key, str) or not isinstance(value, str):
            raise ImmRuntimeError("web request headers must be Map<String, String>")
        headers[key] = value
    result["headers"] = headers
    if result["body"] is not None and not isinstance(result["body"], str):
        raise ImmRuntimeError("web request body must be String or Null")
    if type(result["timeout_ms"]) is not int:
        raise ImmRuntimeError("web request timeout_ms must be Int")
    if result["timeout_ms"] <= 0:
        raise ImmRuntimeError("web request timeout_ms must be > 0")
    return result


def path_bfs(runtime, field, start, goal, passable):
    _require_path_args(field, start, goal)
    queue = deque([start])
    came_from = {start: None}
    while queue:
        current = queue.popleft()
        if current == goal:
            return _reconstruct_path(came_from, current)
        for nxt in field.neighbors4(current):
            if nxt in came_from:
                continue
            if not runtime.call_value(passable, [field.get([nxt])]):
                continue
            came_from[nxt] = current
            queue.append(nxt)
    return None


def path_astar(runtime, field, start, goal, passable):
    _require_path_args(field, start, goal)
    counter = 0
    frontier = [(0, counter, start)]
    came_from = {start: None}
    cost_so_far = {start: 0}
    while frontier:
        _, _, current = heapq.heappop(frontier)
        if current == goal:
            return _reconstruct_path(came_from, current)
        for nxt in field.neighbors4(current):
            if not runtime.call_value(passable, [field.get([nxt])]):
                continue
            new_cost = cost_so_far[current] + 1
            if nxt not in cost_so_far or new_cost < cost_so_far[nxt]:
                cost_so_far[nxt] = new_cost
                counter += 1
                priority = new_cost + abs(goal.x - nxt.x) + abs(goal.y - nxt.y)
                heapq.heappush(frontier, (priority, counter, nxt))
                came_from[nxt] = current
    return None


def chaser_direction(src, dst):
    if not isinstance(src, Point) or not isinstance(dst, Point):
        raise ImmRuntimeError("chaser.direction expects Point, Point")
    if dst.x > src.x:
        return "RIGHT"
    if dst.x < src.x:
        return "LEFT"
    if dst.y > src.y:
        return "DOWN"
    if dst.y < src.y:
        return "UP"
    return "STAY"


def chaser_step(pos, direction):
    if not isinstance(pos, Point):
        raise ImmRuntimeError("chaser.step expects Point, String")
    if direction == "UP":
        return Point(pos.x, pos.y - 1)
    if direction == "RIGHT":
        return Point(pos.x + 1, pos.y)
    if direction == "DOWN":
        return Point(pos.x, pos.y + 1)
    if direction == "LEFT":
        return Point(pos.x - 1, pos.y)
    if direction == "STAY":
        return pos
    raise ImmRuntimeError(f"unknown CHaser direction {direction}")


def chaser_parse_field(lines):
    if not isinstance(lines, list):
        raise ImmRuntimeError("chaser.parse_field expects Array<String>")
    rows = []
    for line in lines:
        if not isinstance(line, str):
            raise ImmRuntimeError("chaser.parse_field expects Array<String>")
        rows.append(list(line))
    return Matrix(rows)


def chaser_safe_moves(field, pos, wall):
    if not isinstance(field, Matrix) or not isinstance(pos, Point):
        raise ImmRuntimeError("chaser.safe_moves expects Matrix, Point, wall value")
    moves = []
    for point in field.neighbors4(pos):
        if field.get([point]) != wall:
            moves.append(point)
    return moves


def chaser_random_move(field, pos, wall):
    moves = chaser_safe_moves(field, pos, wall)
    if not moves:
        return "STAY"
    return chaser_direction(pos, random.choice(moves))


def store_open(runtime, path):
    if not isinstance(path, str):
        raise ImmRuntimeError("store.open expects String path")
    store_path = Path(path)
    if not store_path.is_absolute():
        base = runtime.source_path.parent if runtime.source_path is not None else Path.cwd()
        store_path = base / store_path
    return StoreDatabase(store_path)


def store_save(runtime, db, obj):
    db = require_store(db)
    obj = unwrap_object_view(obj)
    if not isinstance(obj, ObjectInstance):
        raise ImmRuntimeError("store.save expects a den object")
    db_key = str(db.path)
    den_name = obj.den_type.name
    object_id = obj.store_ids.get(db_key)
    records = db.records_for(den_name)
    if object_id is None or str(object_id) not in records:
        object_id = db.next_id(den_name)
    obj.store_ids[db_key] = object_id
    records[str(object_id)] = {
        "id": object_id,
        "den": den_name,
        "fields": serialize_object_fields(runtime, obj, set()),
    }
    db.flush()
    return object_id


def store_load(runtime, db, den_type, object_id):
    db = require_store(db)
    den_type = require_den_type(den_type)
    object_id = require_store_id(object_id)
    record = db.records_for(den_type.name).get(str(object_id))
    if record is None:
        return None
    return deserialize_object(runtime, den_type, record["fields"], db, object_id)


def store_all(runtime, db, den_type):
    db = require_store(db)
    den_type = require_den_type(den_type)
    records = db.records_for(den_type.name)
    return [
        deserialize_object(runtime, den_type, records[key]["fields"], db, int(key))
        for key in sorted(records, key=lambda value: int(value))
    ]


def store_find(runtime, db, den_type, field_name, value):
    db = require_store(db)
    den_type = require_den_type(den_type)
    if not isinstance(field_name, str):
        raise ImmRuntimeError("store.find field name must be String")
    needle = serialize_value(runtime, value, set())
    results = []
    records = db.records_for(den_type.name)
    for key in sorted(records, key=lambda value: int(value)):
        record = records[key]
        if record.get("fields", {}).get(field_name) == needle:
            results.append(deserialize_object(runtime, den_type, record["fields"], db, int(key)))
    return results


def store_get(runtime, db, den_type, field_name, value):
    matches = store_find(runtime, db, den_type, field_name, value)
    return matches[0] if matches else None


def store_delete(db, den_type, object_id):
    db = require_store(db)
    den_type = require_den_type(den_type)
    object_id = require_store_id(object_id)
    records = db.records_for(den_type.name)
    existed = str(object_id) in records
    if existed:
        del records[str(object_id)]
        db.flush()
    return existed


def store_count(db, den_type):
    db = require_store(db)
    den_type = require_den_type(den_type)
    return len(db.records_for(den_type.name))


def store_clear(db, den_type):
    db = require_store(db)
    den_type = require_den_type(den_type)
    records = db.records_for(den_type.name)
    count = len(records)
    records.clear()
    db.flush()
    return count


def require_store(value):
    if not isinstance(value, StoreDatabase):
        raise ImmRuntimeError("expected Store")
    return value


def require_den_type(value):
    if not isinstance(value, DenType):
        raise ImmRuntimeError("expected den type")
    return value


def require_store_id(value):
    if type(value) is not int:
        raise ImmRuntimeError("store id must be Int")
    return value


def unwrap_object_view(value):
    if isinstance(value, ObjectView):
        return value.instance
    return value


def serialize_object_fields(runtime, obj, seen):
    fields = {}
    for field in obj.den_type.field_order():
        value = obj.fields.get(field.name, UNINITIALIZED)
        if value is UNINITIALIZED:
            raise ImmRuntimeError(f"{obj.den_type.name}.{field.name} is not initialized")
        fields[field.name] = serialize_value(runtime, value, seen)
    return fields


def serialize_value(runtime, value, seen):
    value = unwrap_object_view(value)
    if value is None:
        return {"kind": "Null"}
    if type(value) is bool:
        return {"kind": "Bool", "value": value}
    if type(value) is int:
        return {"kind": "Int", "value": value}
    if type(value) is float:
        return {"kind": "Float", "value": value}
    if isinstance(value, str):
        return {"kind": "String", "value": value}
    if isinstance(value, list):
        return {"kind": "Array", "items": [serialize_value(runtime, item, seen) for item in value]}
    if isinstance(value, Point):
        return {"kind": "Point", "x": value.x, "y": value.y}
    if isinstance(value, Matrix):
        return {
            "kind": "Matrix",
            "rows": [[serialize_value(runtime, item, seen) for item in row] for row in value.rows],
        }
    if isinstance(value, ObjectInstance):
        identity = id(value)
        if identity in seen:
            raise ImmRuntimeError("store cannot serialize cyclic object graphs")
        seen.add(identity)
        try:
            return {
                "kind": "Object",
                "den": value.den_type.name,
                "fields": serialize_object_fields(runtime, value, seen),
            }
        finally:
            seen.remove(identity)
    raise ImmRuntimeError(f"store cannot serialize {type_name(value)}")


def deserialize_value(runtime, encoded):
    kind = encoded.get("kind")
    if kind == "Null":
        return None
    if kind in ("Bool", "Int", "Float", "String"):
        return encoded.get("value")
    if kind == "Array":
        return [deserialize_value(runtime, item) for item in encoded.get("items", [])]
    if kind == "Point":
        return Point(encoded["x"], encoded["y"])
    if kind == "Matrix":
        return Matrix([[deserialize_value(runtime, item) for item in row] for row in encoded.get("rows", [])])
    if kind == "Object":
        den_name = encoded.get("den")
        if den_name not in runtime.dens:
            raise ImmRuntimeError(f"stored den {den_name} is not defined")
        return deserialize_object(runtime, runtime.dens[den_name], encoded.get("fields", {}), None, None)
    raise ImmRuntimeError(f"unknown stored value kind {kind}")


def deserialize_object(runtime, den_type, fields, db, object_id):
    instance = ObjectInstance(den_type)
    for field in den_type.field_order():
        if field.name not in fields:
            raise ImmRuntimeError(f"stored {den_type.name}.{field.name} is missing")
        value = deserialize_value(runtime, fields[field.name])
        check_type(value, field.type_name, f"{field.owner.name}.{field.name}", runtime)
        instance.fields[field.name] = value
    if db is not None and object_id is not None:
        instance.store_ids[str(db.path)] = object_id
    runtime._check_initialized_fields(instance)
    return instance


def _require_path_args(field, start, goal):
    if not isinstance(field, Matrix) or not isinstance(start, Point) or not isinstance(goal, Point):
        raise ImmRuntimeError("path functions expect Matrix, Point, Point")


def _reconstruct_path(came_from, current):
    path = []
    while current is not None:
        path.append(current)
        current = came_from[current]
    path.reverse()
    return path


def core_names():
    return {"len", "type", "str", "int", "float", "bool", "map", "filter", "reduce", "nap"}


def same_signature(left, right):
    if len(left.params) != len(right.params):
        return False
    for left_param, right_param in zip(left.params, right.params):
        if normalize_type_name(left_param.type_name) != normalize_type_name(right_param.type_name):
            return False
    return normalize_type_name(left.return_type) == normalize_type_name(right.return_type)


def normalize_type_name(type_name_hint):
    if type_name_hint is None:
        return "Void"
    return type_name_hint


@dataclass(frozen=True)
class TypeRef:
    name: str
    args: tuple


def parse_type_ref(type_name_hint):
    text = type_name_hint.strip()
    if not text:
        raise ImmRuntimeError("empty type annotation")
    if "<" not in text:
        return TypeRef(text, ())
    if not text.endswith(">"):
        raise ImmRuntimeError(f"invalid type annotation {type_name_hint}")
    base, inner = text.split("<", 1)
    inner = inner[:-1]
    args = tuple(parse_type_ref(part) for part in split_type_args(inner))
    return TypeRef(base.strip(), args)


def split_type_args(text):
    args = []
    start = 0
    depth = 0
    for index, char in enumerate(text):
        if char == "<":
            depth += 1
        elif char == ">":
            depth -= 1
            if depth < 0:
                raise ImmRuntimeError(f"invalid type annotation {text}")
        elif char == "," and depth == 0:
            args.append(text[start:index].strip())
            start = index + 1
    if depth != 0:
        raise ImmRuntimeError(f"invalid type annotation {text}")
    args.append(text[start:].strip())
    return [arg for arg in args if arg]


def check_type(value, type_name_hint, label, runtime=None):
    if type_name_hint is None:
        return
    type_ref = parse_type_ref(type_name_hint)
    _check_type_ref(value, type_ref, label, runtime)


def _check_type_ref(value, type_ref, label, runtime=None):
    base = type_ref.name
    if isinstance(value, ObjectView):
        value = value.instance
    if base in ("Any", "T"):
        return
    if base == "Void":
        if value is not None:
            raise ImmRuntimeError(f"{label} must be Void")
        return
    if base == "Int":
        ok = type(value) is int
    elif base == "Float":
        ok = type(value) in (int, float)
    elif base == "Bool":
        ok = type(value) is bool
    elif base == "String":
        ok = isinstance(value, str)
    elif base == "Array":
        ok = isinstance(value, list)
    elif base == "Map":
        ok = isinstance(value, dict)
    elif base == "Matrix":
        ok = isinstance(value, Matrix)
    elif base == "Point":
        ok = isinstance(value, Point)
    elif base == "Response":
        ok = isinstance(value, Response)
    elif base == "Task":
        ok = isinstance(value, ImmTask)
    elif base == "TaskGroup":
        ok = isinstance(value, TaskGroup)
    elif base == "Null":
        ok = value is None
    else:
        if value is None:
            return
        if isinstance(value, ObjectInstance):
            ok = value.den_type.is_a(base) or value.den_type.wears(base)
        elif isinstance(value, DenType):
            ok = value.name == base
        elif isinstance(value, MaskType):
            ok = value.name == base
        elif runtime is not None and (base in runtime.dens or base in runtime.masks):
            ok = False
        else:
            raise ImmRuntimeError(f"unknown type annotation {format_type_ref(type_ref)}")

    if not ok:
        raise ImmRuntimeError(f"{label} must be {format_type_ref(type_ref)}, got {type_name(value)}")

    if base == "Array" and type_ref.args:
        if len(type_ref.args) != 1:
            raise ImmRuntimeError("Array expects one type argument")
        item_type = type_ref.args[0]
        for index, item in enumerate(value):
            _check_type_ref(item, item_type, f"{label}[{index}]", runtime)
    elif base == "Matrix" and type_ref.args:
        if len(type_ref.args) != 1:
            raise ImmRuntimeError("Matrix expects one type argument")
        item_type = type_ref.args[0]
        for y, row in enumerate(value.rows):
            for x, item in enumerate(row):
                _check_type_ref(item, item_type, f"{label}[{y}, {x}]", runtime)
    elif base == "Map" and type_ref.args:
        if len(type_ref.args) != 1:
            raise ImmRuntimeError("Map expects one type argument")
        item_type = type_ref.args[0]
        for key, item in value.items():
            _check_type_ref(item, item_type, f"{label}[{key}]", runtime)


def format_type_ref(type_ref):
    if not type_ref.args:
        return type_ref.name
    return f"{type_ref.name}<" + ", ".join(format_type_ref(arg) for arg in type_ref.args) + ">"


def type_name(value):
    if value is None:
        return "Null"
    if type(value) is bool:
        return "Bool"
    if type(value) is int:
        return "Int"
    if type(value) is float:
        return "Float"
    if isinstance(value, str):
        return "String"
    if isinstance(value, list):
        return "Array"
    if isinstance(value, dict):
        return "Map"
    if isinstance(value, Matrix):
        return "Matrix"
    if isinstance(value, Point):
        return "Point"
    if isinstance(value, Response):
        return "Response"
    if isinstance(value, ImmTask):
        return "Task"
    if isinstance(value, TaskGroup):
        return "TaskGroup"
    if isinstance(value, Namespace):
        return "Module"
    if isinstance(value, ObjectInstance):
        return value.den_type.name
    if isinstance(value, ObjectView):
        return value.mask.name
    if isinstance(value, DenType):
        return "Den"
    if isinstance(value, MaskType):
        return "Mask"
    if isinstance(value, StoreDatabase):
        return "Store"
    return type(value).__name__


def format_value(value):
    if value is None:
        return "null"
    if type(value) is bool:
        return "true" if value else "false"
    if isinstance(value, str):
        return value
    if isinstance(value, Point):
        return str(value)
    if isinstance(value, Matrix):
        return str(value)
    if isinstance(value, ObjectView):
        return str(value)
    if isinstance(value, StoreDatabase):
        return str(value)
    if isinstance(value, Response):
        return str(value)
    if isinstance(value, ImmTask):
        return str(value)
    if isinstance(value, TaskGroup):
        return str(value)
    if isinstance(value, list):
        return "[" + ", ".join(format_value(item) for item in value) + "]"
    if isinstance(value, dict):
        items = [f"{format_value(key)}: {format_value(item)}" for key, item in value.items()]
        return "{" + ", ".join(items) + "}"
    return str(value)

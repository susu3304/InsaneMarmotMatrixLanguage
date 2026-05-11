from dataclasses import dataclass


@dataclass
class Program:
    items: list


@dataclass
class MainDef:
    body: list
    insane: bool = False


@dataclass
class HowlMainDef:
    body: list
    insane: bool = False


@dataclass
class FunctionDef:
    name: str
    params: list
    return_type: str | None
    body: list


@dataclass
class HowlFunctionDef:
    name: str
    params: list
    return_type: str | None
    body: list


@dataclass
class FieldDef:
    name: str
    type_name: str | None
    expr: object | None
    access: str = "fang"


@dataclass
class MethodDef:
    name: str
    params: list
    return_type: str | None
    body: list
    access: str = "fang"


@dataclass
class DenDef:
    name: str
    parent: str | None
    masks: list
    members: list


@dataclass
class MaskMethod:
    name: str
    params: list
    return_type: str | None


@dataclass
class MaskDef:
    name: str
    methods: list


@dataclass
class Param:
    name: str
    type_name: str | None = None


@dataclass
class UseStmt:
    name: str


@dataclass
class ModuleDef:
    name: str


@dataclass
class ProbeDef:
    name: str
    body: list


@dataclass
class PackDef:
    entry: str | None
    crate: str | None
    pelt: str | None


@dataclass
class LetStmt:
    name: str
    expr: object
    type_name: str | None = None
    const: bool = False


@dataclass
class IfStmt:
    condition: object
    then_body: list
    else_body: object | None = None


@dataclass
class ForStmt:
    name: str
    iterable: object
    body: list
    insane: bool = False


@dataclass
class WhileStmt:
    condition: object
    body: list


@dataclass
class ReturnStmt:
    expr: object | None = None


@dataclass
class BreakStmt:
    pass


@dataclass
class ContinueStmt:
    pass


@dataclass
class ExprStmt:
    expr: object


@dataclass
class SqueakStmt:
    exprs: list


@dataclass
class PanicStmt:
    expr: object


@dataclass
class ExpectStmt:
    expr: object


@dataclass
class TraceStmt:
    exprs: list


@dataclass
class TryStmt:
    body: list
    catch_name: str | None
    catch_body: list | None
    insane: bool = False


@dataclass
class InsaneBlock:
    body: list


@dataclass
class Literal:
    value: object


@dataclass
class Var:
    name: str


@dataclass
class Assign:
    target: object
    value: object


@dataclass
class Binary:
    left: object
    op: str
    right: object


@dataclass
class Unary:
    op: str
    expr: object


@dataclass
class ArrayLiteral:
    items: list


@dataclass
class MatrixLiteral:
    rows: list


@dataclass
class MapLiteral:
    pairs: list


@dataclass
class PointLiteral:
    x: object
    y: object


@dataclass
class HatchExpr:
    name: str
    args: list


@dataclass
class Call:
    callee: object
    args: list


@dataclass
class Index:
    target: object
    args: list


@dataclass
class Member:
    target: object
    name: str


@dataclass
class RangeExpr:
    start: object
    end: object


@dataclass
class LambdaExpr:
    params: list
    body: object
    is_block: bool = False


@dataclass
class TunnelExpr:
    left: object
    right: object


@dataclass
class InsaneChoose:
    expr: object


@dataclass
class WaitExpr:
    expr: object


@dataclass
class ScatterExpr:
    expr: object
    insane: bool = False


@dataclass
class NestExpr:
    items: list


@dataclass
class SniffExpr:
    pass

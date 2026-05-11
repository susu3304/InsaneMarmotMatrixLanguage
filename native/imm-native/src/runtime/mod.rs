use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value as JsonValue};

use crate::ast::*;
use crate::diagnostics::{Category, Diagnostic};
use crate::parser::parse_source;

type EnvRef = Rc<RefCell<Environment>>;
type ObjRef = Rc<RefCell<ObjectInstance>>;
type MatrixRef = Rc<RefCell<MatrixData>>;
type ArrayRef = Rc<RefCell<Vec<Value>>>;
type MapRef = Rc<RefCell<BTreeMap<String, Value>>>;
type StoreRef = Rc<RefCell<StoreDatabase>>;
type TaskRef = Rc<RefCell<TaskData>>;
type ModuleCache = Rc<RefCell<HashMap<PathBuf, Value>>>;

#[derive(Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(ArrayRef),
    Map(MapRef),
    Matrix(MatrixRef),
    Point(Point),
    Range(i64, i64),
    Function(Rc<UserFunction>),
    Lambda(Rc<LambdaFunction>),
    Builtin(BuiltinKind),
    NativeMethod(Rc<NativeMethod>),
    Namespace(Rc<Namespace>),
    DenType(String),
    MaskType(String),
    Object(ObjRef),
    ObjectView { object: ObjRef, mask: String },
    ObjectMethod(Rc<ObjectBoundMethod>),
    UnderProxy { object: ObjRef, parent: String },
    Response(Rc<Response>),
    Task(TaskRef),
    TaskGroup(Vec<Value>),
    Store(StoreRef),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format_value(self))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

#[derive(Clone, Debug)]
pub struct MatrixData {
    rows: Vec<Vec<Value>>,
}

#[derive(Clone)]
pub struct Namespace {
    name: String,
    values: BTreeMap<String, Value>,
}

#[derive(Clone)]
pub struct UserFunction {
    name: String,
    params: Vec<Param>,
    return_type: Option<String>,
    body: Vec<Stmt>,
    closure: EnvRef,
    howl: bool,
}

#[derive(Clone)]
pub struct LambdaFunction {
    params: Vec<String>,
    body: LambdaBody,
    closure: EnvRef,
}

#[derive(Clone, Debug)]
pub struct FieldSpec {
    name: String,
    type_name: Option<String>,
    expr: Option<Expr>,
    access: Access,
    owner: String,
}

#[derive(Clone)]
pub struct MethodSpec {
    name: String,
    params: Vec<Param>,
    return_type: Option<String>,
    body: Vec<Stmt>,
    access: Access,
    owner: String,
    closure: EnvRef,
}

#[derive(Clone)]
pub struct DenType {
    name: String,
    parent: Option<String>,
    masks: Vec<String>,
    local_fields: BTreeMap<String, FieldSpec>,
    local_methods: BTreeMap<String, MethodSpec>,
}

#[derive(Clone)]
pub struct MaskType {
    methods: BTreeMap<String, MaskMethod>,
}

#[derive(Clone)]
pub struct ObjectInstance {
    den_name: String,
    fields: BTreeMap<String, Option<Value>>,
    store_ids: BTreeMap<String, i64>,
}

#[derive(Clone)]
pub struct ObjectBoundMethod {
    object: ObjRef,
    method: MethodSpec,
}

#[derive(Clone)]
pub struct Response {
    status: i64,
    headers: BTreeMap<String, Value>,
    body: String,
    url: String,
    ok: bool,
}

#[derive(Clone)]
pub struct TaskData {
    value: Value,
}

#[derive(Clone)]
pub struct StoreDatabase {
    path: PathBuf,
    data: JsonValue,
}

#[derive(Clone)]
pub struct NativeMethod {
    receiver: Value,
    kind: NativeMethodKind,
}

#[derive(Clone, Copy)]
pub enum BuiltinKind {
    Len,
    Type,
    Str,
    Int,
    Float,
    Bool,
    Map,
    Filter,
    Reduce,
    Nap,
    MathAbs,
    MathMin,
    MathMax,
    MathSqrt,
    MathFloor,
    MathCeil,
    MathRandom,
    PathBfs,
    PathAstar,
    ChaserDirection,
    ChaserStep,
    ChaserParseField,
    ChaserSafeMoves,
    ChaserRandomMove,
    StoreOpen,
    StoreSave,
    StoreLoad,
    StoreAll,
    StoreFind,
    StoreGet,
    StoreDelete,
    StoreCount,
    StoreClear,
    WebGrab,
    WebFetch,
    TickNow,
}

#[derive(Clone, Copy)]
pub enum NativeMethodKind {
    ArrayLen,
    MapLen,
    StringLen,
    StringToInt,
    StringToFloat,
    StringToBool,
    MatrixWidth,
    MatrixHeight,
    MatrixInBounds,
    MatrixPoints,
    MatrixNeighbors4,
    MatrixNeighbors8,
    MatrixFind,
    MatrixFindAll,
    ResponseText,
    ResponseJson,
    TaskDone,
    TaskCancel,
}

#[derive(Clone)]
struct Cell {
    value: Value,
    is_const: bool,
    type_name: Option<String>,
}

#[derive(Clone, Default)]
struct Environment {
    parent: Option<EnvRef>,
    values: BTreeMap<String, Cell>,
}

impl Environment {
    fn child(parent: EnvRef) -> EnvRef {
        Rc::new(RefCell::new(Self {
            parent: Some(parent),
            values: BTreeMap::new(),
        }))
    }

    fn define_unchecked(
        &mut self,
        name: impl Into<String>,
        value: Value,
        is_const: bool,
        type_name: Option<String>,
    ) {
        self.values.insert(
            name.into(),
            Cell {
                value,
                is_const,
                type_name,
            },
        );
    }
}

#[derive(Clone, Debug)]
enum Control {
    None,
    Return(Value),
    Break,
    Continue,
}

pub struct Runtime {
    source_path: Option<PathBuf>,
    env: EnvRef,
    module_cache: ModuleCache,
    module_stack: Vec<PathBuf>,
    output: Rc<RefCell<Vec<String>>>,
    trace: Rc<RefCell<Vec<String>>>,
    trace_enabled: bool,
    dens: BTreeMap<String, DenType>,
    masks: BTreeMap<String, MaskType>,
    current_den: Vec<String>,
    insane_depth: usize,
    howl_depth: usize,
    embedded_sources: Rc<BTreeMap<String, String>>,
}

impl Runtime {
    pub fn new(source_path: Option<PathBuf>) -> Self {
        let mut runtime = Self {
            source_path: source_path.map(|path| path.to_path_buf()),
            env: Rc::new(RefCell::new(Environment::default())),
            module_cache: Rc::new(RefCell::new(HashMap::new())),
            module_stack: Vec::new(),
            output: Rc::new(RefCell::new(Vec::new())),
            trace: Rc::new(RefCell::new(Vec::new())),
            trace_enabled: false,
            dens: BTreeMap::new(),
            masks: BTreeMap::new(),
            current_den: Vec::new(),
            insane_depth: 0,
            howl_depth: 0,
            embedded_sources: Rc::new(BTreeMap::new()),
        };
        runtime.install_core();
        runtime
    }

    pub fn with_embedded_sources(entry_name: &str, sources: BTreeMap<String, String>) -> Self {
        let mut runtime = Self::new(Some(PathBuf::from(entry_name)));
        runtime.embedded_sources = Rc::new(sources);
        runtime
    }

    pub fn set_trace_enabled(&mut self, enabled: bool) {
        self.trace_enabled = enabled;
    }

    pub fn output_lines(&self) -> Vec<String> {
        self.output.borrow().clone()
    }

    pub fn trace_lines(&self) -> Vec<String> {
        self.trace.borrow().clone()
    }

    pub fn load_program(&self, path: &Path) -> Result<Program, Diagnostic> {
        let source = fs::read_to_string(path).map_err(io_error)?;
        parse_source(0, &source)
    }

    pub fn check(&mut self, program: &Program) -> Result<(), Diagnostic> {
        self.prepare(program)?;
        self.static_check(program)
    }

    pub fn run(&mut self, program: &Program, run_main: bool) -> Result<(), Diagnostic> {
        let main = self.prepare(program)?;
        for item in &program.items {
            if should_skip_top_level_execute(item) {
                continue;
            }
            if let Item::Stmt(stmt) = item {
                match self.execute(stmt)? {
                    Control::None => {}
                    Control::Return(_) => return Err(runtime_error("return outside function")),
                    Control::Break | Control::Continue => {
                        return Err(runtime_error("loop control outside loop"));
                    }
                }
            }
        }
        if run_main {
            let Some(main) = main else {
                return Err(runtime_error("marmot main is not defined"));
            };
            match main {
                PreparedMain::Main { body, insane } => {
                    if insane {
                        self.insane_depth += 1;
                    }
                    let result = self.execute_block(&body, Environment::child(self.env.clone()));
                    if insane {
                        self.insane_depth -= 1;
                    }
                    match result? {
                        Control::None => {}
                        Control::Return(_) => return Err(runtime_error("return outside function")),
                        Control::Break | Control::Continue => {
                            return Err(runtime_error("loop control outside loop"));
                        }
                    }
                }
                PreparedMain::HowlMain { body, insane } => {
                    if insane {
                        self.insane_depth += 1;
                    }
                    self.howl_depth += 1;
                    let result = self.execute_block(&body, Environment::child(self.env.clone()));
                    self.howl_depth -= 1;
                    if insane {
                        self.insane_depth -= 1;
                    }
                    result?;
                }
            }
        }
        Ok(())
    }

    pub fn run_probe_blocks(
        &mut self,
        program: &Program,
    ) -> Result<Vec<(String, bool, Option<String>)>, Diagnostic> {
        self.prepare(program)?;
        for item in &program.items {
            if should_skip_top_level_execute(item) {
                continue;
            }
            if let Item::Stmt(stmt) = item {
                self.execute(stmt)?;
            }
        }

        let mut results = Vec::new();
        for item in &program.items {
            if let Item::Probe { name, body } = item {
                match self.execute_block(body, Environment::child(self.env.clone())) {
                    Ok(Control::None) => results.push((name.clone(), true, None)),
                    Ok(_) => results.push((
                        name.clone(),
                        false,
                        Some("probe exited with control flow".to_string()),
                    )),
                    Err(err) => results.push((name.clone(), false, Some(err.message))),
                }
            }
        }
        Ok(results)
    }

    fn prepare(&mut self, program: &Program) -> Result<Option<PreparedMain>, Diagnostic> {
        let mut main = None;
        let mut howl_main = None;
        for item in &program.items {
            if let Item::Use(name) = item {
                let namespace = self.load_namespace(name)?;
                self.env
                    .borrow_mut()
                    .define_unchecked(name.clone(), namespace, true, None);
            }
        }
        for item in &program.items {
            if let Item::Mask(mask) = item {
                self.register_mask(mask)?;
            }
        }
        for item in &program.items {
            if let Item::Den(den) = item {
                self.register_den(den)?;
            }
        }
        self.validate_dens()?;
        for item in &program.items {
            match item {
                Item::Function(def) => {
                    let function = UserFunction {
                        name: def.name.clone(),
                        params: def.params.clone(),
                        return_type: def.return_type.clone(),
                        body: def.body.clone(),
                        closure: self.env.clone(),
                        howl: false,
                    };
                    self.env.borrow_mut().define_unchecked(
                        def.name.clone(),
                        Value::Function(Rc::new(function)),
                        true,
                        None,
                    );
                }
                Item::HowlFunction(def) => {
                    let function = UserFunction {
                        name: def.name.clone(),
                        params: def.params.clone(),
                        return_type: def.return_type.clone(),
                        body: def.body.clone(),
                        closure: self.env.clone(),
                        howl: true,
                    };
                    self.env.borrow_mut().define_unchecked(
                        def.name.clone(),
                        Value::Function(Rc::new(function)),
                        true,
                        None,
                    );
                }
                Item::Main { body, insane } => {
                    if main.is_some() {
                        return Err(runtime_error("duplicate marmot main"));
                    }
                    main = Some(PreparedMain::Main {
                        body: body.clone(),
                        insane: *insane,
                    });
                }
                Item::HowlMain { body, insane } => {
                    if howl_main.is_some() {
                        return Err(runtime_error("duplicate howl marmot main"));
                    }
                    howl_main = Some(PreparedMain::HowlMain {
                        body: body.clone(),
                        insane: *insane,
                    });
                }
                _ => {}
            }
        }
        if main.is_some() && howl_main.is_some() {
            return Err(runtime_error(
                "cannot define both marmot main and howl marmot main",
            ));
        }
        Ok(howl_main.or(main))
    }

    fn register_mask(&mut self, item: &MaskDef) -> Result<(), Diagnostic> {
        if self.masks.contains_key(&item.name) || self.dens.contains_key(&item.name) {
            return Err(runtime_error(format!(
                "type {} is already defined",
                item.name
            )));
        }
        let mut methods = BTreeMap::new();
        for method in &item.methods {
            if methods
                .insert(method.name.clone(), method.clone())
                .is_some()
            {
                return Err(runtime_error(format!(
                    "duplicate method in mask {}",
                    item.name
                )));
            }
        }
        self.masks.insert(item.name.clone(), MaskType { methods });
        self.env.borrow_mut().define_unchecked(
            item.name.clone(),
            Value::MaskType(item.name.clone()),
            true,
            None,
        );
        Ok(())
    }

    fn register_den(&mut self, item: &DenDef) -> Result<(), Diagnostic> {
        if self.dens.contains_key(&item.name) || self.masks.contains_key(&item.name) {
            return Err(runtime_error(format!(
                "type {} is already defined",
                item.name
            )));
        }
        let mut local_fields = BTreeMap::new();
        let mut local_methods = BTreeMap::new();
        for member in &item.members {
            match member {
                DenMember::Field(field) => {
                    if local_fields
                        .insert(
                            field.name.clone(),
                            FieldSpec {
                                name: field.name.clone(),
                                type_name: field.type_name.clone(),
                                expr: field.expr.clone(),
                                access: field.access,
                                owner: item.name.clone(),
                            },
                        )
                        .is_some()
                    {
                        return Err(runtime_error(format!(
                            "duplicate field {}.{}",
                            item.name, field.name
                        )));
                    }
                }
                DenMember::Method(method) => {
                    if method.name == "init" && method.return_type.is_some() {
                        return Err(runtime_error(format!(
                            "{}.init cannot declare a return type",
                            item.name
                        )));
                    }
                    if local_methods
                        .insert(
                            method.name.clone(),
                            MethodSpec {
                                name: method.name.clone(),
                                params: method.params.clone(),
                                return_type: method.return_type.clone(),
                                body: method.body.clone(),
                                access: method.access,
                                owner: item.name.clone(),
                                closure: self.env.clone(),
                            },
                        )
                        .is_some()
                    {
                        return Err(runtime_error(format!(
                            "duplicate method {}.{}",
                            item.name, method.name
                        )));
                    }
                }
            }
        }
        self.dens.insert(
            item.name.clone(),
            DenType {
                name: item.name.clone(),
                parent: item.parent.clone(),
                masks: item.masks.clone(),
                local_fields,
                local_methods,
            },
        );
        self.env.borrow_mut().define_unchecked(
            item.name.clone(),
            Value::DenType(item.name.clone()),
            true,
            None,
        );
        Ok(())
    }

    fn validate_dens(&self) -> Result<(), Diagnostic> {
        for den in self.dens.values() {
            if let Some(parent) = &den.parent {
                if !self.dens.contains_key(parent) {
                    return Err(runtime_error(format!(
                        "parent den {parent} for {} is not defined",
                        den.name
                    )));
                }
            }
            for mask in &den.masks {
                if !self.masks.contains_key(mask) {
                    return Err(runtime_error(format!(
                        "mask {mask} for {} is not defined",
                        den.name
                    )));
                }
            }
        }
        for den in self.dens.values() {
            for mask_name in &den.masks {
                let mask = self.masks.get(mask_name).expect("validated mask");
                for (method_name, required) in &mask.methods {
                    let Some(method) = self.find_method(&den.name, method_name) else {
                        return Err(runtime_error(format!(
                            "{} wears {mask_name} but does not implement {method_name}",
                            den.name
                        )));
                    };
                    if !same_signature_method_mask(&method, required) {
                        return Err(runtime_error(format!(
                            "{}.{method_name} does not match mask {mask_name}.{method_name}",
                            den.name
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    fn execute_block(&mut self, statements: &[Stmt], env: EnvRef) -> Result<Control, Diagnostic> {
        let previous = self.env.clone();
        self.env = env;
        let result = (|| {
            for stmt in statements {
                match self.execute(stmt)? {
                    Control::None => {}
                    control => return Ok(control),
                }
            }
            Ok(Control::None)
        })();
        self.env = previous;
        result
    }

    fn execute(&mut self, stmt: &Stmt) -> Result<Control, Diagnostic> {
        match stmt {
            Stmt::Let {
                name,
                expr,
                type_name,
                is_const,
            } => {
                let value = self.evaluate(expr)?;
                self.define_current(name.clone(), value, *is_const, type_name.clone())?;
                Ok(Control::None)
            }
            Stmt::Expr(expr) => {
                self.evaluate(expr)?;
                Ok(Control::None)
            }
            Stmt::Squeak(exprs) => {
                let values = exprs
                    .iter()
                    .map(|expr| self.evaluate(expr).map(|value| format_value(&value)))
                    .collect::<Result<Vec<_>, _>>()?;
                self.output.borrow_mut().push(values.join(" "));
                Ok(Control::None)
            }
            Stmt::Panic(expr) => Err(runtime_error(format_value(&self.evaluate(expr)?))),
            Stmt::Expect(expr) => {
                let value = self.evaluate(expr)?;
                match value {
                    Value::Bool(true) => Ok(Control::None),
                    Value::Bool(false) => Err(runtime_error("expect failed")),
                    _ => Err(runtime_error("expect expression must be Bool")),
                }
            }
            Stmt::Trace(exprs) => {
                if self.trace_enabled {
                    let values = exprs
                        .iter()
                        .map(|expr| self.evaluate(expr).map(|value| format_value(&value)))
                        .collect::<Result<Vec<_>, _>>()?;
                    let payload = values.join(" ");
                    self.trace.borrow_mut().push(if payload.is_empty() {
                        "[trace]".to_string()
                    } else {
                        format!("[trace] {payload}")
                    });
                }
                Ok(Control::None)
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                let condition_value = self.evaluate(condition)?;
                if self.require_bool(&condition_value, "if condition")? {
                    self.execute_block(then_body, Environment::child(self.env.clone()))
                } else if let Some(else_body) = else_body {
                    match else_body {
                        ElseBody::If(stmt) => self.execute(stmt),
                        ElseBody::Block(body) => {
                            self.execute_block(body, Environment::child(self.env.clone()))
                        }
                    }
                } else {
                    Ok(Control::None)
                }
            }
            Stmt::While { condition, body } => {
                while {
                    let condition_value = self.evaluate(condition)?;
                    self.require_bool(&condition_value, "while condition")?
                } {
                    match self.execute_block(body, Environment::child(self.env.clone()))? {
                        Control::None => {}
                        Control::Continue => continue,
                        Control::Break => break,
                        control => return Ok(control),
                    }
                }
                Ok(Control::None)
            }
            Stmt::For {
                name,
                iterable,
                body,
                insane,
            } => {
                let iterable_value = self.evaluate(iterable)?;
                let values = self.iter_values(&iterable_value)?;
                if *insane {
                    self.insane_depth += 1;
                }
                for value in values {
                    let loop_env = Environment::child(self.env.clone());
                    self.define_in_env(&loop_env, name.clone(), value, false, None)?;
                    match self.execute_block(body, loop_env)? {
                        Control::None => {}
                        Control::Continue => continue,
                        Control::Break => break,
                        control => {
                            if *insane {
                                self.insane_depth -= 1;
                            }
                            return Ok(control);
                        }
                    }
                }
                if *insane {
                    self.insane_depth -= 1;
                }
                Ok(Control::None)
            }
            Stmt::Return(expr) => {
                let value = if let Some(expr) = expr {
                    self.evaluate(expr)?
                } else {
                    Value::Null
                };
                Ok(Control::Return(value))
            }
            Stmt::Break => Ok(Control::Break),
            Stmt::Continue => Ok(Control::Continue),
            Stmt::Try {
                body,
                catch_name,
                catch_body,
                insane,
            } => match self.execute_block(body, Environment::child(self.env.clone())) {
                Ok(control) => Ok(control),
                Err(err) => {
                    if *insane && catch_body.is_none() {
                        return Ok(Control::None);
                    }
                    let Some(catch_body) = catch_body else {
                        return Err(err);
                    };
                    let catch_env = Environment::child(self.env.clone());
                    self.define_in_env(
                        &catch_env,
                        catch_name.clone().unwrap_or_else(|| "err".to_string()),
                        Value::String(err.message),
                        false,
                        None,
                    )?;
                    self.execute_block(catch_body, catch_env)
                }
            },
            Stmt::InsaneBlock(body) => {
                self.insane_depth += 1;
                let result = self.execute_block(body, Environment::child(self.env.clone()));
                self.insane_depth -= 1;
                result
            }
        }
    }

    fn evaluate(&mut self, expr: &Expr) -> Result<Value, Diagnostic> {
        match expr {
            Expr::Literal(literal) => Ok(match literal {
                Literal::Null => Value::Null,
                Literal::Bool(value) => Value::Bool(*value),
                Literal::Int(value) => Value::Int(*value),
                Literal::Float(value) => Value::Float(*value),
                Literal::String(value) => Value::String(value.clone()),
            }),
            Expr::Var(name) => self.get_var(name),
            Expr::Array(items) => {
                let values = items
                    .iter()
                    .map(|item| self.evaluate(item))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Array(Rc::new(RefCell::new(values))))
            }
            Expr::Map(pairs) => {
                let mut values = BTreeMap::new();
                for (key_expr, value_expr) in pairs {
                    let key = self.evaluate(key_expr)?;
                    let Value::String(key) = key else {
                        return Err(runtime_error("map literal keys must be String"));
                    };
                    let value = self.evaluate(value_expr)?;
                    values.insert(key, value);
                }
                Ok(Value::Map(Rc::new(RefCell::new(values))))
            }
            Expr::Matrix(rows) => {
                let mut matrix_rows = Vec::new();
                let mut width = None;
                for row_expr in rows {
                    let row_value = self.evaluate(row_expr)?;
                    let Value::Array(row) = row_value else {
                        return Err(runtime_error("matrix row must be an array"));
                    };
                    let row = row.borrow().clone();
                    if let Some(width) = width {
                        if row.len() != width {
                            return Err(runtime_error("matrix rows must have the same length"));
                        }
                    } else {
                        width = Some(row.len());
                    }
                    matrix_rows.push(row);
                }
                Ok(Value::Matrix(Rc::new(RefCell::new(MatrixData {
                    rows: matrix_rows,
                }))))
            }
            Expr::Point { x, y } => {
                let x = self.evaluate(x)?;
                let y = self.evaluate(y)?;
                let (Value::Int(x), Value::Int(y)) = (x, y) else {
                    return Err(runtime_error("@point requires Int x and y"));
                };
                Ok(Value::Point(Point { x, y }))
            }
            Expr::Hatch { name, args } => {
                let args = self.eval_args(args)?;
                self.hatch(name, args)
            }
            Expr::Sniff => Ok(Value::String(String::new())),
            Expr::Unary { op, expr } => {
                let value = self.evaluate(expr)?;
                match op.as_str() {
                    "-" => match value {
                        Value::Int(value) => Ok(Value::Int(-value)),
                        Value::Float(value) => Ok(Value::Float(-value)),
                        _ => Err(runtime_error("- operand must be numeric")),
                    },
                    "!" => Ok(Value::Bool(!self.require_bool(&value, "! operand")?)),
                    _ => Err(runtime_error(format!("unknown unary operator {op}"))),
                }
            }
            Expr::Binary { left, op, right } => self.binary(left, op, right),
            Expr::Range { start, end } => {
                let start = self.evaluate(start)?;
                let end = self.evaluate(end)?;
                let (Value::Int(start), Value::Int(end)) = (start, end) else {
                    return Err(runtime_error("range bounds must be Int"));
                };
                Ok(Value::Range(start, end))
            }
            Expr::Call { callee, args } => {
                let callee = self.evaluate(callee)?;
                let args = self.eval_args(args)?;
                self.call_value(callee, args)
            }
            Expr::Index { target, args } => {
                let target = self.evaluate(target)?;
                let args = self.eval_args(args)?;
                self.get_index(&target, &args)
            }
            Expr::Member { target, name } => {
                let target = self.evaluate(target)?;
                self.get_member(&target, name)
            }
            Expr::Assign { target, value } => {
                let value = self.evaluate(value)?;
                self.assign_target(target, value)
            }
            Expr::Lambda { params, body } => Ok(Value::Lambda(Rc::new(LambdaFunction {
                params: params.clone(),
                body: body.clone(),
                closure: self.env.clone(),
            }))),
            Expr::Tunnel { left, right } => {
                let value = self.evaluate(left)?;
                self.eval_tunnel(value, right)
            }
            Expr::InsaneChoose(expr) => {
                let values = self.evaluate(expr)?;
                let mut values = self.iter_values(&values)?;
                if values.is_empty() {
                    Ok(Value::Null)
                } else {
                    Ok(values.remove(0))
                }
            }
            Expr::Wait(expr) => {
                if self.howl_depth == 0 {
                    return Err(runtime_error("wait can only be used inside howl context"));
                }
                let value = self.evaluate(expr)?;
                self.wait_value(value)
            }
            Expr::Scatter { expr, insane } => {
                if self.howl_depth == 0 {
                    return Err(runtime_error(
                        "scatter can only be used inside howl context",
                    ));
                }
                if *insane {
                    self.insane_depth += 1;
                }
                let value = self.evaluate(expr)?;
                if *insane {
                    self.insane_depth -= 1;
                }
                Ok(Value::Task(Rc::new(RefCell::new(TaskData { value }))))
            }
            Expr::Nest(items) => {
                if self.howl_depth == 0 {
                    return Err(runtime_error("nest can only be used inside howl context"));
                }
                let tasks = items
                    .iter()
                    .map(|item| self.evaluate(item))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::TaskGroup(tasks))
            }
        }
    }

    fn eval_args(&mut self, args: &[Expr]) -> Result<Vec<Value>, Diagnostic> {
        args.iter().map(|arg| self.evaluate(arg)).collect()
    }

    fn call_value(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, Diagnostic> {
        match callee {
            Value::Function(function) => self.call_user_function(&function, args),
            Value::Lambda(lambda) => self.call_lambda(&lambda, args),
            Value::Builtin(kind) => self.call_builtin(kind, args),
            Value::NativeMethod(method) => self.call_native_method(&method, args),
            Value::ObjectMethod(method) => {
                self.call_object_method(&method.object, &method.method, args)
            }
            _ => Err(runtime_error(format!(
                "{} is not callable",
                format_value(&callee)
            ))),
        }
    }

    fn call_user_function(
        &mut self,
        function: &UserFunction,
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        if args.len() != function.params.len() {
            return Err(runtime_error(format!(
                "{} expects {} arguments, got {}",
                function.name,
                function.params.len(),
                args.len()
            )));
        }
        if function.howl {
            let previous_howl = self.howl_depth;
            self.howl_depth += 1;
            let result = self.call_user_function_body(function, args);
            self.howl_depth = previous_howl;
            return Ok(Value::Task(Rc::new(RefCell::new(TaskData {
                value: result?,
            }))));
        }
        self.call_user_function_body(function, args)
    }

    fn call_user_function_body(
        &mut self,
        function: &UserFunction,
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let env = Environment::child(function.closure.clone());
        for (param, value) in function.params.iter().zip(args) {
            if let Some(type_name) = &param.type_name {
                self.check_type(&value, type_name, &format!("parameter {}", param.name))?;
            }
            self.define_in_env(
                &env,
                param.name.clone(),
                value,
                false,
                param.type_name.clone(),
            )?;
        }
        match self.execute_block(&function.body, env)? {
            Control::Return(value) => {
                if let Some(type_name) = &function.return_type {
                    self.check_type(
                        &value,
                        type_name,
                        &format!("return value of {}", function.name),
                    )?;
                }
                Ok(value)
            }
            Control::None => {
                let value = Value::Null;
                if let Some(type_name) = &function.return_type {
                    self.check_type(
                        &value,
                        type_name,
                        &format!("return value of {}", function.name),
                    )?;
                }
                Ok(value)
            }
            _ => Err(runtime_error("loop control outside loop")),
        }
    }

    fn call_lambda(
        &mut self,
        lambda: &LambdaFunction,
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        if args.len() != lambda.params.len() {
            return Err(runtime_error(format!(
                "lambda expects {} arguments, got {}",
                lambda.params.len(),
                args.len()
            )));
        }
        let env = Environment::child(lambda.closure.clone());
        for (name, value) in lambda.params.iter().zip(args) {
            self.define_in_env(&env, name.clone(), value, false, None)?;
        }
        match &lambda.body {
            LambdaBody::Expr(expr) => {
                let previous = self.env.clone();
                self.env = env;
                let result = self.evaluate(expr);
                self.env = previous;
                result
            }
            LambdaBody::Block(body) => match self.execute_block(body, env)? {
                Control::Return(value) => Ok(value),
                Control::None => Ok(Value::Null),
                _ => Err(runtime_error("loop control outside loop")),
            },
        }
    }

    fn eval_tunnel(&mut self, value: Value, right: &Expr) -> Result<Value, Diagnostic> {
        if let Expr::Call { callee, args } = right {
            let callee = self.evaluate(callee)?;
            let mut all_args = vec![value];
            all_args.extend(self.eval_args(args)?);
            self.call_value(callee, all_args)
        } else {
            let callee = self.evaluate(right)?;
            self.call_value(callee, vec![value])
        }
    }

    fn binary(&mut self, left: &Expr, op: &str, right: &Expr) -> Result<Value, Diagnostic> {
        if op == "&&" {
            let left = self.evaluate(left)?;
            if !self.require_bool(&left, "left side of &&")? {
                return Ok(Value::Bool(false));
            }
            let right = self.evaluate(right)?;
            return Ok(Value::Bool(self.require_bool(&right, "right side of &&")?));
        }
        if op == "||" {
            let left = self.evaluate(left)?;
            if self.require_bool(&left, "left side of ||")? {
                return Ok(Value::Bool(true));
            }
            let right = self.evaluate(right)?;
            return Ok(Value::Bool(self.require_bool(&right, "right side of ||")?));
        }
        let left = self.evaluate(left)?;
        let right = self.evaluate(right)?;
        self.apply_binary(op, &left, &right)
    }

    fn apply_binary(&self, op: &str, left: &Value, right: &Value) -> Result<Value, Diagnostic> {
        match op {
            "+" => match (left, right) {
                (Value::Point(a), Value::Point(b)) => Ok(Value::Point(Point {
                    x: a.x + b.x,
                    y: a.y + b.y,
                })),
                (Value::String(_), _) | (_, Value::String(_)) => Ok(Value::String(format!(
                    "{}{}",
                    format_value(left),
                    format_value(right)
                ))),
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
                (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
                (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
                (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
                _ => Err(runtime_error("unsupported + operands")),
            },
            "-" => numeric_binary(left, right, |a, b| a - b, |a, b| a - b),
            "*" => numeric_binary(left, right, |a, b| a * b, |a, b| a * b),
            "/" => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Float(*a as f64 / *b as f64)),
                _ => numeric_binary(left, right, |a, b| a / b, |a, b| a / b),
            },
            "%" => match (left, right) {
                (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
                _ => Err(runtime_error("unsupported % operands")),
            },
            "==" => Ok(Value::Bool(value_eq(left, right))),
            "!=" => Ok(Value::Bool(!value_eq(left, right))),
            "<" | "<=" | ">" | ">=" => compare_values(op, left, right),
            _ => Err(runtime_error(format!("unknown operator {op}"))),
        }
    }

    fn assign_target(&mut self, target: &Expr, value: Value) -> Result<Value, Diagnostic> {
        match target {
            Expr::Var(name) => {
                self.assign_var(name, value.clone())?;
                Ok(value)
            }
            Expr::Index { target, args } => {
                let object = self.evaluate(target)?;
                let args = self.eval_args(args)?;
                self.set_index(&object, &args, value.clone())?;
                Ok(value)
            }
            Expr::Member { target, name } => {
                let object = self.evaluate(target)?;
                self.set_member(&object, name, value.clone())?;
                Ok(value)
            }
            _ => Err(runtime_error("invalid assignment target")),
        }
    }

    fn get_index(&self, target: &Value, args: &[Value]) -> Result<Value, Diagnostic> {
        match target {
            Value::Matrix(matrix) => matrix_get(&matrix.borrow(), args, self.insane_depth > 0),
            Value::Array(array) => {
                let index = one_int_arg(args, "array index must be one Int")?;
                let array = array.borrow();
                if index < 0 || index as usize >= array.len() {
                    if self.insane_depth > 0 {
                        return Ok(Value::Null);
                    }
                    return Err(runtime_error(format!("array index out of bounds: {index}")));
                }
                Ok(array[index as usize].clone())
            }
            Value::String(string) => {
                let index = one_int_arg(args, "string index must be one Int")?;
                let chars = string.chars().collect::<Vec<_>>();
                if index < 0 || index as usize >= chars.len() {
                    if self.insane_depth > 0 {
                        return Ok(Value::Null);
                    }
                    return Err(runtime_error(format!(
                        "string index out of bounds: {index}"
                    )));
                }
                Ok(Value::String(chars[index as usize].to_string()))
            }
            Value::Map(map) => {
                if args.len() != 1 {
                    return Err(runtime_error("map index must be one String"));
                }
                let Value::String(key) = &args[0] else {
                    return Err(runtime_error("map index must be one String"));
                };
                map.borrow()
                    .get(key)
                    .cloned()
                    .ok_or_else(|| {
                        if self.insane_depth > 0 {
                            runtime_error("__IMM_NULL__")
                        } else {
                            runtime_error(format!("map key not found: {key}"))
                        }
                    })
                    .or_else(|err| {
                        if err.message == "__IMM_NULL__" {
                            Ok(Value::Null)
                        } else {
                            Err(err)
                        }
                    })
            }
            _ => Err(runtime_error(format!(
                "{} is not indexable",
                type_name(target)
            ))),
        }
    }

    fn set_index(
        &mut self,
        target: &Value,
        args: &[Value],
        value: Value,
    ) -> Result<(), Diagnostic> {
        match target {
            Value::Matrix(matrix) => {
                matrix_set(&mut matrix.borrow_mut(), args, value, self.insane_depth > 0)
            }
            Value::Array(array) => {
                let index = one_int_arg(args, "array index must be one Int")?;
                let mut array = array.borrow_mut();
                if index < 0 || index as usize >= array.len() {
                    if self.insane_depth > 0 {
                        return Ok(());
                    }
                    return Err(runtime_error(format!("array index out of bounds: {index}")));
                }
                array[index as usize] = value;
                Ok(())
            }
            Value::Map(map) => {
                if args.len() != 1 {
                    return Err(runtime_error("map index must be one String"));
                }
                let Value::String(key) = &args[0] else {
                    return Err(runtime_error("map index must be one String"));
                };
                map.borrow_mut().insert(key.clone(), value);
                Ok(())
            }
            _ => Err(runtime_error(format!(
                "{} is not assignable by index",
                type_name(target)
            ))),
        }
    }

    fn get_member(&self, target: &Value, name: &str) -> Result<Value, Diagnostic> {
        match target {
            Value::Null => Err(runtime_error("null has no members")),
            Value::Namespace(namespace) => namespace.values.get(name).cloned().ok_or_else(|| {
                runtime_error(format!("{}.{} is not defined", namespace.name, name))
            }),
            Value::Response(response) => match name {
                "status" => Ok(Value::Int(response.status)),
                "headers" => Ok(Value::Map(Rc::new(RefCell::new(response.headers.clone())))),
                "body" => Ok(Value::String(response.body.clone())),
                "url" => Ok(Value::String(response.url.clone())),
                "ok" => Ok(Value::Bool(response.ok)),
                "json" => Ok(Value::NativeMethod(Rc::new(NativeMethod {
                    receiver: target.clone(),
                    kind: NativeMethodKind::ResponseJson,
                }))),
                "text" => Ok(Value::NativeMethod(Rc::new(NativeMethod {
                    receiver: target.clone(),
                    kind: NativeMethodKind::ResponseText,
                }))),
                _ => Err(runtime_error(format!("Response has no member {name}"))),
            },
            Value::Task(_) => match name {
                "done" => Ok(Value::NativeMethod(Rc::new(NativeMethod {
                    receiver: target.clone(),
                    kind: NativeMethodKind::TaskDone,
                }))),
                "cancel" => Ok(Value::NativeMethod(Rc::new(NativeMethod {
                    receiver: target.clone(),
                    kind: NativeMethodKind::TaskCancel,
                }))),
                _ => Err(runtime_error(format!("Task has no member {name}"))),
            },
            Value::UnderProxy { object, parent } => self.get_under_member(object, parent, name),
            Value::ObjectView { object, mask } => self.get_object_view_member(object, mask, name),
            Value::Object(object) => self.get_object_member(object, name),
            Value::Point(point) => match name {
                "x" => Ok(Value::Int(point.x)),
                "y" => Ok(Value::Int(point.y)),
                _ => Err(runtime_error(format!("Point has no member {name}"))),
            },
            Value::Matrix(_) => matrix_method(target, name),
            Value::Array(_) => match name {
                "len" => Ok(Value::NativeMethod(Rc::new(NativeMethod {
                    receiver: target.clone(),
                    kind: NativeMethodKind::ArrayLen,
                }))),
                _ => Err(runtime_error(format!("Array has no member {name}"))),
            },
            Value::Map(_) => match name {
                "len" => Ok(Value::NativeMethod(Rc::new(NativeMethod {
                    receiver: target.clone(),
                    kind: NativeMethodKind::MapLen,
                }))),
                _ => Err(runtime_error(format!("Map has no member {name}"))),
            },
            Value::String(_) => string_method(target, name),
            _ => Err(runtime_error(format!(
                "{} has no member {name}",
                type_name(target)
            ))),
        }
    }

    fn set_member(&mut self, target: &Value, name: &str, value: Value) -> Result<(), Diagnostic> {
        match target {
            Value::ObjectView { mask, .. } => Err(runtime_error(format!(
                "mask {mask} has no assignable field {name}"
            ))),
            Value::Object(object) => {
                let den_name = object.borrow().den_name.clone();
                let Some(field) = self.find_field(&den_name, name) else {
                    return Err(runtime_error(format!("{den_name} has no field {name}")));
                };
                if !self.can_access(field.access, &field.owner) {
                    return Err(runtime_error(format!("{den_name}.{name} is private")));
                }
                if let Some(type_name) = &field.type_name {
                    self.check_type(
                        &value,
                        type_name,
                        &format!("{}.{}", field.owner, field.name),
                    )?;
                }
                object
                    .borrow_mut()
                    .fields
                    .insert(name.to_string(), Some(value));
                Ok(())
            }
            _ => Err(runtime_error(format!(
                "{} has no assignable member {name}",
                type_name(target)
            ))),
        }
    }

    fn get_object_view_member(
        &self,
        object: &ObjRef,
        mask: &str,
        name: &str,
    ) -> Result<Value, Diagnostic> {
        let mask_type = self.masks.get(mask).expect("known mask");
        if !mask_type.methods.contains_key(name) {
            return Err(runtime_error(format!("mask {mask} has no member {name}")));
        }
        let den_name = object.borrow().den_name.clone();
        let Some(method) = self.find_method(&den_name, name) else {
            return Err(runtime_error(format!("{den_name} has no member {name}")));
        };
        if !self.can_access(method.access, &method.owner) {
            return Err(runtime_error(format!("{den_name}.{name} is private")));
        }
        Ok(Value::ObjectMethod(Rc::new(ObjectBoundMethod {
            object: object.clone(),
            method,
        })))
    }

    fn get_object_member(&self, object: &ObjRef, name: &str) -> Result<Value, Diagnostic> {
        let den_name = object.borrow().den_name.clone();
        if let Some(field) = self.find_field(&den_name, name) {
            if !self.can_access(field.access, &field.owner) {
                return Err(runtime_error(format!("{den_name}.{name} is private")));
            }
            return object
                .borrow()
                .fields
                .get(name)
                .cloned()
                .flatten()
                .ok_or_else(|| runtime_error(format!("{den_name}.{name} is not initialized")));
        }
        if let Some(method) = self.find_method(&den_name, name) {
            if !self.can_access(method.access, &method.owner) {
                return Err(runtime_error(format!("{den_name}.{name} is private")));
            }
            return Ok(Value::ObjectMethod(Rc::new(ObjectBoundMethod {
                object: object.clone(),
                method,
            })));
        }
        Err(runtime_error(format!("{den_name} has no member {name}")))
    }

    fn get_under_member(
        &self,
        object: &ObjRef,
        parent: &str,
        name: &str,
    ) -> Result<Value, Diagnostic> {
        let method = if name == "init" {
            self.dens
                .get(parent)
                .and_then(|den| den.local_methods.get("init"))
                .cloned()
        } else {
            self.find_method(parent, name)
        };
        let Some(method) = method else {
            return Err(runtime_error(format!("parent den has no method {name}")));
        };
        Ok(Value::ObjectMethod(Rc::new(ObjectBoundMethod {
            object: object.clone(),
            method,
        })))
    }

    fn hatch(&mut self, name: &str, args: Vec<Value>) -> Result<Value, Diagnostic> {
        if !self.dens.contains_key(name) {
            return Err(runtime_error(format!("den {name} is not defined")));
        }
        let object = Rc::new(RefCell::new(ObjectInstance {
            den_name: name.to_string(),
            fields: BTreeMap::new(),
            store_ids: BTreeMap::new(),
        }));
        self.initialize_fields(&object)?;
        if let Some(init) = self
            .dens
            .get(name)
            .and_then(|den| den.local_methods.get("init"))
            .cloned()
        {
            self.call_object_method(&object, &init, args)?;
        } else if !args.is_empty() {
            return Err(runtime_error(format!(
                "{name} has no init and expects 0 arguments"
            )));
        }
        self.check_initialized_fields(&object)?;
        Ok(Value::Object(object))
    }

    fn initialize_fields(&mut self, object: &ObjRef) -> Result<(), Diagnostic> {
        let den_name = object.borrow().den_name.clone();
        for field in self.field_order(&den_name)? {
            object.borrow_mut().fields.insert(field.name.clone(), None);
            if let Some(expr) = &field.expr {
                let env = Environment::child(self.env.clone());
                self.define_in_env(
                    &env,
                    "self".to_string(),
                    Value::Object(object.clone()),
                    true,
                    None,
                )?;
                let previous = self.env.clone();
                self.env = env;
                self.current_den.push(field.owner.clone());
                let value = self.evaluate(expr);
                self.current_den.pop();
                self.env = previous;
                let value = value?;
                if let Some(type_name) = &field.type_name {
                    self.check_type(
                        &value,
                        type_name,
                        &format!("{}.{}", field.owner, field.name),
                    )?;
                }
                object
                    .borrow_mut()
                    .fields
                    .insert(field.name.clone(), Some(value));
            }
        }
        Ok(())
    }

    fn check_initialized_fields(&self, object: &ObjRef) -> Result<(), Diagnostic> {
        let den_name = object.borrow().den_name.clone();
        for field in self.field_order(&den_name)? {
            if object
                .borrow()
                .fields
                .get(&field.name)
                .is_none_or(|value| value.is_none())
            {
                return Err(runtime_error(format!(
                    "{den_name}.{} is not initialized",
                    field.name
                )));
            }
        }
        Ok(())
    }

    fn call_object_method(
        &mut self,
        object: &ObjRef,
        method: &MethodSpec,
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        if args.len() != method.params.len() {
            return Err(runtime_error(format!(
                "{}.{} expects {} arguments, got {}",
                method.owner,
                method.name,
                method.params.len(),
                args.len()
            )));
        }
        let env = Environment::child(method.closure.clone());
        self.define_in_env(
            &env,
            "self".to_string(),
            Value::Object(object.clone()),
            true,
            None,
        )?;
        if let Some(parent) = self
            .dens
            .get(&method.owner)
            .and_then(|den| den.parent.clone())
        {
            self.define_in_env(
                &env,
                "under".to_string(),
                Value::UnderProxy {
                    object: object.clone(),
                    parent,
                },
                true,
                None,
            )?;
        }
        for (param, value) in method.params.iter().zip(args) {
            if let Some(type_name) = &param.type_name {
                self.check_type(&value, type_name, &format!("parameter {}", param.name))?;
            }
            self.define_in_env(
                &env,
                param.name.clone(),
                value,
                false,
                param.type_name.clone(),
            )?;
        }
        self.current_den.push(method.owner.clone());
        let result = self.execute_block(&method.body, env);
        self.current_den.pop();
        match result? {
            Control::Return(value) => {
                if method.name == "init" {
                    if !matches!(value, Value::Null) {
                        return Err(runtime_error(format!(
                            "{}.init cannot return a value",
                            method.owner
                        )));
                    }
                    Ok(Value::Null)
                } else {
                    if let Some(type_name) = &method.return_type {
                        self.check_type(
                            &value,
                            type_name,
                            &format!("return value of {}.{}", method.owner, method.name),
                        )?;
                    }
                    Ok(value)
                }
            }
            Control::None => {
                let value = Value::Null;
                if method.name != "init" {
                    if let Some(type_name) = &method.return_type {
                        self.check_type(
                            &value,
                            type_name,
                            &format!("return value of {}.{}", method.owner, method.name),
                        )?;
                    }
                }
                Ok(value)
            }
            _ => Err(runtime_error("loop control outside loop")),
        }
    }

    fn can_access(&self, access: Access, owner: &str) -> bool {
        access == Access::Fur
            || self
                .current_den
                .last()
                .is_some_and(|current| current == owner)
    }

    fn find_field(&self, den_name: &str, field: &str) -> Option<FieldSpec> {
        let den = self.dens.get(den_name)?;
        if let Some(local) = den.local_fields.get(field) {
            return Some(local.clone());
        }
        den.parent
            .as_ref()
            .and_then(|parent| self.find_field(parent, field))
    }

    fn find_method(&self, den_name: &str, method: &str) -> Option<MethodSpec> {
        let den = self.dens.get(den_name)?;
        if method != "init" {
            if let Some(local) = den.local_methods.get(method) {
                return Some(local.clone());
            }
        }
        den.parent
            .as_ref()
            .and_then(|parent| self.find_method(parent, method))
    }

    fn field_order(&self, den_name: &str) -> Result<Vec<FieldSpec>, Diagnostic> {
        let den = self
            .dens
            .get(den_name)
            .ok_or_else(|| runtime_error(format!("den {den_name} is not defined")))?;
        let mut fields = if let Some(parent) = &den.parent {
            self.field_order(parent)?
        } else {
            Vec::new()
        };
        fields.extend(den.local_fields.values().cloned());
        Ok(fields)
    }

    fn wait_value(&mut self, value: Value) -> Result<Value, Diagnostic> {
        match value {
            Value::Task(task) => {
                let value = task.borrow().value.clone();
                if matches!(value, Value::Task(_) | Value::TaskGroup(_)) {
                    self.wait_value(value)
                } else {
                    Ok(value)
                }
            }
            Value::TaskGroup(values) => {
                let mut results = Vec::new();
                for value in values {
                    results.push(self.wait_value(value)?);
                }
                Ok(Value::Array(Rc::new(RefCell::new(results))))
            }
            other => Err(runtime_error(format!(
                "wait expects Task, got {}",
                type_name(&other)
            ))),
        }
    }

    fn iter_values(&self, value: &Value) -> Result<Vec<Value>, Diagnostic> {
        match value {
            Value::Null => Err(runtime_error("cannot iterate null")),
            Value::Array(values) => Ok(values.borrow().clone()),
            Value::String(value) => Ok(value
                .chars()
                .map(|ch| Value::String(ch.to_string()))
                .collect::<Vec<_>>()),
            Value::Range(start, end) => Ok((*start..*end).map(Value::Int).collect()),
            Value::Map(map) => Ok(map
                .borrow()
                .iter()
                .map(|(key, value)| {
                    Value::Array(Rc::new(RefCell::new(vec![
                        Value::String(key.clone()),
                        value.clone(),
                    ])))
                })
                .collect()),
            _ => Err(runtime_error(format!(
                "{} is not iterable",
                type_name(value)
            ))),
        }
    }

    fn define_current(
        &mut self,
        name: String,
        value: Value,
        is_const: bool,
        type_name: Option<String>,
    ) -> Result<(), Diagnostic> {
        let env = self.env.clone();
        self.define_in_env(&env, name, value, is_const, type_name)
    }

    fn define_in_env(
        &self,
        env: &EnvRef,
        name: String,
        value: Value,
        is_const: bool,
        type_name: Option<String>,
    ) -> Result<(), Diagnostic> {
        if let Some(type_name) = &type_name {
            self.check_type(&value, type_name, &name)?;
        }
        env.borrow_mut().values.insert(
            name,
            Cell {
                value,
                is_const,
                type_name,
            },
        );
        Ok(())
    }

    fn get_var(&self, name: &str) -> Result<Value, Diagnostic> {
        let cell = find_cell(&self.env, name)
            .ok_or_else(|| runtime_error(format!("{name} is not defined")))?;
        self.apply_type_view(cell.value, cell.type_name)
    }

    fn assign_var(&self, name: &str, value: Value) -> Result<(), Diagnostic> {
        let env = find_env(&self.env, name)
            .ok_or_else(|| runtime_error(format!("{name} is not defined")))?;
        let mut env_ref = env.borrow_mut();
        let cell = env_ref.values.get_mut(name).expect("found cell");
        if cell.is_const {
            return Err(runtime_error(format!("{name} is a stash constant")));
        }
        if let Some(type_name) = &cell.type_name {
            self.check_type(&value, type_name, name)?;
        }
        cell.value = value;
        Ok(())
    }

    fn apply_type_view(
        &self,
        value: Value,
        type_name: Option<String>,
    ) -> Result<Value, Diagnostic> {
        if let (Value::Object(object), Some(type_name)) = (&value, type_name.as_ref()) {
            let type_ref = parse_type_ref(type_name)?;
            if self.masks.contains_key(&type_ref.name) {
                return Ok(Value::ObjectView {
                    object: object.clone(),
                    mask: type_ref.name,
                });
            }
        }
        Ok(value)
    }

    fn require_bool(&self, value: &Value, label: &str) -> Result<bool, Diagnostic> {
        if let Value::Bool(value) = value {
            Ok(*value)
        } else {
            Err(runtime_error(format!("{label} must be Bool")))
        }
    }

    fn call_builtin(&mut self, kind: BuiltinKind, args: Vec<Value>) -> Result<Value, Diagnostic> {
        match kind {
            BuiltinKind::Len => {
                require_arg_count("len", &args, 1)?;
                Ok(Value::Int(self.iter_len(&args[0])? as i64))
            }
            BuiltinKind::Type => {
                require_arg_count("type", &args, 1)?;
                Ok(Value::String(type_name(&args[0]).to_string()))
            }
            BuiltinKind::Str => {
                require_arg_count("str", &args, 1)?;
                Ok(Value::String(format_value(&args[0])))
            }
            BuiltinKind::Int => {
                require_arg_count("int", &args, 1)?;
                match &args[0] {
                    Value::Int(value) => Ok(Value::Int(*value)),
                    Value::Float(value) => Ok(Value::Int(*value as i64)),
                    Value::String(value) => value
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|err| runtime_error(err.to_string())),
                    _ => Err(runtime_error("int expects Int, Float, or String")),
                }
            }
            BuiltinKind::Float => {
                require_arg_count("float", &args, 1)?;
                match &args[0] {
                    Value::Int(value) => Ok(Value::Float(*value as f64)),
                    Value::Float(value) => Ok(Value::Float(*value)),
                    Value::String(value) => value
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|err| runtime_error(err.to_string())),
                    _ => Err(runtime_error("float expects Int, Float, or String")),
                }
            }
            BuiltinKind::Bool => {
                require_arg_count("bool", &args, 1)?;
                Ok(Value::Bool(truthy(&args[0])))
            }
            BuiltinKind::Map => {
                require_arg_count("map", &args, 2)?;
                let values = self.iter_values(&args[0])?;
                let mapped = values
                    .into_iter()
                    .map(|value| self.call_value(args[1].clone(), vec![value]))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Array(Rc::new(RefCell::new(mapped))))
            }
            BuiltinKind::Filter => {
                require_arg_count("filter", &args, 2)?;
                let values = self.iter_values(&args[0])?;
                let mut filtered = Vec::new();
                for value in values {
                    let keep = self.call_value(args[1].clone(), vec![value.clone()])?;
                    if self.require_bool(&keep, "filter predicate")? {
                        filtered.push(value);
                    }
                }
                Ok(Value::Array(Rc::new(RefCell::new(filtered))))
            }
            BuiltinKind::Reduce => {
                require_arg_count("reduce", &args, 3)?;
                let mut acc = args[1].clone();
                for value in self.iter_values(&args[0])? {
                    acc = self.call_value(args[2].clone(), vec![acc, value])?;
                }
                Ok(acc)
            }
            BuiltinKind::Nap => {
                require_arg_count("nap", &args, 1)?;
                let Value::Int(ms) = args[0] else {
                    return Err(runtime_error("nap expects Int milliseconds"));
                };
                if ms < 0 {
                    return Err(runtime_error("nap milliseconds must be >= 0"));
                }
                Ok(Value::Task(Rc::new(RefCell::new(TaskData {
                    value: Value::Null,
                }))))
            }
            BuiltinKind::MathAbs => {
                require_arg_count("math.abs", &args, 1)?;
                match args[0] {
                    Value::Int(value) => Ok(Value::Int(value.abs())),
                    Value::Float(value) => Ok(Value::Float(value.abs())),
                    _ => Err(runtime_error("math.abs expects number")),
                }
            }
            BuiltinKind::MathMin => min_max_builtin("math.min", &args, true),
            BuiltinKind::MathMax => min_max_builtin("math.max", &args, false),
            BuiltinKind::MathSqrt => {
                require_arg_count("math.sqrt", &args, 1)?;
                Ok(Value::Float(number_arg(&args[0], "math.sqrt")?.sqrt()))
            }
            BuiltinKind::MathFloor => {
                require_arg_count("math.floor", &args, 1)?;
                Ok(Value::Int(
                    number_arg(&args[0], "math.floor")?.floor() as i64
                ))
            }
            BuiltinKind::MathCeil => {
                require_arg_count("math.ceil", &args, 1)?;
                Ok(Value::Int(number_arg(&args[0], "math.ceil")?.ceil() as i64))
            }
            BuiltinKind::MathRandom => {
                require_arg_count("math.random", &args, 0)?;
                Ok(Value::Float(0.5))
            }
            BuiltinKind::PathBfs => self.path_search(args, false),
            BuiltinKind::PathAstar => self.path_search(args, true),
            BuiltinKind::ChaserDirection => {
                require_arg_count("chaser.direction", &args, 2)?;
                let src = require_point(&args[0], "chaser.direction expects Point, Point")?;
                let dst = require_point(&args[1], "chaser.direction expects Point, Point")?;
                Ok(Value::String(chaser_direction(src, dst).to_string()))
            }
            BuiltinKind::ChaserStep => {
                require_arg_count("chaser.step", &args, 2)?;
                let pos = require_point(&args[0], "chaser.step expects Point, String")?;
                let Value::String(direction) = &args[1] else {
                    return Err(runtime_error("chaser.step expects Point, String"));
                };
                let point = match direction.as_str() {
                    "UP" => Point {
                        x: pos.x,
                        y: pos.y - 1,
                    },
                    "RIGHT" => Point {
                        x: pos.x + 1,
                        y: pos.y,
                    },
                    "DOWN" => Point {
                        x: pos.x,
                        y: pos.y + 1,
                    },
                    "LEFT" => Point {
                        x: pos.x - 1,
                        y: pos.y,
                    },
                    "STAY" => pos,
                    _ => {
                        return Err(runtime_error(format!(
                            "unknown CHaser direction {direction}"
                        )))
                    }
                };
                Ok(Value::Point(point))
            }
            BuiltinKind::ChaserParseField => {
                require_arg_count("chaser.parse_field", &args, 1)?;
                let Value::Array(lines) = &args[0] else {
                    return Err(runtime_error("chaser.parse_field expects Array<String>"));
                };
                let mut rows = Vec::new();
                for line in lines.borrow().iter() {
                    let Value::String(line) = line else {
                        return Err(runtime_error("chaser.parse_field expects Array<String>"));
                    };
                    rows.push(
                        line.chars()
                            .map(|ch| Value::String(ch.to_string()))
                            .collect(),
                    );
                }
                Ok(Value::Matrix(Rc::new(RefCell::new(MatrixData { rows }))))
            }
            BuiltinKind::ChaserSafeMoves => {
                require_arg_count("chaser.safe_moves", &args, 3)?;
                let moves = chaser_safe_moves(&args[0], &args[1], &args[2])?;
                Ok(Value::Array(Rc::new(RefCell::new(moves))))
            }
            BuiltinKind::ChaserRandomMove => {
                require_arg_count("chaser.random_move", &args, 3)?;
                let moves = chaser_safe_moves(&args[0], &args[1], &args[2])?;
                if moves.is_empty() {
                    Ok(Value::String("STAY".to_string()))
                } else {
                    let src = require_point(
                        &args[1],
                        "chaser.random_move expects Matrix, Point, wall value",
                    )?;
                    let Value::Point(dst) = moves[0] else {
                        unreachable!();
                    };
                    Ok(Value::String(chaser_direction(src, dst).to_string()))
                }
            }
            BuiltinKind::StoreOpen => {
                require_arg_count("store.open", &args, 1)?;
                let Value::String(path) = &args[0] else {
                    return Err(runtime_error("store.open expects String path"));
                };
                let path = if Path::new(path).is_absolute() {
                    PathBuf::from(path)
                } else {
                    self.source_path
                        .as_ref()
                        .and_then(|path| path.parent().map(Path::to_path_buf))
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(path)
                };
                Ok(Value::Store(Rc::new(RefCell::new(StoreDatabase::open(
                    path,
                )?))))
            }
            BuiltinKind::StoreSave => self.store_save(args),
            BuiltinKind::StoreLoad => self.store_load(args),
            BuiltinKind::StoreAll => self.store_all(args),
            BuiltinKind::StoreFind => self.store_find(args),
            BuiltinKind::StoreGet => {
                let matches = self.store_find(args)?;
                if let Value::Array(values) = matches {
                    Ok(values.borrow().first().cloned().unwrap_or(Value::Null))
                } else {
                    unreachable!()
                }
            }
            BuiltinKind::StoreDelete => {
                require_arg_count("store.delete", &args, 3)?;
                let db = require_store(&args[0])?;
                let den_name = require_den_type_name(&args[1])?;
                let Value::Int(id) = args[2] else {
                    return Err(runtime_error("store id must be Int"));
                };
                let mut db = db.borrow_mut();
                let existed = db
                    .records_for_mut(&den_name)
                    .and_then(|records| {
                        records
                            .as_object_mut()
                            .map(|object| object.remove(&id.to_string()).is_some())
                    })
                    .unwrap_or(false);
                if existed {
                    db.flush()?;
                }
                Ok(Value::Bool(existed))
            }
            BuiltinKind::StoreCount => {
                require_arg_count("store.count", &args, 2)?;
                let db = require_store(&args[0])?;
                let den_name = require_den_type_name(&args[1])?;
                let count = db.borrow().records_for(&den_name).map_or(0, |records| {
                    records.as_object().map_or(0, |object| object.len())
                });
                Ok(Value::Int(count as i64))
            }
            BuiltinKind::StoreClear => {
                require_arg_count("store.clear", &args, 2)?;
                let db = require_store(&args[0])?;
                let den_name = require_den_type_name(&args[1])?;
                let mut db = db.borrow_mut();
                let count = db.records_for(&den_name).map_or(0, |records| {
                    records.as_object().map_or(0, |object| object.len())
                });
                db.set_records_for(&den_name, json!({}));
                db.flush()?;
                Ok(Value::Int(count as i64))
            }
            BuiltinKind::WebGrab => {
                require_arg_count("web.grab", &args, 1)?;
                self.web_grab(&args[0])
            }
            BuiltinKind::WebFetch => {
                require_arg_count("web.fetch", &args, 1)?;
                let response = self.web_grab(&args[0])?;
                Ok(Value::Task(Rc::new(RefCell::new(TaskData {
                    value: response,
                }))))
            }
            BuiltinKind::TickNow => {
                require_arg_count("tick.now", &args, 0)?;
                let millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|duration| duration.as_millis() as i64)
                    .unwrap_or_default();
                Ok(Value::Int(millis))
            }
        }
    }

    fn call_native_method(
        &mut self,
        method: &NativeMethod,
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        match method.kind {
            NativeMethodKind::ArrayLen => {
                require_arg_count("Array.len", &args, 0)?;
                if let Value::Array(values) = &method.receiver {
                    Ok(Value::Int(values.borrow().len() as i64))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::MapLen => {
                require_arg_count("Map.len", &args, 0)?;
                if let Value::Map(values) = &method.receiver {
                    Ok(Value::Int(values.borrow().len() as i64))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::StringLen => {
                require_arg_count("String.len", &args, 0)?;
                if let Value::String(value) = &method.receiver {
                    Ok(Value::Int(value.chars().count() as i64))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::StringToInt => {
                require_arg_count("String.to_int", &args, 0)?;
                if let Value::String(value) = &method.receiver {
                    value
                        .parse::<i64>()
                        .map(Value::Int)
                        .map_err(|err| runtime_error(err.to_string()))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::StringToFloat => {
                require_arg_count("String.to_float", &args, 0)?;
                if let Value::String(value) = &method.receiver {
                    value
                        .parse::<f64>()
                        .map(Value::Float)
                        .map_err(|err| runtime_error(err.to_string()))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::StringToBool => {
                require_arg_count("String.to_bool", &args, 0)?;
                if let Value::String(value) = &method.receiver {
                    Ok(Value::Bool(matches!(
                        value.to_lowercase().as_str(),
                        "true" | "1" | "yes"
                    )))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::MatrixWidth => matrix_zero_arg(&method.receiver, &args, |matrix| {
                Value::Int(matrix_width(matrix) as i64)
            }),
            NativeMethodKind::MatrixHeight => matrix_zero_arg(&method.receiver, &args, |matrix| {
                Value::Int(matrix_height(matrix) as i64)
            }),
            NativeMethodKind::MatrixPoints => matrix_zero_arg(&method.receiver, &args, |matrix| {
                Value::Array(Rc::new(RefCell::new(matrix_points(matrix))))
            }),
            NativeMethodKind::MatrixInBounds => {
                require_arg_count("Matrix.in_bounds", &args, 1)?;
                let point = require_point(&args[0], "expected Point")?;
                if let Value::Matrix(matrix) = &method.receiver {
                    Ok(Value::Bool(matrix_in_bounds(&matrix.borrow(), point)))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::MatrixNeighbors4 => {
                require_arg_count("Matrix.neighbors4", &args, 1)?;
                let point = require_point(&args[0], "expected Point")?;
                if let Value::Matrix(matrix) = &method.receiver {
                    Ok(Value::Array(Rc::new(RefCell::new(matrix_neighbors4(
                        &matrix.borrow(),
                        point,
                    )))))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::MatrixNeighbors8 => {
                require_arg_count("Matrix.neighbors8", &args, 1)?;
                let point = require_point(&args[0], "expected Point")?;
                if let Value::Matrix(matrix) = &method.receiver {
                    Ok(Value::Array(Rc::new(RefCell::new(matrix_neighbors8(
                        &matrix.borrow(),
                        point,
                    )))))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::MatrixFind => {
                require_arg_count("Matrix.find", &args, 1)?;
                if let Value::Matrix(matrix) = &method.receiver {
                    let matrix = matrix.borrow();
                    for point_value in matrix_points(&matrix) {
                        let Value::Point(point) = point_value else {
                            unreachable!();
                        };
                        if value_eq(&matrix.rows[point.y as usize][point.x as usize], &args[0]) {
                            return Ok(Value::Point(point));
                        }
                    }
                    Ok(Value::Null)
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::MatrixFindAll => {
                require_arg_count("Matrix.find_all", &args, 1)?;
                if let Value::Matrix(matrix) = &method.receiver {
                    let matrix = matrix.borrow();
                    let mut points = Vec::new();
                    for point_value in matrix_points(&matrix) {
                        let Value::Point(point) = point_value else {
                            unreachable!();
                        };
                        if value_eq(&matrix.rows[point.y as usize][point.x as usize], &args[0]) {
                            points.push(Value::Point(point));
                        }
                    }
                    Ok(Value::Array(Rc::new(RefCell::new(points))))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::ResponseText => {
                require_arg_count("Response.text", &args, 0)?;
                if let Value::Response(response) = &method.receiver {
                    Ok(Value::String(response.body.clone()))
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::ResponseJson => {
                require_arg_count("Response.json", &args, 0)?;
                if let Value::Response(response) = &method.receiver {
                    let json: JsonValue = serde_json::from_str(&response.body)
                        .map_err(|err| runtime_error(format!("invalid JSON response: {err}")))?;
                    json_to_value(&json)
                } else {
                    unreachable!()
                }
            }
            NativeMethodKind::TaskDone => {
                require_arg_count("Task.done", &args, 0)?;
                Ok(Value::Bool(true))
            }
            NativeMethodKind::TaskCancel => {
                require_arg_count("Task.cancel", &args, 0)?;
                Ok(Value::Bool(false))
            }
        }
    }

    fn iter_len(&self, value: &Value) -> Result<usize, Diagnostic> {
        match value {
            Value::Array(values) => Ok(values.borrow().len()),
            Value::Map(values) => Ok(values.borrow().len()),
            Value::String(value) => Ok(value.chars().count()),
            Value::Matrix(matrix) => Ok(matrix_height(&matrix.borrow())),
            _ => Err(runtime_error(format!(
                "len expects collection, got {}",
                type_name(value)
            ))),
        }
    }

    fn path_search(&mut self, args: Vec<Value>, _astar: bool) -> Result<Value, Diagnostic> {
        require_arg_count("path search", &args, 4)?;
        let Value::Matrix(matrix) = &args[0] else {
            return Err(runtime_error("path functions expect Matrix, Point, Point"));
        };
        let start = require_point(&args[1], "path functions expect Matrix, Point, Point")?;
        let goal = require_point(&args[2], "path functions expect Matrix, Point, Point")?;
        let passable = args[3].clone();
        let mut queue = VecDeque::from([start]);
        let mut came_from: BTreeMap<(i64, i64), Option<Point>> = BTreeMap::new();
        came_from.insert((start.x, start.y), None);
        while let Some(current) = queue.pop_front() {
            if current == goal {
                let mut path = Vec::new();
                let mut cursor = Some(current);
                while let Some(point) = cursor {
                    path.push(Value::Point(point));
                    cursor = came_from.get(&(point.x, point.y)).copied().flatten();
                }
                path.reverse();
                return Ok(Value::Array(Rc::new(RefCell::new(path))));
            }
            let neighbors = matrix_neighbors4(&matrix.borrow(), current);
            for neighbor in neighbors {
                let Value::Point(next) = neighbor else {
                    unreachable!();
                };
                if came_from.contains_key(&(next.x, next.y)) {
                    continue;
                }
                let cell = matrix_get(&matrix.borrow(), &[Value::Point(next)], false)?;
                let ok = self.call_value(passable.clone(), vec![cell])?;
                if !self.require_bool(&ok, "path passable predicate")? {
                    continue;
                }
                came_from.insert((next.x, next.y), Some(current));
                queue.push_back(next);
            }
        }
        Ok(Value::Null)
    }

    fn store_save(&mut self, args: Vec<Value>) -> Result<Value, Diagnostic> {
        require_arg_count("store.save", &args, 2)?;
        let db = require_store(&args[0])?;
        let object = unwrap_object(&args[1])?;
        let db_key = db.borrow().path.to_string_lossy().to_string();
        let den_name = object.borrow().den_name.clone();
        let existing_id = object.borrow().store_ids.get(&db_key).copied();
        let mut db_mut = db.borrow_mut();
        let object_id = if let Some(id) = existing_id {
            id
        } else {
            db_mut.next_id(&den_name)
        };
        object.borrow_mut().store_ids.insert(db_key, object_id);
        let fields = self.serialize_object_fields(&object)?;
        let record = json!({
            "id": object_id,
            "den": den_name,
            "fields": fields,
        });
        db_mut
            .records_for_mut(&den_name)
            .and_then(|records| records.as_object_mut())
            .expect("records object")
            .insert(object_id.to_string(), record);
        db_mut.flush()?;
        Ok(Value::Int(object_id))
    }

    fn store_load(&mut self, args: Vec<Value>) -> Result<Value, Diagnostic> {
        require_arg_count("store.load", &args, 3)?;
        let db = require_store(&args[0])?;
        let den_name = require_den_type_name(&args[1])?;
        let Value::Int(id) = args[2] else {
            return Err(runtime_error("store id must be Int"));
        };
        let record = db
            .borrow()
            .records_for(&den_name)
            .and_then(|records| records.get(id.to_string()))
            .cloned();
        let Some(record) = record else {
            return Ok(Value::Null);
        };
        self.deserialize_object(&den_name, &record["fields"], Some((db, id)))
    }

    fn store_all(&mut self, args: Vec<Value>) -> Result<Value, Diagnostic> {
        require_arg_count("store.all", &args, 2)?;
        let db = require_store(&args[0])?;
        let den_name = require_den_type_name(&args[1])?;
        let records = db
            .borrow()
            .records_for(&den_name)
            .and_then(|records| records.as_object().cloned())
            .unwrap_or_default();
        let mut ids = records
            .keys()
            .filter_map(|key| key.parse::<i64>().ok())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        let mut values = Vec::new();
        for id in ids {
            if let Some(record) = records.get(&id.to_string()) {
                values.push(self.deserialize_object(
                    &den_name,
                    &record["fields"],
                    Some((db.clone(), id)),
                )?);
            }
        }
        Ok(Value::Array(Rc::new(RefCell::new(values))))
    }

    fn store_find(&mut self, args: Vec<Value>) -> Result<Value, Diagnostic> {
        require_arg_count("store.find", &args, 4)?;
        let db = require_store(&args[0])?;
        let den_name = require_den_type_name(&args[1])?;
        let Value::String(field_name) = &args[2] else {
            return Err(runtime_error("store.find field name must be String"));
        };
        let needle = self.serialize_value(&args[3])?;
        let records = db
            .borrow()
            .records_for(&den_name)
            .and_then(|records| records.as_object().cloned())
            .unwrap_or_default();
        let mut ids = records
            .keys()
            .filter_map(|key| key.parse::<i64>().ok())
            .collect::<Vec<_>>();
        ids.sort_unstable();
        let mut values = Vec::new();
        for id in ids {
            if let Some(record) = records.get(&id.to_string()) {
                if record["fields"].get(field_name) == Some(&needle) {
                    values.push(self.deserialize_object(
                        &den_name,
                        &record["fields"],
                        Some((db.clone(), id)),
                    )?);
                }
            }
        }
        Ok(Value::Array(Rc::new(RefCell::new(values))))
    }

    fn serialize_object_fields(&self, object: &ObjRef) -> Result<JsonValue, Diagnostic> {
        let den_name = object.borrow().den_name.clone();
        let mut fields = serde_json::Map::new();
        for field in self.field_order(&den_name)? {
            let value = object
                .borrow()
                .fields
                .get(&field.name)
                .cloned()
                .flatten()
                .ok_or_else(|| {
                    runtime_error(format!("{den_name}.{} is not initialized", field.name))
                })?;
            fields.insert(field.name, self.serialize_value(&value)?);
        }
        Ok(JsonValue::Object(fields))
    }

    fn serialize_value(&self, value: &Value) -> Result<JsonValue, Diagnostic> {
        match value {
            Value::Null => Ok(json!({"kind": "Null"})),
            Value::Bool(value) => Ok(json!({"kind": "Bool", "value": value})),
            Value::Int(value) => Ok(json!({"kind": "Int", "value": value})),
            Value::Float(value) => Ok(json!({"kind": "Float", "value": value})),
            Value::String(value) => Ok(json!({"kind": "String", "value": value})),
            Value::Array(values) => Ok(json!({
                "kind": "Array",
                "items": values.borrow().iter().map(|item| self.serialize_value(item)).collect::<Result<Vec<_>, _>>()?,
            })),
            Value::Point(point) => Ok(json!({"kind": "Point", "x": point.x, "y": point.y})),
            Value::Matrix(matrix) => Ok(json!({
                "kind": "Matrix",
                "rows": matrix.borrow().rows.iter().map(|row| {
                    row.iter().map(|item| self.serialize_value(item)).collect::<Result<Vec<_>, _>>()
                }).collect::<Result<Vec<_>, _>>()?,
            })),
            Value::Object(object) => Ok(json!({
                "kind": "Object",
                "den": object.borrow().den_name,
                "fields": self.serialize_object_fields(object)?,
            })),
            Value::ObjectView { object, .. } => {
                self.serialize_value(&Value::Object(object.clone()))
            }
            other => Err(runtime_error(format!(
                "store cannot serialize {}",
                type_name(other)
            ))),
        }
    }

    fn deserialize_object(
        &self,
        den_name: &str,
        fields: &JsonValue,
        db_info: Option<(StoreRef, i64)>,
    ) -> Result<Value, Diagnostic> {
        if !self.dens.contains_key(den_name) {
            return Err(runtime_error(format!(
                "stored den {den_name} is not defined"
            )));
        }
        let object = Rc::new(RefCell::new(ObjectInstance {
            den_name: den_name.to_string(),
            fields: BTreeMap::new(),
            store_ids: BTreeMap::new(),
        }));
        for field in self.field_order(den_name)? {
            let Some(encoded) = fields.get(&field.name) else {
                return Err(runtime_error(format!(
                    "stored {den_name}.{} is missing",
                    field.name
                )));
            };
            let value = self.deserialize_value(encoded)?;
            if let Some(type_name) = &field.type_name {
                self.check_type(
                    &value,
                    type_name,
                    &format!("{}.{}", field.owner, field.name),
                )?;
            }
            object.borrow_mut().fields.insert(field.name, Some(value));
        }
        if let Some((db, id)) = db_info {
            object
                .borrow_mut()
                .store_ids
                .insert(db.borrow().path.to_string_lossy().to_string(), id);
        }
        Ok(Value::Object(object))
    }

    fn deserialize_value(&self, encoded: &JsonValue) -> Result<Value, Diagnostic> {
        match encoded.get("kind").and_then(JsonValue::as_str) {
            Some("Null") => Ok(Value::Null),
            Some("Bool") => Ok(Value::Bool(encoded["value"].as_bool().unwrap_or(false))),
            Some("Int") => Ok(Value::Int(encoded["value"].as_i64().unwrap_or_default())),
            Some("Float") => Ok(Value::Float(encoded["value"].as_f64().unwrap_or_default())),
            Some("String") => Ok(Value::String(
                encoded["value"].as_str().unwrap_or_default().to_string(),
            )),
            Some("Array") => Ok(Value::Array(Rc::new(RefCell::new(
                encoded["items"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|item| self.deserialize_value(item))
                    .collect::<Result<Vec<_>, _>>()?,
            )))),
            Some("Point") => Ok(Value::Point(Point {
                x: encoded["x"].as_i64().unwrap_or_default(),
                y: encoded["y"].as_i64().unwrap_or_default(),
            })),
            Some("Matrix") => {
                let rows = encoded["rows"]
                    .as_array()
                    .unwrap_or(&Vec::new())
                    .iter()
                    .map(|row| {
                        row.as_array()
                            .unwrap_or(&Vec::new())
                            .iter()
                            .map(|item| self.deserialize_value(item))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Value::Matrix(Rc::new(RefCell::new(MatrixData { rows }))))
            }
            Some("Object") => {
                let den_name = encoded["den"].as_str().unwrap_or_default();
                self.deserialize_object(den_name, &encoded["fields"], None)
            }
            Some(kind) => Err(runtime_error(format!("unknown stored value kind {kind}"))),
            None => Err(runtime_error("stored value missing kind")),
        }
    }

    fn web_grab(&self, options: &Value) -> Result<Value, Diagnostic> {
        let url = match options {
            Value::String(url) => url.clone(),
            Value::Map(map) => {
                let map = map.borrow();
                match map.get("url") {
                    Some(Value::String(url)) => url.clone(),
                    _ => return Err(runtime_error("web request url must be String")),
                }
            }
            _ => {
                return Err(runtime_error(
                    "web request expects String URL or Map options",
                ))
            }
        };
        if let Some(payload) = url.strip_prefix("data:") {
            let (_, body) = payload.split_once(',').unwrap_or(("", payload));
            let body = percent_decode(body)?;
            let headers = BTreeMap::new();
            return Ok(Value::Response(Rc::new(Response {
                status: 200,
                headers,
                body,
                url,
                ok: true,
            })));
        }
        Err(Diagnostic::new(
            Category::Network,
            "network request failed: only data: URLs are supported natively",
        ))
    }

    fn check_type(
        &self,
        value: &Value,
        type_name_hint: &str,
        label: &str,
    ) -> Result<(), Diagnostic> {
        let type_ref = parse_type_ref(type_name_hint)?;
        self.check_type_ref(value, &type_ref, label)
    }

    fn check_type_ref(
        &self,
        value: &Value,
        type_ref: &TypeRef,
        label: &str,
    ) -> Result<(), Diagnostic> {
        let value = match value {
            Value::ObjectView { object, .. } => Value::Object(object.clone()),
            other => other.clone(),
        };
        let ok = match type_ref.name.as_str() {
            "Any" | "T" => true,
            "Void" | "Null" => matches!(value, Value::Null),
            "Int" => matches!(value, Value::Int(_)),
            "Float" => matches!(value, Value::Int(_) | Value::Float(_)),
            "Bool" => matches!(value, Value::Bool(_)),
            "String" => matches!(value, Value::String(_)),
            "Array" => matches!(value, Value::Array(_)),
            "Map" => matches!(value, Value::Map(_)),
            "Matrix" => matches!(value, Value::Matrix(_)),
            "Point" => matches!(value, Value::Point(_)),
            "Response" => matches!(value, Value::Response(_)),
            "Task" => matches!(value, Value::Task(_)),
            "TaskGroup" => matches!(value, Value::TaskGroup(_)),
            name if self.dens.contains_key(name) => match &value {
                Value::Null => true,
                Value::Object(object) => self.den_is_a(&object.borrow().den_name, name),
                Value::DenType(den) => den == name,
                _ => false,
            },
            name if self.masks.contains_key(name) => match &value {
                Value::Null => true,
                Value::Object(object) => self.den_wears(&object.borrow().den_name, name),
                Value::ObjectView { mask, .. } => mask == name,
                Value::MaskType(mask) => mask == name,
                _ => false,
            },
            _ => {
                return Err(runtime_error(format!(
                    "unknown type annotation {}",
                    type_ref.text()
                )))
            }
        };
        if !ok {
            return Err(runtime_error(format!(
                "{label} must be {}, got {}",
                type_ref.text(),
                type_name(&value)
            )));
        }
        match (&value, type_ref.name.as_str(), type_ref.args.as_slice()) {
            (Value::Array(values), "Array", [item_type]) => {
                for (index, item) in values.borrow().iter().enumerate() {
                    self.check_type_ref(item, item_type, &format!("{label}[{index}]"))?;
                }
            }
            (Value::Matrix(matrix), "Matrix", [item_type]) => {
                for (y, row) in matrix.borrow().rows.iter().enumerate() {
                    for (x, item) in row.iter().enumerate() {
                        self.check_type_ref(item, item_type, &format!("{label}[{y}, {x}]"))?;
                    }
                }
            }
            (Value::Map(map), "Map", [item_type]) => {
                for (key, item) in map.borrow().iter() {
                    self.check_type_ref(item, item_type, &format!("{label}[{key}]"))?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn den_is_a(&self, den_name: &str, expected: &str) -> bool {
        if den_name == expected {
            return true;
        }
        self.dens
            .get(den_name)
            .and_then(|den| den.parent.as_ref())
            .is_some_and(|parent| self.den_is_a(parent, expected))
    }

    fn den_wears(&self, den_name: &str, mask: &str) -> bool {
        let Some(den) = self.dens.get(den_name) else {
            return false;
        };
        den.masks.iter().any(|candidate| candidate == mask)
            || den
                .parent
                .as_ref()
                .is_some_and(|parent| self.den_wears(parent, mask))
    }

    fn static_check(&self, program: &Program) -> Result<(), Diagnostic> {
        let mut env = StaticEnv::default();
        for item in &program.items {
            match item {
                Item::Function(def) | Item::HowlFunction(def) => {
                    env.values.insert(
                        def.name.clone(),
                        StaticBinding {
                            type_name: def.return_type.clone(),
                            is_const: true,
                        },
                    );
                }
                Item::Den(den) => {
                    env.values.insert(
                        den.name.clone(),
                        StaticBinding {
                            type_name: Some(den.name.clone()),
                            is_const: true,
                        },
                    );
                }
                Item::Mask(mask) => {
                    env.values.insert(
                        mask.name.clone(),
                        StaticBinding {
                            type_name: Some(mask.name.clone()),
                            is_const: true,
                        },
                    );
                }
                _ => {}
            }
        }
        for item in &program.items {
            match item {
                Item::Main { body, .. } => {
                    self.static_check_block(body, &mut env.child(), false, None, None)?
                }
                Item::HowlMain { body, .. } => {
                    self.static_check_block(body, &mut env.child(), true, None, None)?
                }
                Item::Function(def) => {
                    let mut fn_env = env.child();
                    for param in &def.params {
                        fn_env.values.insert(
                            param.name.clone(),
                            StaticBinding {
                                type_name: param.type_name.clone(),
                                is_const: false,
                            },
                        );
                    }
                    self.static_check_block(
                        &def.body,
                        &mut fn_env,
                        false,
                        def.return_type.clone(),
                        None,
                    )?;
                }
                Item::HowlFunction(def) => {
                    let mut fn_env = env.child();
                    for param in &def.params {
                        fn_env.values.insert(
                            param.name.clone(),
                            StaticBinding {
                                type_name: param.type_name.clone(),
                                is_const: false,
                            },
                        );
                    }
                    self.static_check_block(
                        &def.body,
                        &mut fn_env,
                        true,
                        def.return_type.clone(),
                        None,
                    )?;
                }
                Item::Den(den) => {
                    for member in &den.members {
                        if let DenMember::Method(method) = member {
                            let mut method_env = env.child();
                            method_env.values.insert(
                                "self".to_string(),
                                StaticBinding {
                                    type_name: Some(den.name.clone()),
                                    is_const: true,
                                },
                            );
                            for param in &method.params {
                                method_env.values.insert(
                                    param.name.clone(),
                                    StaticBinding {
                                        type_name: param.type_name.clone(),
                                        is_const: false,
                                    },
                                );
                            }
                            self.static_check_block(
                                &method.body,
                                &mut method_env,
                                false,
                                method.return_type.clone(),
                                Some(den.name.clone()),
                            )?;
                        }
                    }
                }
                Item::Probe { body, .. } | Item::Stmt(Stmt::InsaneBlock(body)) => {
                    self.static_check_block(body, &mut env.child(), false, None, None)?;
                }
                Item::Stmt(stmt) => self.static_check_stmt(stmt, &mut env, false, None, None)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn static_check_block(
        &self,
        body: &[Stmt],
        env: &mut StaticEnv,
        howl: bool,
        return_type: Option<String>,
        current_den: Option<String>,
    ) -> Result<(), Diagnostic> {
        for stmt in body {
            self.static_check_stmt(stmt, env, howl, return_type.clone(), current_den.clone())?;
        }
        Ok(())
    }

    fn static_check_stmt(
        &self,
        stmt: &Stmt,
        env: &mut StaticEnv,
        howl: bool,
        return_type: Option<String>,
        current_den: Option<String>,
    ) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Let {
                name,
                expr,
                type_name,
                is_const,
            } => {
                self.static_check_expr(expr, env, howl, current_den.clone())?;
                if let Some(type_name) = type_name {
                    if let Some(expr_type) = self.static_expr_type(expr, env) {
                        self.static_require_assignable(&expr_type, type_name, name)?;
                    }
                }
                env.values.insert(
                    name.clone(),
                    StaticBinding {
                        type_name: type_name
                            .clone()
                            .or_else(|| self.static_expr_type(expr, env)),
                        is_const: *is_const,
                    },
                );
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.static_check_expr(condition, env, howl, current_den.clone())?;
                if let Some(kind) = self.static_expr_type(condition, env) {
                    if kind != "Bool" {
                        return Err(runtime_error("if condition must be Bool"));
                    }
                }
                self.static_check_block(
                    then_body,
                    &mut env.child(),
                    howl,
                    return_type.clone(),
                    current_den.clone(),
                )?;
                if let Some(else_body) = else_body {
                    match else_body {
                        ElseBody::If(stmt) => {
                            self.static_check_stmt(stmt, env, howl, return_type, current_den)?
                        }
                        ElseBody::Block(body) => self.static_check_block(
                            body,
                            &mut env.child(),
                            howl,
                            return_type,
                            current_den,
                        )?,
                    }
                }
            }
            Stmt::While { condition, body } => {
                self.static_check_expr(condition, env, howl, current_den.clone())?;
                if let Some(kind) = self.static_expr_type(condition, env) {
                    if kind != "Bool" {
                        return Err(runtime_error("while condition must be Bool"));
                    }
                }
                self.static_check_block(body, &mut env.child(), howl, return_type, current_den)?;
            }
            Stmt::For {
                name,
                iterable,
                body,
                ..
            } => {
                self.static_check_expr(iterable, env, howl, current_den.clone())?;
                let mut loop_env = env.child();
                loop_env.values.insert(
                    name.clone(),
                    StaticBinding {
                        type_name: None,
                        is_const: false,
                    },
                );
                self.static_check_block(body, &mut loop_env, howl, return_type, current_den)?;
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.static_check_expr(expr, env, howl, current_den.clone())?;
                    if let (Some(expected), Some(actual)) =
                        (return_type.as_ref(), self.static_expr_type(expr, env))
                    {
                        self.static_require_assignable(&actual, expected, "return value")?;
                    }
                } else if return_type.as_deref().is_some_and(|kind| kind != "Void") {
                    return Err(runtime_error(format!(
                        "return value must be {}",
                        return_type.unwrap()
                    )));
                }
            }
            Stmt::Expr(expr) | Stmt::Panic(expr) | Stmt::Expect(expr) => {
                self.static_check_expr(expr, env, howl, current_den)?
            }
            Stmt::Squeak(exprs) | Stmt::Trace(exprs) => {
                for expr in exprs {
                    self.static_check_expr(expr, env, howl, current_den.clone())?;
                }
            }
            Stmt::Try {
                body,
                catch_name,
                catch_body,
                ..
            } => {
                self.static_check_block(
                    body,
                    &mut env.child(),
                    howl,
                    return_type.clone(),
                    current_den.clone(),
                )?;
                if let Some(catch_body) = catch_body {
                    let mut catch_env = env.child();
                    if let Some(catch_name) = catch_name {
                        catch_env.values.insert(
                            catch_name.clone(),
                            StaticBinding {
                                type_name: Some("String".to_string()),
                                is_const: false,
                            },
                        );
                    }
                    self.static_check_block(
                        catch_body,
                        &mut catch_env,
                        howl,
                        return_type,
                        current_den,
                    )?;
                }
            }
            Stmt::InsaneBlock(body) => {
                self.static_check_block(body, &mut env.child(), howl, return_type, current_den)?
            }
            Stmt::Break | Stmt::Continue => {}
        }
        Ok(())
    }

    fn static_check_expr(
        &self,
        expr: &Expr,
        env: &StaticEnv,
        howl: bool,
        current_den: Option<String>,
    ) -> Result<(), Diagnostic> {
        match expr {
            Expr::Wait(inner) => {
                if !howl {
                    return Err(runtime_error("wait can only be used inside howl context"));
                }
                self.static_check_expr(inner, env, howl, current_den)
            }
            Expr::Scatter { expr, .. } => {
                if !howl {
                    return Err(runtime_error(
                        "scatter can only be used inside howl context",
                    ));
                }
                self.static_check_expr(expr, env, howl, current_den)
            }
            Expr::Nest(items) => {
                if !howl {
                    return Err(runtime_error("nest can only be used inside howl context"));
                }
                for item in items {
                    self.static_check_expr(item, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Assign { target, value } => {
                if let Expr::Var(name) = target.as_ref() {
                    if env.get(name).is_some_and(|binding| binding.is_const) {
                        return Err(runtime_error(format!("{name} is a stash constant")));
                    }
                }
                self.static_check_expr(target, env, howl, current_den.clone())?;
                self.static_check_expr(value, env, howl, current_den)
            }
            Expr::Binary { left, right, .. } => {
                self.static_check_expr(left, env, howl, current_den.clone())?;
                self.static_check_expr(right, env, howl, current_den)
            }
            Expr::Unary { expr, .. } | Expr::InsaneChoose(expr) => {
                self.static_check_expr(expr, env, howl, current_den)
            }
            Expr::Array(items) | Expr::Matrix(items) => {
                for item in items {
                    self.static_check_expr(item, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Map(pairs) => {
                for (key, value) in pairs {
                    self.static_check_expr(key, env, howl, current_den.clone())?;
                    self.static_check_expr(value, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Point { x, y } | Expr::Range { start: x, end: y } => {
                self.static_check_expr(x, env, howl, current_den.clone())?;
                self.static_check_expr(y, env, howl, current_den)
            }
            Expr::Call { callee, args } => {
                self.static_check_expr(callee, env, howl, current_den.clone())?;
                for arg in args {
                    self.static_check_expr(arg, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Index { target, args } => {
                self.static_check_expr(target, env, howl, current_den.clone())?;
                for arg in args {
                    self.static_check_expr(arg, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Member { target, name } => {
                self.static_check_expr(target, env, howl, current_den.clone())?;
                if let Some(target_type) = self.static_expr_type(target, env) {
                    if let Some(mask) = self.masks.get(&target_type) {
                        if !mask.methods.contains_key(name) {
                            return Err(runtime_error(format!(
                                "mask {target_type} has no member {name}"
                            )));
                        }
                    }
                    if self.dens.contains_key(&target_type) {
                        if let Some(field) = self.find_field(&target_type, name) {
                            if field.access == Access::Fang
                                && current_den.as_deref() != Some(field.owner.as_str())
                            {
                                return Err(runtime_error(format!(
                                    "{target_type}.{name} is private"
                                )));
                            }
                        }
                    }
                }
                Ok(())
            }
            Expr::Lambda { body, .. } => match body {
                LambdaBody::Expr(expr) => self.static_check_expr(expr, env, howl, current_den),
                LambdaBody::Block(body) => {
                    self.static_check_block(body, &mut env.child(), howl, None, current_den)
                }
            },
            Expr::Tunnel { left, right } => {
                self.static_check_expr(left, env, howl, current_den.clone())?;
                self.static_check_expr(right, env, howl, current_den)
            }
            Expr::Hatch { args, .. } => {
                for arg in args {
                    self.static_check_expr(arg, env, howl, current_den.clone())?;
                }
                Ok(())
            }
            Expr::Literal(_) | Expr::Var(_) | Expr::Sniff => Ok(()),
        }
    }

    fn static_expr_type(&self, expr: &Expr, env: &StaticEnv) -> Option<String> {
        match expr {
            Expr::Literal(Literal::Null) => Some("Null".to_string()),
            Expr::Literal(Literal::Bool(_)) => Some("Bool".to_string()),
            Expr::Literal(Literal::Int(_)) => Some("Int".to_string()),
            Expr::Literal(Literal::Float(_)) => Some("Float".to_string()),
            Expr::Literal(Literal::String(_)) => Some("String".to_string()),
            Expr::Array(items) => {
                if let Some(first) = items
                    .first()
                    .and_then(|item| self.static_expr_type(item, env))
                {
                    Some(format!("Array<{first}>"))
                } else {
                    Some("Array".to_string())
                }
            }
            Expr::Matrix(rows) => {
                if let Some(Expr::Array(items)) = rows.first() {
                    if let Some(first) = items
                        .first()
                        .and_then(|item| self.static_expr_type(item, env))
                    {
                        return Some(format!("Matrix<{first}>"));
                    }
                }
                Some("Matrix".to_string())
            }
            Expr::Point { .. } => Some("Point".to_string()),
            Expr::Hatch { name, .. } => Some(name.clone()),
            Expr::Var(name) => env.get(name).and_then(|binding| binding.type_name.clone()),
            Expr::Binary { op, .. }
                if ["==", "!=", "<", "<=", ">", ">=", "&&", "||"].contains(&op.as_str()) =>
            {
                Some("Bool".to_string())
            }
            Expr::Binary { left, op, right } if op == "+" => {
                let left = self.static_expr_type(left, env);
                let right = self.static_expr_type(right, env);
                if left.as_deref() == Some("String") || right.as_deref() == Some("String") {
                    Some("String".to_string())
                } else {
                    left.or(right)
                }
            }
            Expr::Member { target, name } => {
                let target_type = self.static_expr_type(target, env)?;
                self.find_field(&target_type, name)
                    .and_then(|field| field.type_name)
            }
            _ => None,
        }
    }

    fn static_require_assignable(
        &self,
        actual: &str,
        expected: &str,
        label: &str,
    ) -> Result<(), Diagnostic> {
        let expected_ref = parse_type_ref(expected)?;
        let actual_base = parse_type_ref(actual)?.name;
        let ok = expected_ref.name == "Any"
            || actual_base == "Null"
            || actual_base == expected_ref.name
            || (expected_ref.name == "Float" && actual_base == "Int")
            || (self.dens.contains_key(&expected_ref.name)
                && self.den_is_a(&actual_base, &expected_ref.name))
            || (self.masks.contains_key(&expected_ref.name)
                && self.den_wears(&actual_base, &expected_ref.name));
        if !ok {
            Err(runtime_error(format!(
                "{label} must be {}",
                expected_ref.text()
            )))
        } else {
            Ok(())
        }
    }

    fn load_namespace(&mut self, name: &str) -> Result<Value, Diagnostic> {
        match name {
            "math" => return Ok(self.math_namespace()),
            "path" => return Ok(self.path_namespace()),
            "chaser" => return Ok(self.chaser_namespace()),
            "store" => return Ok(self.store_namespace()),
            "web" => return Ok(self.web_namespace()),
            "tick" => return Ok(self.tick_namespace()),
            _ => {}
        }

        let Some(source_path) = &self.source_path else {
            return Err(runtime_error(format!("cannot resolve module {name}")));
        };
        let module_path = source_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("{name}.imm"));

        if let Some(source) = self
            .embedded_sources
            .get(&normalize_embedded_key(&module_path))
        {
            return self.load_module_from_source(name, &module_path, source.clone());
        }

        if !module_path.exists() {
            return Err(runtime_error(format!(
                "module {name} not found at {}",
                module_path.display()
            )));
        }
        let module_path = module_path.canonicalize().map_err(io_error)?;
        if let Some(value) = self.module_cache.borrow().get(&module_path).cloned() {
            return Ok(value);
        }
        if self.module_stack.contains(&module_path) {
            let cycle = self
                .module_stack
                .iter()
                .chain(std::iter::once(&module_path))
                .map(|path| {
                    path.file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(runtime_error(format!("cyclic module import: {cycle}")));
        }
        let source = fs::read_to_string(&module_path).map_err(io_error)?;
        self.load_module_from_source(name, &module_path, source)
    }

    fn load_module_from_source(
        &mut self,
        name: &str,
        module_path: &Path,
        source: String,
    ) -> Result<Value, Diagnostic> {
        let program = parse_source(0, &source)?;
        let mut runtime = Runtime::new(Some(module_path.to_path_buf()));
        runtime.output = self.output.clone();
        runtime.trace = self.trace.clone();
        runtime.trace_enabled = self.trace_enabled;
        runtime.module_cache = self.module_cache.clone();
        runtime.module_stack = {
            let mut stack = self.module_stack.clone();
            stack.push(module_path.to_path_buf());
            stack
        };
        runtime.embedded_sources = self.embedded_sources.clone();
        runtime.run(&program, false)?;
        let hidden = core_names();
        let values = runtime
            .env
            .borrow()
            .values
            .iter()
            .filter(|(key, _)| !hidden.contains(&key.as_str()))
            .map(|(key, cell)| (key.clone(), cell.value.clone()))
            .collect::<BTreeMap<_, _>>();
        let namespace = Value::Namespace(Rc::new(Namespace {
            name: name.to_string(),
            values,
        }));
        self.module_cache
            .borrow_mut()
            .insert(module_path.to_path_buf(), namespace.clone());
        Ok(namespace)
    }

    fn install_core(&mut self) {
        for (name, kind) in [
            ("len", BuiltinKind::Len),
            ("type", BuiltinKind::Type),
            ("str", BuiltinKind::Str),
            ("int", BuiltinKind::Int),
            ("float", BuiltinKind::Float),
            ("bool", BuiltinKind::Bool),
            ("map", BuiltinKind::Map),
            ("filter", BuiltinKind::Filter),
            ("reduce", BuiltinKind::Reduce),
            ("nap", BuiltinKind::Nap),
        ] {
            self.env
                .borrow_mut()
                .define_unchecked(name, Value::Builtin(kind), true, None);
        }
        for (name, namespace) in [
            ("math", self.math_namespace()),
            ("path", self.path_namespace()),
            ("chaser", self.chaser_namespace()),
            ("store", self.store_namespace()),
            ("web", self.web_namespace()),
            ("tick", self.tick_namespace()),
        ] {
            self.env
                .borrow_mut()
                .define_unchecked(name, namespace, true, None);
        }
    }

    fn namespace(&self, name: &str, values: &[(&str, BuiltinKind)]) -> Value {
        Value::Namespace(Rc::new(Namespace {
            name: name.to_string(),
            values: values
                .iter()
                .map(|(name, kind)| (name.to_string(), Value::Builtin(*kind)))
                .collect(),
        }))
    }

    fn math_namespace(&self) -> Value {
        self.namespace(
            "math",
            &[
                ("abs", BuiltinKind::MathAbs),
                ("min", BuiltinKind::MathMin),
                ("max", BuiltinKind::MathMax),
                ("sqrt", BuiltinKind::MathSqrt),
                ("floor", BuiltinKind::MathFloor),
                ("ceil", BuiltinKind::MathCeil),
                ("random", BuiltinKind::MathRandom),
            ],
        )
    }

    fn path_namespace(&self) -> Value {
        self.namespace(
            "path",
            &[
                ("bfs", BuiltinKind::PathBfs),
                ("astar", BuiltinKind::PathAstar),
            ],
        )
    }

    fn chaser_namespace(&self) -> Value {
        self.namespace(
            "chaser",
            &[
                ("direction", BuiltinKind::ChaserDirection),
                ("step", BuiltinKind::ChaserStep),
                ("parse_field", BuiltinKind::ChaserParseField),
                ("safe_moves", BuiltinKind::ChaserSafeMoves),
                ("random_move", BuiltinKind::ChaserRandomMove),
            ],
        )
    }

    fn store_namespace(&self) -> Value {
        self.namespace(
            "store",
            &[
                ("open", BuiltinKind::StoreOpen),
                ("save", BuiltinKind::StoreSave),
                ("load", BuiltinKind::StoreLoad),
                ("all", BuiltinKind::StoreAll),
                ("find", BuiltinKind::StoreFind),
                ("get", BuiltinKind::StoreGet),
                ("delete", BuiltinKind::StoreDelete),
                ("count", BuiltinKind::StoreCount),
                ("clear", BuiltinKind::StoreClear),
            ],
        )
    }

    fn web_namespace(&self) -> Value {
        self.namespace(
            "web",
            &[
                ("grab", BuiltinKind::WebGrab),
                ("fetch", BuiltinKind::WebFetch),
            ],
        )
    }

    fn tick_namespace(&self) -> Value {
        self.namespace("tick", &[("now", BuiltinKind::TickNow)])
    }
}

#[derive(Clone)]
enum PreparedMain {
    Main { body: Vec<Stmt>, insane: bool },
    HowlMain { body: Vec<Stmt>, insane: bool },
}

fn should_skip_top_level_execute(item: &Item) -> bool {
    matches!(
        item,
        Item::Function(_)
            | Item::HowlFunction(_)
            | Item::Main { .. }
            | Item::HowlMain { .. }
            | Item::Use(_)
            | Item::Module(_)
            | Item::Den(_)
            | Item::Mask(_)
            | Item::Probe { .. }
            | Item::Pack(_)
    )
}

fn find_env(env: &EnvRef, name: &str) -> Option<EnvRef> {
    if env.borrow().values.contains_key(name) {
        return Some(env.clone());
    }
    env.borrow()
        .parent
        .as_ref()
        .and_then(|parent| find_env(parent, name))
}

fn find_cell(env: &EnvRef, name: &str) -> Option<Cell> {
    find_env(env, name).and_then(|found| found.borrow().values.get(name).cloned())
}

fn runtime_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(Category::Runtime, message.into())
}

fn io_error(err: std::io::Error) -> Diagnostic {
    Diagnostic::new(Category::Io, err.to_string())
}

fn numeric_binary(
    left: &Value,
    right: &Value,
    int_op: fn(i64, i64) -> i64,
    float_op: fn(f64, f64) -> f64,
) -> Result<Value, Diagnostic> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float(float_op(*a as f64, *b))),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(float_op(*a, *b as f64))),
        _ => Err(runtime_error("numeric operands required")),
    }
}

fn compare_values(op: &str, left: &Value, right: &Value) -> Result<Value, Diagnostic> {
    let result = match (left, right) {
        (Value::Int(a), Value::Int(b)) => compare_ord(op, *a as f64, *b as f64),
        (Value::Float(a), Value::Float(b)) => compare_ord(op, *a, *b),
        (Value::Int(a), Value::Float(b)) => compare_ord(op, *a as f64, *b),
        (Value::Float(a), Value::Int(b)) => compare_ord(op, *a, *b as f64),
        (Value::String(a), Value::String(b)) => match op {
            "<" => a < b,
            "<=" => a <= b,
            ">" => a > b,
            ">=" => a >= b,
            _ => unreachable!(),
        },
        _ => return Err(runtime_error("unsupported comparison operands")),
    };
    Ok(Value::Bool(result))
}

fn compare_ord(op: &str, a: f64, b: f64) -> bool {
    match op {
        "<" => a < b,
        "<=" => a <= b,
        ">" => a > b,
        ">=" => a >= b,
        _ => unreachable!(),
    }
}

fn value_eq(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Int(a), Value::Int(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Int(a), Value::Float(b)) => (*a as f64) == *b,
        (Value::Float(a), Value::Int(b)) => *a == (*b as f64),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Point(a), Value::Point(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => {
            let a = a.borrow();
            let b = b.borrow();
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| value_eq(a, b))
        }
        (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
        _ => false,
    }
}

fn one_int_arg(args: &[Value], message: &str) -> Result<i64, Diagnostic> {
    if args.len() != 1 {
        return Err(runtime_error(message));
    }
    if let Value::Int(value) = args[0] {
        Ok(value)
    } else {
        Err(runtime_error(message))
    }
}

fn two_int_args(args: &[Value], message: &str) -> Result<(i64, i64), Diagnostic> {
    if args.len() != 2 {
        return Err(runtime_error(message));
    }
    let (Value::Int(a), Value::Int(b)) = (&args[0], &args[1]) else {
        return Err(runtime_error(message));
    };
    Ok((*a, *b))
}

fn matrix_get(matrix: &MatrixData, args: &[Value], unsafe_mode: bool) -> Result<Value, Diagnostic> {
    let (y, x) = matrix_coords(args)?;
    if y < 0 || x < 0 || y as usize >= matrix.rows.len() || x as usize >= matrix_width(matrix) {
        if unsafe_mode {
            return Ok(Value::Null);
        }
        return Err(runtime_error(format!(
            "matrix index out of bounds: [{y}, {x}]"
        )));
    }
    Ok(matrix.rows[y as usize][x as usize].clone())
}

fn matrix_set(
    matrix: &mut MatrixData,
    args: &[Value],
    value: Value,
    unsafe_mode: bool,
) -> Result<(), Diagnostic> {
    let (y, x) = matrix_coords(args)?;
    if y < 0 || x < 0 || y as usize >= matrix.rows.len() || x as usize >= matrix_width(matrix) {
        if unsafe_mode {
            return Ok(());
        }
        return Err(runtime_error(format!(
            "matrix index out of bounds: [{y}, {x}]"
        )));
    }
    matrix.rows[y as usize][x as usize] = value;
    Ok(())
}

fn matrix_coords(args: &[Value]) -> Result<(i64, i64), Diagnostic> {
    if args.len() == 1 {
        if let Value::Point(point) = args[0] {
            return Ok((point.y, point.x));
        }
    }
    if args.len() == 2 {
        let (y, x) = two_int_args(args, "matrix index must be [y, x] or [point]")?;
        return Ok((y, x));
    }
    Err(runtime_error("matrix index must be [y, x] or [point]"))
}

fn matrix_width(matrix: &MatrixData) -> usize {
    matrix.rows.first().map_or(0, Vec::len)
}

fn matrix_height(matrix: &MatrixData) -> usize {
    matrix.rows.len()
}

fn matrix_points(matrix: &MatrixData) -> Vec<Value> {
    let mut points = Vec::new();
    for y in 0..matrix_height(matrix) {
        for x in 0..matrix_width(matrix) {
            points.push(Value::Point(Point {
                x: x as i64,
                y: y as i64,
            }));
        }
    }
    points
}

fn matrix_in_bounds(matrix: &MatrixData, point: Point) -> bool {
    point.x >= 0
        && point.y >= 0
        && (point.x as usize) < matrix_width(matrix)
        && (point.y as usize) < matrix_height(matrix)
}

fn matrix_neighbors4(matrix: &MatrixData, point: Point) -> Vec<Value> {
    [
        Point {
            x: point.x,
            y: point.y - 1,
        },
        Point {
            x: point.x + 1,
            y: point.y,
        },
        Point {
            x: point.x,
            y: point.y + 1,
        },
        Point {
            x: point.x - 1,
            y: point.y,
        },
    ]
    .into_iter()
    .filter(|point| matrix_in_bounds(matrix, *point))
    .map(Value::Point)
    .collect()
}

fn matrix_neighbors8(matrix: &MatrixData, point: Point) -> Vec<Value> {
    [
        Point {
            x: point.x,
            y: point.y - 1,
        },
        Point {
            x: point.x + 1,
            y: point.y - 1,
        },
        Point {
            x: point.x + 1,
            y: point.y,
        },
        Point {
            x: point.x + 1,
            y: point.y + 1,
        },
        Point {
            x: point.x,
            y: point.y + 1,
        },
        Point {
            x: point.x - 1,
            y: point.y + 1,
        },
        Point {
            x: point.x - 1,
            y: point.y,
        },
        Point {
            x: point.x - 1,
            y: point.y - 1,
        },
    ]
    .into_iter()
    .filter(|point| matrix_in_bounds(matrix, *point))
    .map(Value::Point)
    .collect()
}

fn matrix_method(target: &Value, name: &str) -> Result<Value, Diagnostic> {
    let kind = match name {
        "width" => NativeMethodKind::MatrixWidth,
        "height" => NativeMethodKind::MatrixHeight,
        "in_bounds" => NativeMethodKind::MatrixInBounds,
        "points" => NativeMethodKind::MatrixPoints,
        "neighbors4" => NativeMethodKind::MatrixNeighbors4,
        "neighbors8" => NativeMethodKind::MatrixNeighbors8,
        "find" => NativeMethodKind::MatrixFind,
        "find_all" => NativeMethodKind::MatrixFindAll,
        _ => return Err(runtime_error(format!("Matrix has no member {name}"))),
    };
    Ok(Value::NativeMethod(Rc::new(NativeMethod {
        receiver: target.clone(),
        kind,
    })))
}

fn string_method(target: &Value, name: &str) -> Result<Value, Diagnostic> {
    let kind = match name {
        "len" => NativeMethodKind::StringLen,
        "to_int" => NativeMethodKind::StringToInt,
        "to_float" => NativeMethodKind::StringToFloat,
        "to_bool" => NativeMethodKind::StringToBool,
        _ => return Err(runtime_error(format!("String has no member {name}"))),
    };
    Ok(Value::NativeMethod(Rc::new(NativeMethod {
        receiver: target.clone(),
        kind,
    })))
}

fn same_signature_method_mask(method: &MethodSpec, required: &MaskMethod) -> bool {
    method.params.len() == required.params.len()
        && method
            .params
            .iter()
            .zip(required.params.iter())
            .all(|(left, right)| {
                normalize_type_name(left.type_name.as_deref())
                    == normalize_type_name(right.type_name.as_deref())
            })
        && normalize_type_name(method.return_type.as_deref())
            == normalize_type_name(required.return_type.as_deref())
}

fn normalize_type_name(type_name: Option<&str>) -> String {
    type_name.unwrap_or("Void").to_string()
}

fn core_names() -> Vec<&'static str> {
    vec![
        "len", "type", "str", "int", "float", "bool", "map", "filter", "reduce", "nap", "math",
        "path", "chaser", "store", "web", "tick",
    ]
}

fn normalize_embedded_key(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string()
}

#[derive(Clone, Debug)]
struct TypeRef {
    name: String,
    args: Vec<TypeRef>,
}

impl TypeRef {
    fn text(&self) -> String {
        if self.args.is_empty() {
            self.name.clone()
        } else {
            format!(
                "{}<{}>",
                self.name,
                self.args
                    .iter()
                    .map(TypeRef::text)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

fn parse_type_ref(text: &str) -> Result<TypeRef, Diagnostic> {
    let text = text.trim();
    if text.is_empty() {
        return Err(runtime_error("empty type annotation"));
    }
    let Some(start) = text.find('<') else {
        return Ok(TypeRef {
            name: text.to_string(),
            args: Vec::new(),
        });
    };
    if !text.ends_with('>') {
        return Err(runtime_error(format!("invalid type annotation {text}")));
    }
    let name = text[..start].trim().to_string();
    let inner = &text[start + 1..text.len() - 1];
    let args = split_type_args(inner)
        .into_iter()
        .map(|arg| parse_type_ref(&arg))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypeRef { name, args })
}

fn split_type_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0;
    let mut depth = 0_i32;
    for (index, ch) in text.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                args.push(text[start..index].trim().to_string());
                start = index + 1;
            }
            _ => {}
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        args.push(tail.to_string());
    }
    args
}

#[derive(Clone, Debug, Default)]
struct StaticEnv {
    parent: Option<Box<StaticEnv>>,
    values: BTreeMap<String, StaticBinding>,
}

#[derive(Clone, Debug)]
struct StaticBinding {
    type_name: Option<String>,
    is_const: bool,
}

impl StaticEnv {
    fn child(&self) -> StaticEnv {
        StaticEnv {
            parent: Some(Box::new(self.clone())),
            values: BTreeMap::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&StaticBinding> {
        self.values
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.get(name)))
    }
}

impl StoreDatabase {
    fn open(path: PathBuf) -> Result<Self, Diagnostic> {
        let data = if path.exists() {
            let text = fs::read_to_string(&path).map_err(io_error)?;
            serde_json::from_str(&text).map_err(|err| {
                runtime_error(format!("invalid store file {}: {err}", path.display()))
            })?
        } else {
            json!({
                "format": "IMM_STORE_V1",
                "next_id": {},
                "records": {},
            })
        };
        if data.get("format").and_then(JsonValue::as_str) != Some("IMM_STORE_V1") {
            return Err(runtime_error(format!(
                "unsupported store format in {}",
                path.display()
            )));
        }
        Ok(Self { path, data })
    }

    fn flush(&self) -> Result<(), Diagnostic> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
        }
        let tmp = self.path.with_extension(format!(
            "{}tmp",
            self.path
                .extension()
                .map(|ext| format!("{}.", ext.to_string_lossy()))
                .unwrap_or_default()
        ));
        let text = serde_json::to_string_pretty(&self.data)
            .map_err(|err| runtime_error(err.to_string()))?
            + "\n";
        fs::write(&tmp, text).map_err(io_error)?;
        fs::rename(tmp, &self.path).map_err(io_error)?;
        Ok(())
    }

    fn next_id(&mut self, den_name: &str) -> i64 {
        let current = self
            .data
            .get("next_id")
            .and_then(|next| next.get(den_name))
            .and_then(JsonValue::as_i64)
            .unwrap_or(1);
        self.data["next_id"][den_name] = JsonValue::from(current + 1);
        current
    }

    fn records_for(&self, den_name: &str) -> Option<&JsonValue> {
        self.data
            .get("records")
            .and_then(|records| records.get(den_name))
    }

    fn records_for_mut(&mut self, den_name: &str) -> Option<&mut JsonValue> {
        if self
            .data
            .get("records")
            .and_then(JsonValue::as_object)
            .is_none()
        {
            self.data["records"] = json!({});
        }
        if self.data["records"].get(den_name).is_none() {
            self.data["records"][den_name] = json!({});
        }
        self.data
            .get_mut("records")
            .and_then(|records| records.get_mut(den_name))
    }

    fn set_records_for(&mut self, den_name: &str, value: JsonValue) {
        if self
            .data
            .get("records")
            .and_then(JsonValue::as_object)
            .is_none()
        {
            self.data["records"] = json!({});
        }
        self.data["records"][den_name] = value;
    }
}

fn require_arg_count(name: &str, args: &[Value], expected: usize) -> Result<(), Diagnostic> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(runtime_error(format!(
            "{name} expects {expected} arguments, got {}",
            args.len()
        )))
    }
}

fn require_point(value: &Value, message: &str) -> Result<Point, Diagnostic> {
    if let Value::Point(point) = value {
        Ok(*point)
    } else {
        Err(runtime_error(message))
    }
}

fn require_store(value: &Value) -> Result<StoreRef, Diagnostic> {
    if let Value::Store(db) = value {
        Ok(db.clone())
    } else {
        Err(runtime_error("expected Store"))
    }
}

fn require_den_type_name(value: &Value) -> Result<String, Diagnostic> {
    if let Value::DenType(name) = value {
        Ok(name.clone())
    } else {
        Err(runtime_error("expected den type"))
    }
}

fn unwrap_object(value: &Value) -> Result<ObjRef, Diagnostic> {
    match value {
        Value::Object(object) => Ok(object.clone()),
        Value::ObjectView { object, .. } => Ok(object.clone()),
        _ => Err(runtime_error("store.save expects a den object")),
    }
}

fn number_arg(value: &Value, name: &str) -> Result<f64, Diagnostic> {
    match value {
        Value::Int(value) => Ok(*value as f64),
        Value::Float(value) => Ok(*value),
        _ => Err(runtime_error(format!("{name} expects number"))),
    }
}

fn min_max_builtin(name: &str, args: &[Value], is_min: bool) -> Result<Value, Diagnostic> {
    if args.is_empty() {
        return Err(runtime_error(format!(
            "{name} expects at least one argument"
        )));
    }
    let mut best = number_arg(&args[0], name)?;
    for value in &args[1..] {
        let value = number_arg(value, name)?;
        if (is_min && value < best) || (!is_min && value > best) {
            best = value;
        }
    }
    if best.fract() == 0.0 {
        Ok(Value::Int(best as i64))
    } else {
        Ok(Value::Float(best))
    }
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Int(value) => *value != 0,
        Value::Float(value) => *value != 0.0,
        Value::String(value) => !value.is_empty(),
        Value::Array(values) => !values.borrow().is_empty(),
        Value::Map(values) => !values.borrow().is_empty(),
        _ => true,
    }
}

fn chaser_direction(src: Point, dst: Point) -> &'static str {
    if dst.x > src.x {
        "RIGHT"
    } else if dst.x < src.x {
        "LEFT"
    } else if dst.y > src.y {
        "DOWN"
    } else if dst.y < src.y {
        "UP"
    } else {
        "STAY"
    }
}

fn chaser_safe_moves(field: &Value, pos: &Value, wall: &Value) -> Result<Vec<Value>, Diagnostic> {
    let Value::Matrix(matrix) = field else {
        return Err(runtime_error(
            "chaser.safe_moves expects Matrix, Point, wall value",
        ));
    };
    let pos = require_point(pos, "chaser.safe_moves expects Matrix, Point, wall value")?;
    let mut moves = Vec::new();
    for neighbor in matrix_neighbors4(&matrix.borrow(), pos) {
        let Value::Point(point) = neighbor else {
            unreachable!();
        };
        let cell = matrix_get(&matrix.borrow(), &[Value::Point(point)], false)?;
        if !value_eq(&cell, wall) {
            moves.push(Value::Point(point));
        }
    }
    Ok(moves)
}

fn matrix_zero_arg(
    receiver: &Value,
    args: &[Value],
    build: impl FnOnce(&MatrixData) -> Value,
) -> Result<Value, Diagnostic> {
    require_arg_count("Matrix method", args, 0)?;
    if let Value::Matrix(matrix) = receiver {
        Ok(build(&matrix.borrow()))
    } else {
        unreachable!()
    }
}

fn percent_decode(text: &str) -> Result<String, Diagnostic> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(runtime_error("invalid percent escape"));
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                .map_err(|err| runtime_error(err.to_string()))?;
            let value =
                u8::from_str_radix(hex, 16).map_err(|err| runtime_error(err.to_string()))?;
            out.push(value);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|err| runtime_error(err.to_string()))
}

fn json_to_value(value: &JsonValue) -> Result<Value, Diagnostic> {
    match value {
        JsonValue::Null => Ok(Value::Null),
        JsonValue::Bool(value) => Ok(Value::Bool(*value)),
        JsonValue::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(Value::Int(value))
            } else {
                Ok(Value::Float(value.as_f64().unwrap_or_default()))
            }
        }
        JsonValue::String(value) => Ok(Value::String(value.clone())),
        JsonValue::Array(values) => Ok(Value::Array(Rc::new(RefCell::new(
            values
                .iter()
                .map(json_to_value)
                .collect::<Result<Vec<_>, _>>()?,
        )))),
        JsonValue::Object(values) => Ok(Value::Map(Rc::new(RefCell::new(
            values
                .iter()
                .map(|(key, value)| json_to_value(value).map(|value| (key.clone(), value)))
                .collect::<Result<BTreeMap<_, _>, _>>()?,
        )))),
    }
}

pub fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => {
            let values = values.borrow();
            format!(
                "[{}]",
                values
                    .iter()
                    .map(format_value)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Value::Map(values) => {
            let values = values.borrow();
            format!(
                "{{{}}}",
                values
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", format_value(value)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
        Value::Matrix(matrix) => format!(
            "matrix {}",
            format_value(&Value::Array(Rc::new(RefCell::new(
                matrix
                    .borrow()
                    .rows
                    .iter()
                    .map(|row| Value::Array(Rc::new(RefCell::new(row.clone()))))
                    .collect()
            ))))
        ),
        Value::Point(point) => format!("({},{})", point.x, point.y),
        Value::Range(start, end) => format!("{start}..{end}"),
        Value::Function(function) => format!("<dig {}>", function.name),
        Value::Lambda(_) => "<lambda>".to_string(),
        Value::Builtin(_) => "<builtin>".to_string(),
        Value::NativeMethod(_) => "<method>".to_string(),
        Value::Namespace(namespace) => format!("<module {}>", namespace.name),
        Value::DenType(name) => format!("<den {name}>"),
        Value::MaskType(name) => format!("<mask {name}>"),
        Value::Object(object) => format!("<{} object>", object.borrow().den_name),
        Value::ObjectView { object, mask } => {
            format!("<{} view of {}>", mask, object.borrow().den_name)
        }
        Value::ObjectMethod(method) => {
            format!("<method {}.{}>", method.method.owner, method.method.name)
        }
        Value::UnderProxy { .. } => "<under>".to_string(),
        Value::Response(response) => format!("<Response {} {}>", response.status, response.url),
        Value::Task(_) => "<task>".to_string(),
        Value::TaskGroup(values) => format!("<task-group {}>", values.len()),
        Value::Store(db) => format!("<store {}>", db.borrow().path.display()),
    }
}

pub fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "Null",
        Value::Bool(_) => "Bool",
        Value::Int(_) => "Int",
        Value::Float(_) => "Float",
        Value::String(_) => "String",
        Value::Array(_) => "Array",
        Value::Map(_) => "Map",
        Value::Matrix(_) => "Matrix",
        Value::Point(_) => "Point",
        Value::Range(_, _) => "Range",
        Value::Function(_) | Value::Lambda(_) | Value::Builtin(_) | Value::NativeMethod(_) => {
            "Function"
        }
        Value::Namespace(_) => "Module",
        Value::DenType(_) => "Den",
        Value::MaskType(_) => "Mask",
        Value::Object(_) => "Object",
        Value::ObjectView { .. } => "Object",
        Value::ObjectMethod(_) => "Function",
        Value::UnderProxy { .. } => "Under",
        Value::Response(_) => "Response",
        Value::Task(_) => "Task",
        Value::TaskGroup(_) => "TaskGroup",
        Value::Store(_) => "Store",
    }
}

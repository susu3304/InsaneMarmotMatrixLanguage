#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    Main { body: Vec<Stmt>, insane: bool },
    HowlMain { body: Vec<Stmt>, insane: bool },
    Function(FunctionDef),
    HowlFunction(FunctionDef),
    Den(DenDef),
    Mask(MaskDef),
    Use(String),
    Module(String),
    Probe { name: String, body: Vec<Stmt> },
    Pack(PackConfig),
    Stmt(Stmt),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunctionDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: String,
    pub type_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DenDef {
    pub name: String,
    pub parent: Option<String>,
    pub masks: Vec<String>,
    pub members: Vec<DenMember>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DenMember {
    Field(FieldDef),
    Method(MethodDef),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldDef {
    pub name: String,
    pub type_name: Option<String>,
    pub expr: Option<Expr>,
    pub access: Access,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MethodDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Stmt>,
    pub access: Access,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskDef {
    pub name: String,
    pub methods: Vec<MaskMethod>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MaskMethod {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Access {
    Fur,
    Fang,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackConfig {
    pub entry: Option<String>,
    pub crate_path: Option<String>,
    pub pelt: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stmt {
    Let {
        name: String,
        expr: Expr,
        type_name: Option<String>,
        is_const: bool,
    },
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_body: Option<ElseBody>,
    },
    For {
        name: String,
        iterable: Expr,
        body: Vec<Stmt>,
        insane: bool,
    },
    While {
        condition: Expr,
        body: Vec<Stmt>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Expr(Expr),
    Squeak(Vec<Expr>),
    Panic(Expr),
    Expect(Expr),
    Trace(Vec<Expr>),
    Try {
        body: Vec<Stmt>,
        catch_name: Option<String>,
        catch_body: Option<Vec<Stmt>>,
        insane: bool,
    },
    InsaneBlock(Vec<Stmt>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum ElseBody {
    If(Box<Stmt>),
    Block(Vec<Stmt>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Var(String),
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Unary {
        op: String,
        expr: Box<Expr>,
    },
    Array(Vec<Expr>),
    Matrix(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Point {
        x: Box<Expr>,
        y: Box<Expr>,
    },
    Hatch {
        name: String,
        args: Vec<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    Index {
        target: Box<Expr>,
        args: Vec<Expr>,
    },
    Member {
        target: Box<Expr>,
        name: String,
    },
    Lambda {
        params: Vec<String>,
        body: LambdaBody,
    },
    Tunnel {
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
    InsaneChoose(Box<Expr>),
    Wait(Box<Expr>),
    Scatter {
        expr: Box<Expr>,
        insane: bool,
    },
    Nest(Vec<Expr>),
    Sniff,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LambdaBody {
    Expr(Box<Expr>),
    Block(Vec<Stmt>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

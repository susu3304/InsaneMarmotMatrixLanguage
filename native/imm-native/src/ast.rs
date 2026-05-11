#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Program {
    pub items: Vec<Item>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Item {
    Main,
    HowlMain,
    Function(String),
    HowlFunction(String),
    Den(String),
    Mask(String),
    Module(String),
    Use(String),
    Probe(String),
    Pack(PackConfig),
    Statement,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PackConfig {
    pub entry: Option<String>,
    pub crate_path: Option<String>,
    pub pelt: Option<String>,
}

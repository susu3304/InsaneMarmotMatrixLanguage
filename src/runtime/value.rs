use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
    Point { x: i64, y: i64 },
    Matrix(Vec<Vec<Value>>),
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "Null",
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::String(_) => "String",
            Self::Array(_) => "Array",
            Self::Map(_) => "Map",
            Self::Point { .. } => "Point",
            Self::Matrix(_) => "Matrix",
        }
    }

    pub fn format_imm(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(true) => "true".to_string(),
            Self::Bool(false) => "false".to_string(),
            Self::Int(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Array(values) => format_sequence(values),
            Self::Map(values) => {
                let parts = values
                    .iter()
                    .map(|(key, value)| format!("{key}: {}", value.format_imm()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{parts}}}")
            }
            Self::Point { x, y } => format!("({x},{y})"),
            Self::Matrix(rows) => rows
                .iter()
                .map(|row| format_sequence(row))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }
}

fn format_sequence(values: &[Value]) -> String {
    let body = values
        .iter()
        .map(Value::format_imm)
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{body}]")
}

use crate::runtime::Value;

pub fn point(x: i64, y: i64) -> Value {
    Value::Point { x, y }
}

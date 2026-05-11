use crate::diagnostics::Diagnostic;
use crate::runtime::Value;

pub fn evaluate_reference_boundary() -> Result<Value, Diagnostic> {
    Ok(Value::Null)
}

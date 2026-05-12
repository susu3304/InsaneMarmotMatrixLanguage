use std::collections::BTreeMap;

use crate::runtime::Value;

#[derive(Clone, Debug)]
pub struct Binding {
    pub value: Value,
    pub is_stash: bool,
    pub type_annotation: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Environment {
    bindings: BTreeMap<String, Binding>,
}

impl Environment {
    pub fn define(&mut self, name: impl Into<String>, binding: Binding) {
        self.bindings.insert(name.into(), binding);
    }

    pub fn get(&self, name: &str) -> Option<&Binding> {
        self.bindings.get(name)
    }
}

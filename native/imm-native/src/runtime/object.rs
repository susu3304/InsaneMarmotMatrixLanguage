#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DenType {
    pub name: String,
    pub parent: Option<String>,
    pub masks: Vec<String>,
}

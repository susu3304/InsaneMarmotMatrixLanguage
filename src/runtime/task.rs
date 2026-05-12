#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskState {
    Pending,
    Ready,
    Failed,
    Canceled,
}

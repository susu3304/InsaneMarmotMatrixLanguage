#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub file_id: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(
        file_id: usize,
        byte_start: usize,
        byte_end: usize,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            file_id,
            byte_start,
            byte_end,
            line,
            column,
        }
    }
}

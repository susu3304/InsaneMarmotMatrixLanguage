use std::fmt;
use std::path::PathBuf;

use crate::source::Span;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Category {
    Syntax,
    Static,
    Runtime,
    Module,
    Io,
    Network,
    Pack,
    NotImplemented,
}

impl Category {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Syntax => "syntax",
            Self::Static => "static",
            Self::Runtime => "runtime",
            Self::Module => "module",
            Self::Io => "IO",
            Self::Network => "network",
            Self::Pack => "pack",
            Self::NotImplemented => "not implemented",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub category: Category,
    pub message: String,
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn new(category: Category, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
            span: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = self.span {
            write!(
                f,
                "{}:{}:{}: {} error: {}",
                span.file_id,
                span.line,
                span.column,
                self.category.as_str(),
                self.message
            )
        } else {
            write!(f, "{} error: {}", self.category.as_str(), self.message)
        }
    }
}

impl std::error::Error for Diagnostic {}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("{0}")]
    Diagnostic(#[from] Diagnostic),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not locate IMM repository root from {0:?}")]
    RepoRoot(PathBuf),
    #[error("reference interpreter exited without an exit status")]
    MissingExitStatus,
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

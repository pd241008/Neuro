use miette::Diagnostic;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum NeuroError {
    #[error("{0}")]
    #[diagnostic(help("Check the source file for issues at the reported location"))]
    AnalysisError(
        String,
        #[label("here")] Option<miette::SourceSpan>,
    ),

    #[error("Failed to deserialize AST: {0}")]
    DeserializationError(String),

    #[error("{0}")]
    IoError(String),
}

impl NeuroError {
    pub fn analysis(msg: impl Into<String>) -> Self {
        NeuroError::AnalysisError(msg.into(), None)
    }

    pub fn with_span(msg: impl Into<String>, span: miette::SourceSpan) -> Self {
        NeuroError::AnalysisError(msg.into(), Some(span))
    }
}

impl From<String> for NeuroError {
    fn from(msg: String) -> Self {
        NeuroError::analysis(msg)
    }
}

impl From<&str> for NeuroError {
    fn from(msg: &str) -> Self {
        NeuroError::analysis(msg.to_string())
    }
}

impl From<std::io::Error> for NeuroError {
    fn from(e: std::io::Error) -> Self {
        NeuroError::IoError(e.to_string())
    }
}

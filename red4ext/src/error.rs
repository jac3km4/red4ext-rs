use thiserror::Error;

#[derive(Debug, Error)]
pub enum InvokeError {
    #[error("function not found")]
    FunctionNotFound,
    #[error("function is not valid")]
    InvalidFunction,
    #[error("expected {expected} arguments, but {given} given")]
    InvalidArgCount { given: usize, expected: usize },
    #[error("expected {expected} argument at index {index}")]
    ArgMismatch {
        expected: &'static str,
        index: usize,
    },
    #[error("return type mismatch, expected {expected}")]
    ReturnMismatch { expected: &'static str },
}

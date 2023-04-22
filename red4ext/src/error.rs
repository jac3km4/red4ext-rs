#[derive(Debug)]
pub enum InvokeError {
    FunctionNotFound,
    InvalidFunction,
    InvalidArgCount {
        given: usize,
        expected: usize,
    },
    ArgMismatch {
        expected: &'static str,
        index: usize,
    },
    ReturnMismatch {
        expected: &'static str,
    },
}

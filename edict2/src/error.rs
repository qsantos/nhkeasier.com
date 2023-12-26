#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub enum Error {
    ParseError {
        lineno: usize,
        expected: &'static str,
    },
    ParseInteger {
        lineno: usize,
    },
}

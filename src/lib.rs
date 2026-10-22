pub mod ast;
mod builtins;
pub mod error;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod value;

pub use error::{Error, ErrorKind, Span};
pub use interpreter::Interpreter;
pub use parser::{parse, parse_expression};
pub use value::Value;

pub fn run<W: std::io::Write, R: std::io::BufRead>(
    source: &str,
    output: W,
    input: R,
) -> Result<W, (W, Error)> {
    let program = match parse(source) {
        Ok(program) => program,
        Err(error) => return Err((output, error)),
    };
    let mut interpreter = Interpreter::new(output, input);
    match interpreter.run(&program) {
        Ok(()) => Ok(interpreter.into_output()),
        Err(error) => Err((interpreter.into_output(), error)),
    }
}

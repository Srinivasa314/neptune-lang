mod bytecode_compiler;
mod parser;
mod scanner;
mod vm;

#[derive(Debug)]
pub struct CompileError {
    message: String,
    line: u32,
}

pub type CompileResult<T> = Result<T, CompileError>;

fn main() {
    todo!()
}

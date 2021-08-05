mod bytecode;
mod bytecode_compiler;
mod gc;
mod objects;
mod parser;
mod scanner;
mod util;
mod value;
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

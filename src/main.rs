mod bytecode;
mod bytecode_compiler;
mod gc;
mod parser;
mod scanner;
mod value;
mod vm;
mod util;

#[derive(Debug)]
pub struct CompileError {
    message: String,
    line: u32,
}

type CompileResult<T> = Result<T, CompileError>;
fn main() {
    /*let tokens = scanner::Scanner::new("let x=0\nlet y=10\nx*(x+y)\n").scan_tokens();
    let ast = parser::Parser::new(tokens.into_iter()).parse();
    dbg!(ast.1);
    dbg!(&ast.0);
    let mut bytecode_writer = bytecode::BytecodeWriter::new();
    bytecode_writer.evaluate_statements(&ast.0).unwrap();
    dbg!(bytecode_writer.bytecode());*/
}

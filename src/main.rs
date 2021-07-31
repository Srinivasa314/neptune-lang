mod bytecode;
mod bytecode_compiler;
mod gc;
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

type CompileResult<T> = Result<T, CompileError>;
fn main() {
    let tokens = scanner::Scanner::new("{let x=0\nlet y=10\nx*(x+y)}\n").scan_tokens();
    let ast = parser::Parser::new(tokens.into_iter()).parse();
    dbg!(ast.1);
    dbg!(&ast.0);
    let mut bytecode_compiler = bytecode_compiler::BytecodeCompiler::new();
    bytecode_compiler.evaluate_statements(&ast.0).unwrap();
    dbg!(bytecode_compiler.writer.bytecode());
}

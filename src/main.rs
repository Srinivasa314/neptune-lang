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

type CompileResult<T> = Result<T, CompileError>;
fn main() {
    let tokens =
        scanner::Scanner::new("\"Hello\"")
            .scan_tokens();
    let ast = parser::Parser::new(tokens.into_iter()).parse();
    dbg!(ast.1);
    dbg!(&ast.0);
    let gc = gc::GC::new();
    let mut compiler = bytecode_compiler::Compiler::new(&gc);
    let mut bytecode_compiler = bytecode_compiler::BytecodeCompiler::new(&mut compiler);
    bytecode_compiler.evaluate_statments(&ast.0);
    dbg!(bytecode_compiler.writer.bytecode());
}

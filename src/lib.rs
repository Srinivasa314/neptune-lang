use bytecode_compiler::Compiler;
use cxx::UniquePtr;
use parser::Parser;
use scanner::Scanner;
use vm::{new_vm, VMResult, VM};

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

pub struct Neptune {
    inner: UniquePtr<VM>,
}

impl Neptune {
    pub fn new() -> Self {
        Self { inner: new_vm() }
    }

    pub fn exec(&self, source: &str) -> Result<(), Vec<CompileError>> {
        let scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens();
        let parser = Parser::new(tokens.into_iter());
        let ast = parser.parse();
        dbg!(&ast);
        let compiler = Compiler::new(&self.inner);
        let mut fw = compiler.compile(ast.0);
        let mut errors = ast.1;
        if let Err(e) = &mut fw {
            errors.append(e);
        }
        if errors.is_empty() {
            match fw.unwrap().run() {
                VMResult::Success => Ok(()),
                VMResult::Error => todo!(),
                _ => unreachable!(),
            }
        } else {
            errors.sort_by(|e1, e2| e1.line.cmp(&e2.line));
            Err(errors)
        }
    }
}

impl Default for Neptune {
    fn default() -> Self {
        Self::new()
    }
}

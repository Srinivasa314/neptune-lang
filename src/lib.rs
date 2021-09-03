use bytecode_compiler::Compiler;
use cxx::UniquePtr;
use parser::Parser;
use scanner::Scanner;
use vm::{new_vm, VM};

use crate::vm::VMStatus;

mod bytecode_compiler;
mod parser;
mod scanner;
mod vm;

#[derive(Debug)]
pub struct CompileError {
    pub message: String,
    pub line: u32,
}

#[derive(Debug)]
pub enum InterpretError {
    CompileError(Vec<CompileError>),
    RuntimePanic(String),
}

pub type CompileResult<T> = Result<T, CompileError>;

pub struct Neptune {
    inner: UniquePtr<VM>,
}

impl Neptune {
    pub fn new() -> Self {
        Self { inner: new_vm() }
    }

    pub fn run(&self, source: &str) -> Result<String, InterpretError> {
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
            dbg!(fw.as_ref().unwrap());
            let vm_result = unsafe { fw.unwrap().run() };
            match vm_result.get_status() {
                VMStatus::Success => Ok(vm_result.get_result().to_string()),
                VMStatus::Error => Err(InterpretError::RuntimePanic(
                    vm_result.get_result().to_string(),
                )),
                _ => unreachable!(),
            }
        } else {
            errors.sort_by(|e1, e2| e1.line.cmp(&e2.line));
            Err(InterpretError::CompileError(errors))
        }
    }
}

impl Default for Neptune {
    fn default() -> Self {
        Self::new()
    }
}

use crate::vm::VMStatus;
use bytecode_compiler::Compiler;
use cxx::UniquePtr;
use parser::Parser;
use scanner::Scanner;
use serde::{Deserialize, Serialize};
use vm::{new_vm, FunctionContext, VM};

mod bytecode_compiler;
mod parser;
mod scanner;
mod vm;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
    pub line: u32,
}

#[derive(Debug)]
pub enum InterpretError {
    CompileError(Vec<CompileError>),
    RuntimePanic { error: String, stack_trace: String },
}

pub type CompileResult<T> = Result<T, CompileError>;

#[derive(Debug)]
pub struct FunctionRedeclarationError;

pub struct Neptune {
    vm: UniquePtr<VM>,
}

impl Neptune {
    pub fn new() -> Self {
        Self { vm: new_vm() }
    }

    pub fn exec(&self, source: &str) -> Result<(), InterpretError> {
        let scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens();
        let parser = Parser::new(tokens.into_iter());
        let ast = parser.parse(false);
        let compiler = Compiler::new(&self.vm);
        let mut fw = compiler.exec(ast.0);
        let mut errors = ast.1;
        if let Err(e) = &mut fw {
            errors.append(e);
        }
        if errors.is_empty() {
            let mut fw = fw.unwrap();
            let vm_result = unsafe { fw.run(false) };
            match vm_result.get_status() {
                VMStatus::Success => Ok(()),
                VMStatus::Error => Err(InterpretError::RuntimePanic {
                    error: vm_result.get_result().to_string(),
                    stack_trace: vm_result.get_stack_trace().to_string(),
                }),
                _ => unreachable!(),
            }
        } else {
            errors.sort_by(|e1, e2| e1.line.cmp(&e2.line));
            Err(InterpretError::CompileError(errors))
        }
    }

    pub fn eval(&self, source: &str) -> Result<Option<String>, InterpretError> {
        let scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens();
        let parser = Parser::new(tokens.into_iter());
        let ast = parser.parse(true);
        let compiler = Compiler::new(&self.vm);
        let is_expr;
        let mut fw = if let Some(expr) = Compiler::can_eval(&ast.0) {
            is_expr = true;
            compiler.eval(expr)
        } else {
            is_expr = false;
            compiler.exec(ast.0)
        };
        let mut errors = ast.1;
        if let Err(e) = &mut fw {
            errors.append(e);
        }
        if errors.is_empty() {
            let mut fw = fw.unwrap();
            let vm_result = unsafe { fw.run(true) };
            match vm_result.get_status() {
                VMStatus::Success => Ok(if is_expr {
                    Some(vm_result.get_result().to_string())
                } else {
                    None
                }),
                VMStatus::Error => Err(InterpretError::RuntimePanic {
                    error: vm_result.get_result().to_string(),
                    stack_trace: vm_result.get_stack_trace().to_string(),
                }),
                _ => unreachable!(),
            }
        } else {
            errors.sort_by(|e1, e2| e1.line.cmp(&e2.line));
            Err(InterpretError::CompileError(errors))
        }
    }

    pub fn create_function(
        &self,
        name: &str,
        arity: u8,
        extra_slots: u16,
        callback: impl FnMut(FunctionContext) -> Result<u16, u16> + 'static,
    ) -> Result<(), FunctionRedeclarationError> {
        if self
            .vm
            .declare_native_rust_function(name, arity, extra_slots, callback)
        {
            Ok(())
        } else {
            Err(FunctionRedeclarationError)
        }
    }
}

impl Default for Neptune {
    fn default() -> Self {
        Self::new()
    }
}

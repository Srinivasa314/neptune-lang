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

#[derive(Debug, Serialize, Deserialize)]
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
#[cfg(test)]
mod tests {
    use crate::{InterpretError, Neptune};
    use std::{
        env,
        fs::File,
        io::{Read, Write},
        path::PathBuf,
    };
    fn open(file: &str) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests");
        path.push(file);
        path
    }
    fn read(file: &str) -> String {
        let path = open(file);
        let mut s = String::new();
        File::open(path).unwrap().read_to_string(&mut s).unwrap();
        s
    }
    #[test]
    fn test() {
        let n = Neptune::new();
        assert_eq!(n.eval("1+1").unwrap().unwrap(), "2");
        assert_eq!(n.eval("'a'~'b'").unwrap().unwrap(), "'ab'");
        assert_eq!(n.eval("0.1+0.2").unwrap().unwrap(), "0.3");

        assert_eq!(n.eval("1.0/0.0").unwrap().unwrap(), "inf");
        assert_eq!(n.eval("-1.0/0.0").unwrap().unwrap(), "-inf");

        assert!(matches!(
            n.eval("0.0/0.0").unwrap().unwrap().as_str(),
            "-nan" | "nan"
        ));

        n.exec("let a=[0];a[0]=a").unwrap();
        assert_eq!(
            n.eval("a").unwrap().unwrap(),
            "[ [ [ [ [ [ [ [ [ [ [ [ ... ] ] ] ] ] ] ] ] ] ] ] ]"
        );

        n.exec(
            r"
        let m=null
        for i in 0..50 {
            m={@next:m}
        }
        ",
        )
        .unwrap();
        assert_eq!(
            n.eval("m").unwrap().unwrap(),
            "{ @next: { @next: { @next: { @next: { @next: { @next: { @next: { @next: { @next: { @next: { @next: { ... } } } } } } } } } } } }"
        );

        for test in ["assert_eq.np", "assert_failed.np", "assert_failed2.np"] {
            if let InterpretError::CompileError(_) = n.exec(&read(test)).unwrap_err() {
                panic!("Expected a runtime error")
            }
        }
        if let Err(e) = n.exec(&read("test.np")) {
            panic!("{:?}", e);
        }
        let errors: Vec<String> = serde_json::from_str(&read("errors.json")).unwrap();
        for error in errors {
            let res = n.exec(&read(&format!("{}.np", error))).unwrap_err();
            if let InterpretError::CompileError(res) = res {
                let result = serde_json::to_string_pretty(&res).unwrap();
                if env::var("NEPTUNE_GEN_ERRORS").is_ok() {
                    File::create(open(&format!("{}.json", error)))
                        .unwrap()
                        .write(result.as_bytes())
                        .unwrap();
                } else {
                    let mut expected_result = String::new();
                    File::open(open(&format!("{}.json", error)))
                        .unwrap()
                        .read_to_string(&mut expected_result)
                        .unwrap();
                    assert_eq!(expected_result, result);
                }
            } else {
                panic!("Expected a compile error");
            }
        }
    }
}

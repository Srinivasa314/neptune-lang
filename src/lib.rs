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
pub enum FunctionDeclarationError {
    FunctionRedeclarationError,
    ModuleNotFound,
}

pub struct Neptune {
    vm: UniquePtr<VM>,
}

impl Neptune {
    pub fn new() -> Self {
        let vm = new_vm();
        Self { vm }
    }

    fn run(&self, module: String, source: &str, eval: bool) -> Result<bool, InterpretError> {
        if !self.vm.module_exists(module.as_str().into()) {
            self.vm.create_module_with_prelude(module.as_str().into());
        }
        let scanner = Scanner::new(source);
        let tokens = scanner.scan_tokens();
        let parser = Parser::new(tokens.into_iter());
        let ast = parser.parse(eval);
        let compiler = Compiler::new(&self.vm, module);
        let mut is_expr = false;
        let mut fw = if eval {
            if let Some(expr) = Compiler::can_eval(&ast.0) {
                is_expr = true;
                compiler.eval(expr)
            } else {
                is_expr = false;
                compiler.exec(ast.0)
            }
        } else {
            compiler.exec(ast.0)
        };
        let mut errors = ast.1;
        if let Err(e) = &mut fw {
            errors.append(e);
        }
        if errors.is_empty() {
            let mut fw = fw.unwrap();
            match unsafe { fw.run() } {
                VMStatus::Success => Ok(is_expr),
                VMStatus::Error => Err(InterpretError::RuntimePanic {
                    error: self.vm.get_result(),
                    stack_trace: self.vm.get_stack_trace(),
                }),
                _ => unreachable!(),
            }
        } else {
            errors.sort_by(|e1, e2| e1.line.cmp(&e2.line));
            Err(InterpretError::CompileError(errors))
        }
    }

    pub fn exec<S: Into<String>>(&self, module: S, source: &str) -> Result<(), InterpretError> {
        self.run(module.into(), source, false).map(|_| ())
    }

    pub fn eval<S: Into<String>>(
        &self,
        module: S,
        source: &str,
    ) -> Result<Option<String>, InterpretError> {
        match self.run(module.into(), source, true) {
            Ok(is_expr) => {
                if is_expr {
                    Ok(Some(self.vm.get_result()))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(e),
        }
    }

    pub fn create_function(
        &self,
        module: &str,
        name: &str,
        arity: u8,
        extra_slots: u16,
        callback: impl FnMut(FunctionContext) -> Result<u16, u16> + 'static,
    ) -> Result<(), FunctionDeclarationError> {
        if !self.vm.module_exists(module.into()) {
            return Err(FunctionDeclarationError::ModuleNotFound);
        }
        if self
            .vm
            .declare_native_rust_function(module, name, arity, extra_slots, callback)
        {
            Ok(())
        } else {
            Err(FunctionDeclarationError::FunctionRedeclarationError)
        }
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
        assert_eq!(n.eval("fun f(){}", "").unwrap(), None);
        for test in ["assert_eq.np", "assert_failed.np", "assert_failed2.np"] {
            if let InterpretError::CompileError(_) = n.exec(test, &read(test)).unwrap_err() {
                panic!("Expected a runtime error")
            }
        }
        if let InterpretError::RuntimePanic { error, stack_trace } = n
            .exec(
                "<script>",
                r#"
            fun f(){
                fun g(){
                    fun h(){
                        panic 'abc'
                    }
                    h()
                }
                g()
            }
            f()
        "#,
            )
            .unwrap_err()
        {
            assert_eq!(error, "'abc'");
            assert_eq!(stack_trace, "at h (<script>:5)\nat g (<script>:7)\nat f (<script>:9)\nat <script> (<script>:11)\n");
        } else {
            panic!("Expected error");
        }
        if let Err(e) = n.exec("test.np", &read("test.np")) {
            panic!("{:?}", e);
        }
        let errors: Vec<String> = serde_json::from_str(&read("errors.json")).unwrap();
        for error in errors {
            let fname = format!("{}.np", error);
            let source = read(&fname);
            let res = n.exec(fname, &source).unwrap_err();
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

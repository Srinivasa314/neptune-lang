use crate::vm::VMStatus;
use bytecode_compiler::Compiler;
use cxx::UniquePtr;
use parser::Parser;
use scanner::Scanner;
use serde::{Deserialize, Serialize};
use vm::{new_vm, FunctionContext, FunctionInfoWriter, VM};

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
pub enum NeptuneError {
    FunctionAlreadyExists,
    ModuleNotFound,
    ModuleAlreadyExists,
}

pub struct Neptune {
    vm: UniquePtr<VM>,
}

impl Neptune {
    pub fn new(
        mut resolve_module: Option<Box<dyn FnMut(&str, &str) -> Option<String>>>,
        mut get_module_source: Option<Box<dyn FnMut(&str) -> Option<String>>>,
    ) -> Self {
        assert!(
            (resolve_module.is_some() && get_module_source.is_some())
                || (resolve_module.is_none() && get_module_source.is_none())
        );
        let vm = new_vm();
        vm.declare_native_rust_function("prelude", "_compileModule", false, 1, 0, move |ctx| {
            let vm = ctx.vm();
            let module = match ctx.as_string(0) {
                Some(module) => module,
                None => {
                    ctx.string(0, "module must be a string");
                    return Err(0);
                }
            };
            if let Some(get_module_source) = get_module_source.as_mut() {
                let source = get_module_source(&module);
                if let Some(source) = source {
                    match compile(vm, module, &source, false, false) {
                        Ok((f, _)) => {
                            unsafe {
                                ctx.function(0, f);
                            }
                            Ok(0)
                        }
                        Err(e) => {
                            let error = format!("{:?}", e);
                            ctx.string(0, &error);
                            Err(0)
                        }
                    }
                } else {
                    ctx.string(0, &format!("cannot get source of module {}", &module));
                    Err(0)
                }
            } else {
                ctx.string(0, &format!("module {} does not exist", &module));
                Err(0)
            }
        });
        vm.declare_native_rust_function("prelude", "_resolveModule", false, 2, 0, move |ctx| {
            let caller_module = match ctx.as_string(0) {
                Some(module) => module,
                None => {
                    ctx.string(0, "callerModule must be a string");
                    return Err(0);
                }
            };
            let module_name = match ctx.as_string(1) {
                Some(module) => module,
                None => {
                    ctx.string(0, "module name must be a string");
                    return Err(0);
                }
            };
            if let Some(resolve_module) = resolve_module.as_mut() {
                match resolve_module(&caller_module, &module_name) {
                    Some(s) => {
                        ctx.string(0, &s);
                        Ok(0)
                    }
                    None => {
                        ctx.string(0, &format!("module {} does not exist", &module_name));
                        Err(0)
                    }
                }
            } else {
                ctx.string(0, &format!("module {} does not exist", &module_name));
                Err(0)
            }
        });
        let n = Self { vm };
        n.exec("prelude", include_str!("prelude.np")).unwrap();
        n
    }

    pub fn exec<S: Into<String>>(&self, module: S, source: &str) -> Result<(), InterpretError> {
        match compile(&self.vm, module.into(), source, false, true) {
            Ok((mut f, _)) => match unsafe { f.run() } {
                VMStatus::Success => Ok(()),
                VMStatus::Error => Err(InterpretError::RuntimePanic {
                    error: self.vm.get_result(),
                    stack_trace: self.vm.get_stack_trace(),
                }),
                _ => unreachable!(),
            },
            Err(e) => Err(InterpretError::CompileError(e)),
        }
    }

    pub fn eval<S: Into<String>>(
        &self,
        module: S,
        source: &str,
    ) -> Result<Option<String>, InterpretError> {
        match compile(&self.vm, module.into(), source, true, true) {
            Ok((mut f, is_expr)) => match unsafe { f.run() } {
                VMStatus::Success => Ok(if is_expr {
                    Some(self.vm.get_result())
                } else {
                    None
                }),
                VMStatus::Error => Err(InterpretError::RuntimePanic {
                    error: self.vm.get_result(),
                    stack_trace: self.vm.get_stack_trace(),
                }),
                _ => unreachable!(),
            },
            Err(e) => Err(InterpretError::CompileError(e)),
        }
    }

    pub fn create_function(
        &self,
        module: &str,
        name: &str,
        arity: u8,
        extra_slots: u16,
        callback: impl FnMut(FunctionContext) -> Result<u16, u16> + 'static,
    ) -> Result<(), NeptuneError> {
        if !self.vm.module_exists(module.into()) {
            return Err(NeptuneError::ModuleNotFound);
        }
        if self
            .vm
            .declare_native_rust_function(module, name, true, arity, extra_slots, callback)
        {
            Ok(())
        } else {
            Err(NeptuneError::FunctionAlreadyExists)
        }
    }

    pub fn create_module(&self, name: &str) -> Result<(), NeptuneError> {
        if self.vm.module_exists(name.into()) {
            Err(NeptuneError::ModuleAlreadyExists)
        } else {
            self.vm.create_module(name.into());
            Ok(())
        }
    }
}

fn compile<'vm>(
    vm: &'vm VM,
    module: String,
    source: &str,
    eval: bool,
    exit: bool,
) -> Result<(FunctionInfoWriter<'vm>, bool), Vec<CompileError>> {
    if !vm.module_exists(module.as_str().into()) {
        vm.create_module_with_prelude(module.as_str().into());
    }
    let scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens();
    let parser = Parser::new(tokens.into_iter());
    let ast = parser.parse(eval);
    let compiler = Compiler::new(vm, module, exit);
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
        Ok((fw.unwrap(), is_expr))
    } else {
        errors.sort_by(|e1, e2| e1.line.cmp(&e2.line));
        Err(errors)
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
        let n = Neptune::new(None, None);
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

use crate::vm::VMStatus;
use bytecode_compiler::Compiler;
use cxx::UniquePtr;
use parser::Parser;
use scanner::Scanner;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fmt::Display;
use vm::{new_vm, FunctionInfoWriter, VM};
pub use vm::{EFuncContext, EFuncError, ToNeptuneValue};
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
    UncaughtException(String),
}

impl Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.message)
    }
}

impl Display for InterpretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InterpretError::CompileError(c) => {
                for error in c {
                    write!(f, "{}\n", error)?;
                }
            }
            InterpretError::UncaughtException(error) => {
                write!(f, "Uncaught Exception:\n{}", error)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for InterpretError {}

pub type CompileResult<T> = Result<T, CompileError>;

#[derive(Debug)]
pub enum NeptuneError {
    FunctionAlreadyExists,
    ModuleNotFound,
    ModuleAlreadyExists,
    EFuncAlreadyExists,
}

impl Display for NeptuneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NeptuneError::FunctionAlreadyExists => {
                f.write_str("A function with the same name already exists")
            }
            NeptuneError::ModuleNotFound => f.write_str("The module cannot be found"),
            NeptuneError::ModuleAlreadyExists => {
                f.write_str("A module with the same name already exists")
            }
            NeptuneError::EFuncAlreadyExists => {
                f.write_str("An EFunc with the same name already exists")
            }
        }
    }
}

impl std::error::Error for NeptuneError {}

pub enum EFuncErrorOr<T: ToNeptuneValue> {
    EFuncError(EFuncError),
    Other(T),
}

impl<T: Display + ToNeptuneValue> Display for EFuncErrorOr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EFuncError(e) => write!(f, "{}", e),
            Self::Other(e) => write!(f, "{}", e),
        }
    }
}

impl<T: Debug + ToNeptuneValue> Debug for EFuncErrorOr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EFuncError(e) => write!(f, "{:?}", e),
            Self::Other(e) => write!(f, "{:?}", e),
        }
    }
}

impl<T: std::error::Error + ToNeptuneValue> std::error::Error for EFuncErrorOr<T> {}

impl<T: ToNeptuneValue> ToNeptuneValue for EFuncErrorOr<T> {
    fn to_neptune_value(&self, cx: &mut EFuncContext) {
        match self {
            EFuncErrorOr::EFuncError(e) => e.to_neptune_value(cx),
            EFuncErrorOr::Other(e) => e.to_neptune_value(cx),
        }
    }
}

impl<T: ToNeptuneValue> From<EFuncError> for EFuncErrorOr<T> {
    fn from(e: EFuncError) -> Self {
        EFuncErrorOr::EFuncError(e)
    }
}

pub struct Error(String);

impl ToNeptuneValue for Error {
    fn to_neptune_value(&self, cx: &mut EFuncContext) {
        cx.error("<prelude>", "Error", &self.0).unwrap();
    }
}

struct ModuleNotFound {
    module: String,
}

impl ToNeptuneValue for ModuleNotFound {
    fn to_neptune_value(&self, cx: &mut EFuncContext) {
        cx.error(
            "<prelude>",
            "ModuleNotFoundError",
            &format!("Cannot find module {}", self.module),
        )
        .unwrap();
    }
}

pub struct Neptune {
    vm: UniquePtr<VM>,
}

pub trait ModuleLoader: Clone {
    fn resolve(&self, caller_module: &str, module: &str) -> Option<String>;
    fn load(&self, module: &str) -> Option<String>;
}

#[derive(Clone, Copy)]
struct NoopModuleLoader;

impl ModuleLoader for NoopModuleLoader {
    fn resolve(&self, _: &str, _: &str) -> Option<String> {
        None
    }

    fn load(&self, _: &str) -> Option<String> {
        None
    }
}

impl Neptune {
    pub fn new<M: ModuleLoader + 'static>(module_loader: M) -> Self {
        let n = Self { vm: new_vm() };

        n.vm.create_efunc_safe("compile", |vm, mut cx| -> bool {
            let mut eval = false;
            match || -> Result<Result<(FunctionInfoWriter, bool), Vec<CompileError>>, EFuncError> {
                cx.get_property("source")?;
                let source = cx.as_string()?.to_string();
                cx.get_property("eval")?;
                eval = cx.as_bool()?;
                cx.get_property("moduleName")?;
                let module = cx.as_string()?.to_string();
                cx.pop().unwrap();
                Ok(compile(vm, module, &source, eval))
            }() {
                Err(e) => {
                    e.to_neptune_value(&mut cx);
                    false
                }
                Ok(res) => match res {
                    Ok((fw, is_expr)) => {
                        if !is_expr && eval {
                            cx.error("<prelude>", "CompileError", "Expect expression")
                                .unwrap();
                            false
                        } else {
                            unsafe { cx.function(fw) };
                            true
                        }
                    }
                    Err(e) => {
                        let mut message = "".to_owned();
                        use std::fmt::Write;
                        for c in &e {
                            writeln!(message, "{}", c).unwrap();
                        }
                        cx.error("<prelude>", "CompileError", &message).unwrap();
                        e.to_neptune_value(&mut cx);
                        cx.set_object_property("errors").unwrap();
                        false
                    }
                },
            }
        });

        n.create_efunc("resolveModule", {
            let module_loader = module_loader.clone();
            move |cx| -> Result<String, EFuncErrorOr<ModuleNotFound>> {
                cx.get_property("callerModule")?;
                let caller_module = cx.as_string()?.to_string();
                cx.get_property("moduleName")?;
                let module_name = cx.as_string()?.to_string();
                cx.pop().unwrap();
                match module_loader.resolve(&caller_module, &module_name) {
                    Some(s) => Ok(s),
                    None => Err(EFuncErrorOr::Other(ModuleNotFound {
                        module: module_name,
                    })),
                }
            }
        })
        .unwrap();

        n.create_efunc(
            "fetchModule",
            move |cx| -> Result<String, EFuncErrorOr<Error>> {
                let module = cx.as_string()?;
                match module_loader.load(module) {
                    Some(src) => Ok(src),
                    None => Err(EFuncErrorOr::Other(Error(format!(
                        "Cannot get source of module {}",
                        module
                    )))),
                }
            },
        )
        .unwrap();

        n.exec("<prelude>", include_str!("prelude.np")).unwrap();
        n
    }

    pub fn exec<S: Into<String>>(&self, module: S, source: &str) -> Result<(), InterpretError> {
        match compile(&self.vm, module.into(), source, false) {
            Ok((mut f, _)) => match unsafe { f.run() } {
                VMStatus::Success => Ok(()),
                VMStatus::Error => Err(InterpretError::UncaughtException(self.vm.get_result())),
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
        match compile(&self.vm, module.into(), source, true) {
            Ok((mut f, is_expr)) => match unsafe { f.run() } {
                VMStatus::Success => Ok(if is_expr {
                    Some(self.vm.get_result())
                } else {
                    None
                }),
                VMStatus::Error => Err(InterpretError::UncaughtException(self.vm.get_result())),
                _ => unreachable!(),
            },
            Err(e) => Err(InterpretError::CompileError(e)),
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

    pub fn create_efunc<F, T1, T2>(&self, name: &str, mut callback: F) -> Result<(), NeptuneError>
    where
        F: FnMut(&mut EFuncContext) -> Result<T1, T2> + 'static,
        T1: ToNeptuneValue,
        T2: ToNeptuneValue,
    {
        let callback = move |_, mut cx: EFuncContext| match callback(&mut cx) {
            Ok(t1) => {
                t1.to_neptune_value(&mut cx);
                true
            }
            Err(t2) => {
                t2.to_neptune_value(&mut cx);
                false
            }
        };
        if self.vm.create_efunc_safe(name, callback) {
            Ok(())
        } else {
            Err(NeptuneError::EFuncAlreadyExists)
        }
    }
}

fn compile<'vm>(
    vm: &'vm VM,
    module: String,
    source: &str,
    eval: bool,
) -> Result<(FunctionInfoWriter<'vm>, bool), Vec<CompileError>> {
    if !vm.module_exists(module.as_str().into()) {
        vm.create_module_with_prelude(module.as_str().into());
    }
    let scanner = Scanner::new(source);
    let tokens = scanner.scan_tokens();
    let parser = Parser::new(tokens.into_iter());
    let ast = parser.parse(eval);
    let compiler = Compiler::new(vm, module);
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
    use crate::{InterpretError, Neptune, NoopModuleLoader};
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
        let n = Neptune::new(NoopModuleLoader);
        assert_eq!(n.eval("<script>", "fun f(){}").unwrap(), None);
        assert_eq!(n.eval("<script>", "\"'\"").unwrap(), Some("'\\''".into()));
        for test in ["assert_eq.np", "assert_failed.np", "assert_failed2.np"] {
            if let InterpretError::CompileError(_) = n.exec(test, &read(test)).unwrap_err() {
                panic!("Expected a runtime error")
            }
        }
        if let InterpretError::UncaughtException(e) = n.exec("<script>", "throw 'abc'").unwrap_err()
        {
            assert_eq!(e, "'abc'");
        } else {
            panic!("Expected error");
        }
        if let InterpretError::UncaughtException(e) =
            n.exec("<script>", "throw new Error('abc')").unwrap_err()
        {
            assert_eq!(e, "Error: abc\nat <script> (<script>:1)\n");
        } else {
            panic!("Expected error");
        }
        if let Err(e) = n.exec("test.np", &read("test.np")) {
            panic!("{:?}", e);
        }
        if let Err(e) = n.exec("test2.np", &read("test2.np")) {
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

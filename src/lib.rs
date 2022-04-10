use crate::vm::VMStatus;
use compiler::Compiler;
use cxx::UniquePtr;
use futures::stream::FuturesUnordered;
use futures::Future;
use futures::StreamExt;
use parser::Parser;
use scanner::Scanner;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use vm::free_data;
use vm::Data;
use vm::FreeDataCallback;
use vm::UserData;
use vm::{new_vm, FunctionInfoWriter, VM};
pub use vm::{EFuncContext, EFuncError, ToNeptuneValue};
mod compiler;
mod parser;
mod scanner;
mod vm;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompileError {
    pub message: String,
    pub line: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompileErrorList {
    pub module: String,
    pub errors: Vec<CompileError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum InterpretError {
    CompileError(CompileErrorList),
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
                writeln!(f, "In module {}", c.module)?;
                for error in &c.errors {
                    writeln!(f, "{}", error)?;
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

pub(crate) type CompileResult<T> = Result<T, CompileError>;

#[derive(Debug)]
pub enum Error {
    FunctionAlreadyExists,
    ModuleNotFound,
    ModuleAlreadyExists,
    EFuncAlreadyExists,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::FunctionAlreadyExists => {
                f.write_str("A function with the same name already exists")
            }
            Error::ModuleNotFound => f.write_str("The module cannot be found"),
            Error::ModuleAlreadyExists => f.write_str("A module with the same name already exists"),
            Error::EFuncAlreadyExists => f.write_str("An EFunc with the same name already exists"),
        }
    }
}

impl std::error::Error for Error {}

/// This enum can be used to represent errors that can either be an EFuncError or another error type.
/// `EFuncError`s can be converted to EFuncErrorOr using the ? operator.
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

/// This type represents the `Error` class of Neptune lang.
pub struct NeptuneError(pub String);

impl ToNeptuneValue for NeptuneError {
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

/// Instance of a Neptune VM
pub struct Neptune {
    vm: UniquePtr<VM>,
}

/// The embedder needs to implement this trait to specify how to resolve import paths
pub trait ModuleLoader: Clone {
    /// Returns the name of the module where
    /// * `caller_module` is the module calling import
    /// * `module` is  the argument passed to `import`
    fn resolve(&self, caller_module: &str, module: &str) -> Option<String>;
    /// Returns the source of the module
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
        let n = Self {
            vm: unsafe {
                new_vm(
                    Box::into_raw(Box::new(UserData {
                        futures: RefCell::new(FuturesUnordered::new()),
                        resources: RefCell::new(HashMap::new()),
                    })) as *mut Data,
                    free_data::<UserData> as *mut FreeDataCallback,
                )
            },
        };

        n.vm.create_efunc_safe("compile", |mut cx| -> bool {
            let mut eval = false;
            let vm = cx.vm();
            let mut module = None;
            match || -> Result<Result<(FunctionInfoWriter, bool), Vec<CompileError>>, EFuncError> {
                cx.get_property("source")?;
                let source = cx.as_string()?.to_string();
                cx.get_property("eval")?;
                eval = cx.as_bool()?;
                cx.get_property("moduleName")?;
                module = Some(cx.as_string()?.to_string());
                cx.pop().unwrap();
                Ok(compile(vm, module.as_ref().unwrap().clone(), &source, eval))
            }() {
                Err(e) => {
                    e.to_neptune_value(&mut cx);
                    false
                }
                Ok(res) => match res {
                    Ok((fw, is_expr)) => {
                        if eval {
                            cx.object();
                            unsafe { cx.function(fw) };
                            cx.set_object_property("function").unwrap();
                            cx.bool(is_expr);
                            cx.set_object_property("isExpr").unwrap();
                            true
                        } else {
                            unsafe { cx.function(fw) };
                            true
                        }
                    }
                    Err(errors) => {
                        CompileErrorList {
                            module: module.unwrap(),
                            errors,
                        }
                        .to_neptune_value(&mut cx);
                        false
                    }
                },
            }
        });

        n.create_efunc("close", |cx| -> Result<(), EFuncErrorOr<NeptuneError>> {
            let rid = cx.as_int()?;
            if rid < 0 {
                Err(EFuncErrorOr::Other(NeptuneError(
                    "Resource id cannot be negative".into(),
                )))
            } else if cx.resources().borrow_mut().remove(&(rid as u32)).is_none() {
                Err(EFuncErrorOr::Other(NeptuneError(format!(
                    "Resource with id {} does not exist",
                    rid
                ))))
            } else {
                Ok(())
            }
        })
        .unwrap();

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
            move |cx| -> Result<String, EFuncErrorOr<NeptuneError>> {
                let module = cx.as_string()?;
                match module_loader.load(module) {
                    Some(src) => Ok(src),
                    None => Err(EFuncErrorOr::Other(NeptuneError(format!(
                        "Cannot get source of module {}",
                        module
                    )))),
                }
            },
        )
        .unwrap();

        n.exec_sync("<prelude>", include_str!("prelude.np"))
            .unwrap();
        n
    }

    /// Executes source with module `module`
    pub async fn exec<S: Into<String>>(
        &self,
        module: S,
        source: &str,
    ) -> Result<(), InterpretError> {
        let module = module.into();
        match compile(&self.vm, module.clone(), source, false) {
            Ok((mut f, _)) => {
                let mut result = unsafe { f.run() };
                loop {
                    match result {
                        VMStatus::Success => return Ok(()),
                        VMStatus::Error => {
                            return Err(InterpretError::UncaughtException(self.vm.get_result()))
                        }
                        VMStatus::Suspend => {
                            if self.vm.get_user_data().futures.borrow().is_empty() {
                                return Err(InterpretError::UncaughtException(
                                    self.vm.kill_main_task(
                                        "DeadlockError".into(),
                                        "All tasks were asleep".into(),
                                    ),
                                ));
                            } else {
                                let (closure, mut task) = self
                                    .vm
                                    .get_user_data()
                                    .futures
                                    .borrow_mut()
                                    .next()
                                    .await
                                    .unwrap();
                                result = task.resume_safe(closure);
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(errors) => Err(InterpretError::CompileError(CompileErrorList {
                errors,
                module,
            })),
        }
    }

    /// Executes source with module `module`.
    /// It panics if a asynchronous efunc is executed
    pub fn exec_sync<S: Into<String>>(
        &self,
        module: S,
        source: &str,
    ) -> Result<(), InterpretError> {
        let module = module.into();
        match compile(&self.vm, module.clone(), source, false) {
            Ok((mut f, _)) => match unsafe { f.run() } {
                VMStatus::Success => Ok(()),
                VMStatus::Error => Err(InterpretError::UncaughtException(self.vm.get_result())),
                VMStatus::Suspend => {
                    if self.vm.get_user_data().futures.borrow().is_empty() {
                        Err(InterpretError::UncaughtException(self.vm.kill_main_task(
                            "DeadlockError".into(),
                            "All tasks were asleep".into(),
                        )))
                    } else {
                        panic!("Waiting on future in exec_sync");
                    }
                }
                _ => unreachable!(),
            },
            Err(errors) => Err(InterpretError::CompileError(CompileErrorList {
                errors,
                module,
            })),
        }
    }

    /// Creates a module named  `name`.
    /// It returns `Err(ModuleAlreadyExists)` if an existing module is named `name`
    pub fn create_module(&self, name: &str) -> Result<(), Error> {
        if self.vm.module_exists(name.into()) {
            Err(Error::ModuleAlreadyExists)
        } else {
            self.vm.create_module(name.into());
            Ok(())
        }
    }

    /// Creates an synchronous efunc.
    /// Returns Err(EFuncAlreadyExists) if an existing efunc is named `name`
    pub fn create_efunc<F, T1, T2>(&self, name: &str, mut callback: F) -> Result<(), Error>
    where
        F: FnMut(&mut EFuncContext) -> Result<T1, T2> + 'static,
        T1: ToNeptuneValue,
        T2: ToNeptuneValue,
    {
        let callback = move |mut cx: EFuncContext| match callback(&mut cx) {
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
            Err(Error::EFuncAlreadyExists)
        }
    }

    /// Creates an asynchronous efunc.
    /// Returns Err(EFuncAlreadyExists) if an existing efunc is named `name`
    pub fn create_efunc_async<F, Fut, T1, T2>(&self, name: &str, callback: F) -> Result<(), Error>
    where
        F: (FnMut(&mut EFuncContext) -> Fut) + 'static,
        Fut: Future<Output = Result<T1, T2>> + 'static,
        T1: ToNeptuneValue + 'static,
        T2: ToNeptuneValue + 'static,
    {
        if self.vm.create_efunc_async(name, callback) {
            Ok(())
        } else {
            Err(Error::EFuncAlreadyExists)
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
    use crate::{
        EFuncError, EFuncErrorOr, InterpretError, ModuleLoader, Neptune, NeptuneError,
        ToNeptuneValue,
    };
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
    fn read(file: &str) -> std::io::Result<String> {
        let path = open(file);
        let mut s = String::new();
        File::open(path).unwrap().read_to_string(&mut s)?;
        Ok(s)
    }

    #[derive(Clone, Copy)]
    struct TestModuleLoader;
    impl ModuleLoader for TestModuleLoader {
        fn resolve(&self, _: &str, module: &str) -> Option<String> {
            Some(module.into())
        }

        fn load(&self, module: &str) -> Option<String> {
            read(module).ok()
        }
    }

    #[test]
    fn test_basic() {
        let n = Neptune::new(TestModuleLoader);
        n.create_efunc("test_str", |cx| -> Result<(), ()> {
            let s = cx.as_string().unwrap();
            assert_eq!(s, "\n\r\t\0\'\"");
            Ok(())
        })
        .unwrap();
        n.exec_sync(
            "<script>",
            r#"
        const {ecall} = import('vm')
        ecall(@test_str,'\n\r\t\0\'\"')
        "#,
        )
        .unwrap();
    }

    #[test]
    fn test_runtime() {
        let n = Neptune::new(TestModuleLoader);
        if let InterpretError::UncaughtException(e) =
            n.exec_sync("<script>", "throw 'abc'").unwrap_err()
        {
            assert_eq!(e, "'abc'");
        } else {
            panic!("Expected error");
        }
        if let InterpretError::UncaughtException(e) = n
            .exec_sync("<script>", "throw new Error('abc')")
            .unwrap_err()
        {
            assert_eq!(e, "In <Task> Error: abc\nat <script> (<script>:1)");
        } else {
            panic!("Expected error");
        }
        for test in [
            "test.np",
            "test_lines.np",
            "test_many_registers_constants.np",
            "test_jumps.np",
        ] {
            if let Err(e) = n.exec_sync(test, &read(test).unwrap()) {
                panic!("Error in file {}, {:?}", test, e);
            }
        }
        if let InterpretError::UncaughtException(e) = n
            .exec_sync("test_deadlock.np", &read("test_deadlock.np").unwrap())
            .unwrap_err()
        {
            assert_eq!(
                e,
                "In <Task> DeadlockError: All tasks were asleep\nat <script> (test_deadlock.np:7)"
            )
        } else {
            panic!("Expected error")
        }
        n.exec_sync("test_deadlock.np", &read("test_deadlock_post.np").unwrap())
            .unwrap();
        if let InterpretError::UncaughtException(s) = n
            .exec_sync(
                "test_kill_main_task.np",
                &read("test_kill_main_task.np").unwrap(),
            )
            .unwrap_err()
        {
            assert_eq!(
                s,
                "In <Task> Error: main task killed\nat <closure> (test_kill_main_task.np:3)"
            );
        } else {
            panic!("Expected UncaughtException");
        }
        n.exec_sync(
            "test_kill_main_task.np",
            &read("test_kill_main_task_post.np").unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_errors() {
        let n = Neptune::new(TestModuleLoader);
        let errors: Vec<String> = serde_json::from_str(&read("errors.json").unwrap()).unwrap();
        for error in errors {
            let fname = format!("{}.np", error);
            let source = read(&fname).unwrap();
            let res = n.exec_sync(fname, &source).unwrap_err();
            let result = serde_json::to_string_pretty(&res).unwrap();
            if env::var("NEPTUNE_GEN_ERRORS").is_ok() {
                File::create(open(&format!("{}.json", error)))
                    .unwrap()
                    .write_all(result.as_bytes())
                    .unwrap();
            } else {
                let mut expected_result = String::new();
                println!("{}", error);
                File::open(open(&format!("{}.json", error)))
                    .unwrap()
                    .read_to_string(&mut expected_result)
                    .unwrap();
                assert_eq!(expected_result, result);
            }
        }
    }

    struct FirstRest {
        first: f64,
        rest: Vec<f64>,
    }

    enum Bar {
        Baz,
        Ja,
    }

    impl ToNeptuneValue for Bar {
        fn to_neptune_value(&self, cx: &mut crate::EFuncContext) {
            match self {
                Bar::Baz => cx.symbol("baz"),
                Bar::Ja => cx.symbol("ja"),
            }
        }
    }

    struct Foo {
        a: bool,
        d: Vec<(i32, i32)>,
        b: String,
    }

    impl ToNeptuneValue for Foo {
        fn to_neptune_value(&self, cx: &mut crate::EFuncContext) {
            cx.array();
            cx.bool(self.a);
            cx.push_to_array().unwrap();
            cx.map();
            for (a, b) in &self.d {
                cx.int(*a);
                cx.int(*b);
                cx.insert_in_map().unwrap();
            }
            cx.push_to_array().unwrap();
            cx.string(&self.b);
            cx.push_to_array().unwrap();
        }
    }

    impl ToNeptuneValue for FirstRest {
        fn to_neptune_value(&self, cx: &mut crate::EFuncContext) {
            cx.object();
            self.first.to_neptune_value(cx);
            cx.set_object_property("first").unwrap();
            self.rest.to_neptune_value(cx);
            cx.set_object_property("rest").unwrap();
        }
    }

    #[test]
    fn test_efunc() {
        let n = Neptune::new(TestModuleLoader);
        n.create_efunc("add", |ctx| -> Result<i32, EFuncError> {
            ctx.get_property("a")?;
            let i1 = ctx.as_int()?;
            ctx.get_property("b")?;
            let i2 = ctx.as_int()?;
            ctx.pop().unwrap();
            Ok(i1 + i2)
        })
        .unwrap();
        n.create_efunc("firstRest", |ctx| -> Result<FirstRest, EFuncError> {
            let len = ctx.array_length()?;
            ctx.get_element(0)?;
            let first = ctx.as_float()?;
            let mut rest = vec![];
            for i in 1..len {
                ctx.get_element(i)?;
                rest.push(ctx.as_float()?);
            }
            ctx.pop().unwrap();
            Ok(FirstRest { first, rest })
        })
        .unwrap();
        n.create_efunc(
            "test_sym",
            |ctx| -> Result<Bar, EFuncErrorOr<NeptuneError>> {
                match ctx.as_symbol()? {
                    "abc" => Ok(Bar::Baz),
                    "def" => Ok(Bar::Ja),
                    _ => Err(EFuncErrorOr::Other(NeptuneError("invalid!!!".into()))),
                }
            },
        )
        .unwrap();
        //test capturing too!
        let d = vec![(1, 2), (3, 4)];
        n.create_efunc("foo", move |ctx| -> Result<Foo, EFuncError> {
            ctx.get_element(0)?;
            let b = ctx.as_string()?.to_string();
            ctx.get_element(1)?;
            let a = ctx.as_bool()?;
            ctx.pop().unwrap();
            Ok(Foo { a, b, d: d.clone() })
        })
        .unwrap();
        n.create_efunc("test_null", move |ctx| -> Result<(), NeptuneError> {
            if ctx.is_null().unwrap() {
                Ok(())
            } else {
                Err(NeptuneError("Not null!!!".into()))
            }
        })
        .unwrap();
        if let Err(e) = n.exec_sync("test_efunc.np", &read("test_efunc.np").unwrap()) {
            panic!("{:?}", e);
        }
    }
}

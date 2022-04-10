use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tokio::time::{sleep, Duration};

use neptune_lang::{EFuncError, ModuleLoader, Neptune, ToNeptuneValue};
use rustyline::{
    validate::{self, Validator},
    Editor,
};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter};

fn main() {
    let n = Neptune::new(FileSystemModuleLoader);
    n.create_efunc("print", |cx| -> Result<(), EFuncError> {
        println!("{}", cx.as_string()?);
        Ok(())
    })
    .unwrap();
    n.exec_sync("<prelude>", include_str!("prelude.np"))
        .unwrap();

    n.create_efunc("timeNow", |_| -> Result<f64, ()> {
        Ok(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64())
    })
    .unwrap();
    n.create_efunc_async("sleep", |cx| {
        let time = cx.as_int();
        async move {
            sleep(Duration::from_millis(time? as u64)).await;
            Result::<(), EFuncError>::Ok(())
        }
    })
    .unwrap();
    n.exec_sync("time", include_str!("time.np")).unwrap();

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async move {
        match std::env::args().nth(1) {
            Some(file) => match &std::fs::read_to_string(&file) {
                Ok(s) => match n
                    .exec(
                        std::fs::canonicalize(&file)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        s,
                    )
                    .await
                {
                    Ok(()) => {}
                    Err(e) => eprintln!("{}", e),
                },
                Err(e) => {
                    eprintln!("{}", e);
                }
            },
            None => repl(&n).await,
        }
    });
}

#[derive(Clone, Copy)]
struct FileSystemModuleLoader;

impl ModuleLoader for FileSystemModuleLoader {
    fn resolve(&self, caller_module: &str, module: &str) -> Option<String> {
        let current_path: PathBuf;
        if caller_module == "<repl>" {
            if let Ok(dir) = std::env::current_dir() {
                current_path = dir;
            } else {
                return None;
            }
        } else {
            let caller_path = Path::new(caller_module);
            current_path = caller_path.parent().unwrap().into();
        };
        let module_path = current_path.join(module);
        match std::fs::canonicalize(module_path) {
            Ok(path) => Some(path.to_string_lossy().into_owned()),
            Err(_) => None,
        }
    }

    fn load(&self, module: &str) -> Option<String> {
        std::fs::read_to_string(module).ok()
    }
}

#[derive(Helper, Hinter, Highlighter, Completer)]
struct ReplValidator;

impl Validator for ReplValidator {
    fn validate(
        &self,
        cx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        Ok(match are_brackets_balanced(cx.input()) {
            true => validate::ValidationResult::Valid(None),
            false => validate::ValidationResult::Incomplete,
        })
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}

async fn repl(n: &Neptune) {
    let mut rl = Editor::new();
    rl.set_helper(Some(ReplValidator {}));
    let histfile = dirs::cache_dir().map(|mut dir| {
        dir.push("neptune_repl_history");
        let _ = rl.load_history(&dir);
        dir
    });
    println!("Welcome to the Neptune Programming Language!");
    let rl = Arc::new(Mutex::new(rl));
    n.create_efunc_async("replReadline", {
        let rl = rl.clone();
        move |_| {
            let rl = rl.clone();
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut rl = rl.lock().unwrap();
                    match rl.readline(">> ") {
                        Ok(s) => {
                            rl.add_history_entry(&s);
                            Ok(s)
                        }
                        Err(e) => Err(ReadlineError(e)),
                    }
                })
                .await
                .unwrap()
            }
        }
    })
    .unwrap();
    n.exec("<repl>", include_str!("repl.np")).await.unwrap();
    if let Some(file) = histfile {
        if let Err(e) = rl.lock().unwrap().save_history(&file) {
            eprintln!("Error in saving REPL history: {}", e)
        }
    }
}

struct ReadlineError(rustyline::error::ReadlineError);

impl ToNeptuneValue for ReadlineError {
    fn to_neptune_value(&self, cx: &mut neptune_lang::EFuncContext) {
        match &self.0 {
            rustyline::error::ReadlineError::Eof => cx.symbol("eof"),
            rustyline::error::ReadlineError::Interrupted => cx.symbol("interrupted"),
            rustyline::error::ReadlineError::Utf8Error => cx.symbol("utf8"),
            e => panic!("{}", e),
        }
    }
}

// Checks whether brackets are balanced
pub fn are_brackets_balanced(s: &str) -> bool {
    fn string(
        index: &mut usize,
        s: &str,
        delim: u8,
        depths: &mut Vec<u32>,
        delims: &mut Vec<u8>,
    ) -> bool {
        *index += 1; //Consume start of string
        while *index < s.len() {
            match s.as_bytes()[*index] {
                b'\\' => {
                    // is \ the last character
                    if *index + 1 >= s.len() {
                        return false;
                    } else {
                        *index += 1;
                        if s.as_bytes()[*index] == b'(' {
                            *index -= 1;
                            depths.push(0);
                            break;
                        }
                    }
                }
                x if x == delim => {
                    delims.pop();
                    break;
                } //Break;Do not consume end of string as index+=1 is done below
                _ => {}
            }
            *index += 1
        }
        *index != s.len()
    }

    let mut depths = vec![0u32];
    let mut delims = vec![];
    let mut index = 0;
    while index < s.len() {
        match s.as_bytes()[index] {
            b'(' => *depths.last_mut().unwrap() += 1,
            b')' => {
                if *depths.last_mut().unwrap() == 0 {
                    return true;
                }
                *depths.last_mut().unwrap() -= 1;
                if *depths.last().unwrap() == 0 && depths.len() > 1 {
                    depths.pop();
                    if !string(
                        &mut index,
                        s,
                        *delims.last().unwrap(),
                        &mut depths,
                        &mut delims,
                    ) {
                        return false;
                    }
                }
            }
            b'{' => *depths.last_mut().unwrap() += 1,
            b'}' => {
                if *depths.last_mut().unwrap() == 0 {
                    return true;
                }
                *depths.last_mut().unwrap() -= 1
            }
            b'[' => *depths.last_mut().unwrap() += 1,
            b']' => {
                if *depths.last_mut().unwrap() == 0 {
                    return true;
                }
                *depths.last_mut().unwrap() -= 1
            }
            c @ b'"' | c @ b'\'' => {
                delims.push(c);
                if !string(&mut index, s, c, &mut depths, &mut delims) {
                    return false;
                }
            }
            _ => {}
        }
        index += 1;
    }
    depths.len() == 1 && depths[0] == 0
}

#[cfg(test)]
mod tests {
    use super::are_brackets_balanced;
    use crate::FileSystemModuleLoader;
    use neptune_lang::Neptune;

    #[test]
    fn test_brackets_balanced() {
        for s in ["'\"", "(", "'\\({)'", "'\\", "'srcbc", "'\\u", "\"\\()\\\""] {
            assert!(!are_brackets_balanced(s), "{}", s)
        }
        for s in ["''", "()", "({})", "'\\('(')'", "'\\({})'", "a", "[]"] {
            assert!(are_brackets_balanced(s), "{}", s)
        }
    }

    #[test]
    fn test_import() {
        let n = Neptune::new(FileSystemModuleLoader);
        n.exec_sync(
            concat!(env!("CARGO_MANIFEST_DIR"), "/test/test_import1.np"),
            include_str!("../test/test_import1.np"),
        )
        .unwrap();
    }
}

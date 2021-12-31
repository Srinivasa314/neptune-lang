use std::path::{Path, PathBuf};

use neptune_lang::{ModuleLoader, Neptune};
use rustyline::{
    error::ReadlineError,
    validate::{self, Validator},
    Editor,
};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter};

fn main() {
    let n = Neptune::new(FileSystemModuleLoader);
    n.create_efunc("print", |cx| -> Result<(), ()> {
        println!("{}", cx.as_string().unwrap());
        Ok(())
    })
    .unwrap();
    n.exec("<prelude>", include_str!("prelude.np")).unwrap();

    match std::env::args().nth(1) {
        Some(file) => match &std::fs::read_to_string(&file) {
            Ok(s) => match n.exec(
                std::fs::canonicalize(&file)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
                s,
            ) {
                Ok(()) => {}
                Err(e) => eprintln!("{}", e),
            },
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        },
        None => repl(&n),
    }
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

fn repl(n: &Neptune) {
    let mut rl = Editor::new();
    rl.set_helper(Some(ReplValidator {}));
    let histfile = dirs::cache_dir().map(|mut dir| {
        dir.push("neptune_repl_history");
        let _ = rl.load_history(&dir);
        dir
    });
    println!("Welcome to the Neptune Programming Language!");
    loop {
        match rl.readline(">> ") {
            Ok(lines) => {
                match n.eval("<repl>", &lines) {
                    Ok(Some(val)) => {
                        println!("{}", val);
                    }
                    Ok(None) => {}
                    Err(e) => eprintln!("{}", e),
                };
                rl.add_history_entry(lines);
            }
            Err(e) => match e {
                ReadlineError::Eof => break,
                ReadlineError::Interrupted => {}
                e => panic!("{}", e),
            },
        }
    }
    if let Some(file) = histfile {
        if let Err(e) = rl.save_history(&file) {
            eprintln!("Error in saving REPL history: {}", e)
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
                        &mut &mut delims,
                    ) {
                        return false;
                    }
                }
            }
            b'{' => *depths.last_mut().unwrap() += 1,
            b'}' => *depths.last_mut().unwrap() -= 1,
            b'[' => *depths.last_mut().unwrap() += 1,
            b']' => *depths.last_mut().unwrap() -= 1,
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

    #[test]
    fn test_brackets_balanced() {
        for s in ["'\"", "(", "'\\({)'", "'\\", "'srcbc", "'\\u", "\"\\()\\\""] {
            assert!(!are_brackets_balanced(s), "{}", s)
        }
        for s in ["''", "()", "({})", "'\\('(')'", "'\\({})'", "a", "[]"] {
            assert!(are_brackets_balanced(s), "{}", s)
        }
    }
}

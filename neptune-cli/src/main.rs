use neptune_lang::{InterpretError, Neptune};
use rustyline::{
    error::ReadlineError,
    validate::{self, Validator},
    Editor,
};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter};

fn main() -> Result<(), std::io::Error> {
    let mut n = Neptune::new();
    match std::env::args().nth(1) {
        Some(file) => match n.exec(&std::fs::read_to_string(file)?) {
            Ok(()) => {}
            Err(InterpretError::CompileError(c)) => {
                for error in c {
                    eprintln!("line {}: {}", error.line, error.message)
                }
            }
            Err(InterpretError::RuntimePanic(r)) => {
                eprintln!("Runtime Error: {}", r)
            }
        },
        None => repl(&mut n),
    }
    Ok(())
}

#[derive(Helper, Hinter, Highlighter, Completer)]
struct ReplValidator;

impl Validator for ReplValidator {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        Ok(match are_brackets_balanced(ctx.input()) {
            true => validate::ValidationResult::Valid(None),
            false => validate::ValidationResult::Incomplete,
        })
    }

    fn validate_while_typing(&self) -> bool {
        false
    }
}

fn repl(n: &mut Neptune) {
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
                match n.eval(&lines) {
                    Ok(Some(val)) => {
                        println!("{}", val);
                    }
                    Ok(None) => {}
                    Err(InterpretError::CompileError(c)) => {
                        for error in c {
                            eprintln!("line {}: {}", error.line, error.message)
                        }
                    }
                    Err(InterpretError::RuntimePanic(r)) => {
                        eprintln!("Runtime Error: {}", r)
                    }
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

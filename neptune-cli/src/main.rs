use neptune_lang::{InterpretError, Neptune};
use rustyline::{
    error::ReadlineError,
    validate::{self, Validator},
    Editor,
};
use rustyline_derive::{Completer, Helper, Highlighter, Hinter};

fn main() {
    let n = Neptune::new();
    n.create_function("prelude", "print", 1, 0, |ctx| {
        ctx.to_string(0, 0);
        println!("{}", ctx.as_string(0).unwrap());
        ctx.null(0);
        Ok(0)
    })
    .unwrap();
    match std::env::args().nth(1) {
        Some(file) => match &std::fs::read_to_string(&file) {
            Ok(s) => match n.exec(&file, s) {
                Ok(()) => {}
                Err(e) => report_error(e),
            },
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        },
        None => repl(&n),
    }
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
                match n.eval("<stdin>", &lines) {
                    Ok(Some(val)) => {
                        println!("{}", val);
                    }
                    Ok(None) => {}
                    Err(e) => report_error(e),
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

fn report_error(i: InterpretError) {
    match i {
        InterpretError::CompileError(c) => {
            for error in c {
                eprintln!("line {}: {}", error.line, error.message)
            }
        }
        InterpretError::RuntimePanic { error, stack_trace } => {
            eprintln!("Runtime Error: {}", error);
            eprintln!("{}", stack_trace);
        }
    }
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

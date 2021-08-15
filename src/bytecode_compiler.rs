use cxx::Exception;
use cxx::UniquePtr;

use crate::parser::Statement;
use crate::parser::Substring;
use crate::vm::new_function_info;
use crate::vm::FunctionInfo;
use crate::vm::Op;
use crate::vm::VM;
use crate::CompileError;
use crate::CompileResult;
use crate::{parser::Expr, scanner::TokenType};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::hash::Hash;
use std::ops::{Add, Div, Mul, Sub};
use std::pin::Pin;
use std::vec;

pub struct Compiler<'vm> {
    globals: HashMap<String, u32>,
    errors: Vec<CompileError>,
    vm: &'vm VM,
}

impl<'vm> Compiler<'vm> {
    pub fn new(vm: &'vm VM) -> Self {
        Self {
            vm,
            globals: HashMap::default(),
            errors: vec![],
        }
    }
}

struct BytecodeCompiler<'c, 'vm> {
    compiler: Option<&'c mut Compiler<'vm>>,
    bytecode: UniquePtr<FunctionInfo>,
    op_positions: Vec<usize>,
    locals: Vec<String>,
    depth: u32,
    regcount: u16,
    max_registers: u16,
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn new(c: &'c mut Compiler<'vm>) -> Self {
        Self {
            bytecode: new_function_info(),
            locals: vec![],
            regcount: 0,
            depth: 0,
            compiler: Some(c),
            max_registers: 0,
            op_positions: vec![],
        }
    }

    fn push_register(&mut self) -> Option<u16> {
        if self.regcount == u16::MAX {
            None
        } else {
            self.regcount += 1;
            if self.regcount > self.max_registers {
                self.max_registers = self.regcount;
            }
            Some(self.regcount - 1)
        }
    }

    fn pop_register(&mut self) {
        self.regcount -= 1;
    }

    fn get_global(&self, name: &str) -> Option<u32> {
        self.compiler.as_ref().unwrap().globals.get(name).cloned()
    }

    fn new_global(&mut self, name: &str) -> u32 {
        let comp = &mut self.compiler.as_mut().unwrap();
        let len = comp.globals.len() as u32;
        comp.globals.insert(name.to_string(), len);
        comp.vm.add_global(name.into());
        len
    }

    fn error(&mut self, e: CompileError) {
        self.compiler.as_mut().unwrap().errors.push(e)
    }

    fn bc(&mut self) -> Pin<&mut FunctionInfo> {
        self.bytecode.as_mut().unwrap()
    }

    fn vm<'a>(&'a self) -> &'vm VM {
        self.compiler.as_ref().unwrap().vm
    }

    fn write_op(&mut self, op: Op, line: u32) {
        let pos = self.bc().write_op(op, line);
        self.op_positions.push(pos);
    }

    fn pop_last_op(&mut self) {
        let pos = self.op_positions.pop().unwrap();
        self.bc().pop_last_op(pos);
    }
}

macro_rules! binary_op_int {
    ($fn_name:ident,$inst_name:ident) => {
        fn $fn_name(&mut self, i: i32, line: u32) {
            if let Ok(i) = i8::try_from(i) {
                self.write_op(Op::$inst_name, line);
                self.bc().write_i8(i);
            } else if let Ok(i) = i16::try_from(i) {
                self.write_op(Op::Wide, line);
                self.write_op(Op::$inst_name, line);
                self.bc().write_i16(i);
            } else {
                self.write_op(Op::ExtraWide, line);
                self.write_op(Op::$inst_name, line);
                self.bc().write_i32(i);
            }
        }
    };
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn write_op_u16arg(&mut self, op: Op, arg: u16, line: u32) {
        if let Ok(arg) = u8::try_from(arg) {
            self.write_op(op, line);
            self.bc().write_u8(arg)
        } else {
            self.write_op(Op::Wide, line);
            self.write_op(op, line);
            self.bc().write_u16(arg)
        }
    }

    fn write_op_store_register(&mut self, reg: u16, line: u32) {
        match reg {
            0..=15 => {
                self.bc().write_op(
                    Op {
                        repr: (Op::StoreR0.repr + reg as u8),
                    },
                    line,
                );
            }
            16..=255 => {
                self.write_op(Op::StoreRegister, line);
                self.bc().write_u8(reg as u8);
            }
            _ => {
                self.write_op(Op::Wide, line);
                self.write_op(Op::StoreRegister, line);
                self.bc().write_u16(reg);
            }
        }
    }

    fn write_op_load_register(&mut self, reg: u16, line: u32) {
        match reg {
            0..=15 => {
                self.bc().write_op(
                    Op {
                        repr: (Op::LoadR0.repr + reg as u8),
                    },
                    line,
                );
            }
            16..=255 => {
                self.write_op(Op::LoadRegister, line);
                self.bc().write_u8(reg as u8);
            }
            _ => {
                self.write_op(Op::Wide, line);
                self.write_op(Op::LoadRegister, line);
                self.bc().write_u16(reg);
            }
        }
    }

    binary_op_int!(write_op_load_int, LoadInt);

    fn write_op_move(&mut self, reg1: u16, reg2: u16, line: u32) {
        match (u8::try_from(reg1), u8::try_from(reg2)) {
            (Ok(reg1), Ok(reg2)) => {
                self.write_op(Op::Move, line);
                self.bc().write_u8(reg1);
                self.bc().write_u8(reg2)
            }
            _ => {
                self.write_op(Op::Wide, line);
                self.write_op(Op::Move, line);
                self.bc().write_u16(reg1);
                self.bc().write_u16(reg2)
            }
        }
    }

    fn write_op_store_global(&mut self, global: u32, line: u32) {
        if let Ok(global) = u8::try_from(global) {
            self.write_op(Op::StoreGlobal, line);
            self.bc().write_u8(global);
        } else if let Ok(global) = u16::try_from(global) {
            self.write_op(Op::Wide, line);
            self.write_op(Op::StoreGlobal, line);
            self.bc().write_u16(global);
        } else {
            self.write_op(Op::ExtraWide, line);
            self.write_op(Op::StoreGlobal, line);
            self.bc().write_u32(global);
        }
    }

    fn write_op_load_global(&mut self, global: u32, line: u32) {
        if let Ok(global) = u8::try_from(global) {
            self.write_op(Op::LoadGlobal, line);
            self.bc().write_u8(global);
        } else if let Ok(global) = u16::try_from(global) {
            self.write_op(Op::Wide, line);
            self.write_op(Op::LoadGlobal, line);
            self.bc().write_u16(global);
        } else {
            self.write_op(Op::ExtraWide, line);
            self.write_op(Op::LoadGlobal, line);
            self.bc().write_u32(global);
        }
    }

    fn load_const(&mut self, constant: Result<u16, Exception>, line: u32) -> CompileResult<()> {
        self.write_op_u16arg(
            Op::LoadConstant,
            constant.map_err(|e| CompileError {
                line,
                message: "Cannot have more than 65535 constants per function".into(),
            })?,
            line,
        );
        Ok(())
    }

    binary_op_int!(write_op_add_int, AddInt);

    binary_op_int!(write_op_subtract_int, SubtractInt);

    binary_op_int!(write_op_multiply_int, MultiplyInt);

    binary_op_int!(write_op_divide_int, DivideInt);
}

#[derive(Debug, Clone, Copy)]
enum ExprResult {
    Register(u16),
    Accumulator,
    Int(i32),
    Float(f64),
}

macro_rules! binary_op {
    ($op:ident,$register_inst:ident,$int_inst:ident,$op_fn:ident,$op_checked_fn:ident) => {
        fn $op(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
            let left = self.evaluate_expr(left)?;
            let mut reg = 0;
            if !matches!(left, ExprResult::Register(_)) {
                reg = self.push_register().ok_or(CompileError {
                    message: "Cannot have more than 65535 locals+expressions".into(),
                    line,
                })?;
                self.store_in_accumulator(left, line)?;
                self.write_op_store_register(reg, line);
            }
            let right = self.evaluate_expr(right)?;
            if !matches!(left, ExprResult::Register(_)) {
                self.pop_register();
            }
            match (left, right) {
                (ExprResult::Int(i1), ExprResult::Int(i2)) => {
                    self.undo_save_to_register(ExprResult::Int(i1));
                    Ok(ExprResult::Int(i1.$op_checked_fn(i2).ok_or(
                        CompileError {
                            message: format!(
                                "Cannot {} {} and {} as the result cannot be stored in an int",
                                stringify!($op_fn),
                                i1,
                                i2
                            ),
                            line,
                        },
                    )?))
                }
                (ExprResult::Float(f), ExprResult::Int(i)) => {
                    self.undo_save_to_register(ExprResult::Float(f));
                    Ok(ExprResult::Float(f.$op_fn(f64::from(i))))
                }
                (ExprResult::Int(i), ExprResult::Float(f)) => {
                    self.undo_save_to_register(ExprResult::Int(i));
                    Ok(ExprResult::Float(f64::from(i).$op_fn(f)))
                }
                (left, ExprResult::Int(i)) => {
                    self.undo_save_to_register(left);
                    self.store_in_accumulator(left, line)?;
                    self.$int_inst(i, line);
                    Ok(ExprResult::Accumulator)
                }
                (ExprResult::Register(r), right) => {
                    self.store_in_accumulator(right, line)?;
                    self.write_op_u16arg(Op::$register_inst, r, line);
                    Ok(ExprResult::Accumulator)
                }

                (_, right) => {
                    self.store_in_accumulator(right, line)?;
                    self.write_op_u16arg(Op::$register_inst, reg, line);
                    Ok(ExprResult::Accumulator)
                }
            }
        }
    };
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn undo_save_to_register(&mut self, result: ExprResult) {
        match result {
            ExprResult::Register(_) => {
                self.pop_last_op();
            }
            ExprResult::Accumulator => {}
            ExprResult::Int(_) => {
                self.pop_last_op();
            }
            ExprResult::Float(_) => {
                self.pop_last_op();
            }
        }
        self.pop_last_op();
    }

    fn store_in_accumulator(&mut self, result: ExprResult, line: u32) -> CompileResult<()> {
        match result {
            ExprResult::Register(reg) => Ok(self.write_op_load_register(reg, line)),
            ExprResult::Accumulator => Ok(()),
            ExprResult::Int(i) => Ok(self.write_op_load_int(i, line)),
            ExprResult::Float(f) => {
                let c = self.bc().float_constant(f);
                self.load_const(c, line)
            }
        }
    }

    fn resolve_local(&self, name: &str) -> Option<u16> {
        for (index, local) in self.locals.iter().enumerate().rev() {
            if local == name {
                return Some(index as u16);
            }
        }
        None
    }

    fn evaluate_statments(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.evaluate_statement(statement)
        }
    }
    fn var_declaration(&mut self, name: &str, expr: &Expr, line: u32) -> CompileResult<()> {
        if self.resolve_local(name).is_some() || self.get_global(name).is_some() {
            return Err(CompileError {
                message: "Cannot redeclare variable".into(),
                line,
            });
        }
        if self.depth != 0 {
            let reg = u16::try_from(self.locals.len()).map_err(|_| CompileError {
                line,
                message: "Cannot have more than 65535 locals".into(),
            })?;
            self.locals.push(name.to_string());
            self.push_register();
            match self.evaluate_expr(expr)? {
                ExprResult::Register(reg2) => self.write_op_move(reg, reg2, line),
                ExprResult::Accumulator => self.write_op_store_register(reg, line),
                ExprResult::Int(i) => {
                    self.write_op_load_int(i, line);
                    self.write_op_store_register(reg, line)
                }
                ExprResult::Float(f) => {
                    let c = self.bc().float_constant(f);
                    self.load_const(c, line)?;
                    self.write_op_store_register(reg, line)
                }
            }
        } else {
            let g = self.new_global(name);
            let res = self.evaluate_expr(expr)?;
            self.store_in_accumulator(res, line)?;
            self.write_op_store_global(g, line);
        }
        Ok(())
    }

    fn evaluate_statement(&mut self, statement: &Statement) {
        match (|| -> CompileResult<()> {
            match statement {
                Statement::Expr(expr) => {
                    self.evaluate_expr(expr)?;
                }
                Statement::VarDeclaration { name, expr, line } => {
                    self.var_declaration(name, expr, *line)?;
                }
                Statement::Block(b) => {
                    self.depth += 1;
                    self.evaluate_statments(b);
                    self.depth -= 1;
                }
            };
            Ok(())
        })() {
            Err(e) => self.error(e),
            _ => {}
        }
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> CompileResult<ExprResult> {
        match expr {
            Expr::Literal { inner, line } => match inner {
                TokenType::IntLiteral(i) => Ok(ExprResult::Int(*i)),
                TokenType::FloatLiteral(f) => Ok(ExprResult::Float(*f)),
                TokenType::Null => {
                    self.write_op(Op::LoadNull, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::True => {
                    self.write_op(Op::LoadTrue, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::False => {
                    self.write_op(Op::LoadFalse, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::Symbol(sym) => {
                    self.load_const(
                        self.bc().symbol_constant(sym.as_str().into(), self.vm()),
                        *line,
                    )?;
                    Ok(ExprResult::Accumulator)
                }
                _ => todo!(),
            },
            Expr::Binary {
                left,
                op,
                right,
                line,
            } => match op {
                TokenType::Plus => self.add(left, right, *line),
                TokenType::Minus => self.subtract(left, right, *line),
                TokenType::Star => self.multiply(left, right, *line),
                TokenType::Slash => self.divide(left, right, *line),
                TokenType::Equal => self.equal(left, right, *line),
                TokenType::PlusEqual => self.equal(
                    left,
                    &Expr::Binary {
                        left: left.clone(),
                        op: TokenType::Plus,
                        right: right.clone(),
                        line: *line,
                    },
                    *line,
                ),
                TokenType::MinusEqual => self.equal(
                    left,
                    &Expr::Binary {
                        left: left.clone(),
                        op: TokenType::Minus,
                        right: right.clone(),
                        line: *line,
                    },
                    *line,
                ),
                TokenType::StarEqual => self.equal(
                    left,
                    &Expr::Binary {
                        left: left.clone(),
                        op: TokenType::Star,
                        right: right.clone(),
                        line: *line,
                    },
                    *line,
                ),
                TokenType::SlashEqual => self.equal(
                    left,
                    &Expr::Binary {
                        left: left.clone(),
                        op: TokenType::Slash,
                        right: right.clone(),
                        line: *line,
                    },
                    *line,
                ),
                _ => todo!(),
            },
            Expr::Unary { op, right, line } => match op {
                TokenType::Minus => self.negate(right, *line),
                _ => todo!(),
            },
            Expr::Variable { name, line } => match self.resolve_local(name) {
                Some(index) => Ok(ExprResult::Register(index)),
                None => {
                    let global = self
                        .get_global(name)
                        .unwrap_or_else(|| self.new_global(name));
                    self.write_op_load_global(global, *line);
                    Ok(ExprResult::Accumulator)
                }
            },
            Expr::String { inner, line } => {
                if let Substring::String(s) = &inner[0] {
                    self.load_const(
                        self.bc().string_constant(s.as_str().into(), self.vm()),
                        *line,
                    )?;
                } else {
                    unreachable!()
                }
                if inner.len() > 1 {
                    let reg = self.push_register().ok_or(CompileError {
                        message: "Cannot have more than 65535 locals+expressions".into(),
                        line: *line,
                    })?;
                    self.write_op_store_register(reg, *line);
                    for i in &inner[1..] {
                        match i {
                            Substring::String(s) => {
                                if !s.is_empty() {
                                    self.load_const(
                                        self.bc().string_constant(s.as_str().into(), self.vm()),
                                        *line,
                                    )?;
                                } else {
                                    continue;
                                }
                            }
                            Substring::Expr(expr) => {
                                let expr = self.evaluate_expr(expr)?;
                                self.store_in_accumulator(expr, *line)?;
                                self.write_op(Op::ToString, *line);
                            }
                        }
                        self.write_op_u16arg(Op::ConcatRegister, reg, *line);
                    }
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
        }
    }
    fn negate(&mut self, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        match self.evaluate_expr(right)? {
            ExprResult::Register(r) => {
                self.write_op_load_register(r, line);
                self.write_op(Op::Negate, line);
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Accumulator => {
                self.write_op(Op::Negate, line);
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Int(i) => Ok(ExprResult::Int(i.checked_neg().ok_or_else(|| {
                CompileError {
                    message: format!(
                        "Cannot negate {} as the result cannot be stored in an int",
                        i
                    ),
                    line,
                }
            })?)),
            ExprResult::Float(f) => Ok(ExprResult::Float(-f)),
        }
    }

    fn concat(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        let left = self.evaluate_expr(left)?;
        let reg;
        match left {
            ExprResult::Register(r) => reg = r,
            ExprResult::Accumulator => {
                reg = self.push_register().ok_or(CompileError {
                    message: "Cannot have more than 65535 locals+expressions".into(),
                    line,
                })?;
                self.write_op_store_register(reg, line)
            }
            ExprResult::Int(_) | ExprResult::Float(_) => {
                return Err(CompileError {
                    message:
                        "Can only perform concat operation on string.Consider using interpolation"
                            .into(),
                    line,
                })
            }
        }
        let right = self.evaluate_expr(right)?;
        if matches!(left, ExprResult::Accumulator) {
            self.pop_register();
        }
        match right {
            ExprResult::Int(_) => todo!(),
            ExprResult::Float(_) => todo!(),
            right => {
                self.store_in_accumulator(right, line);
                self.write_op_u16arg(Op::ConcatRegister, reg, line);
                Ok(ExprResult::Accumulator)
            }
        }
    }

    fn equal(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        if let Expr::Variable { name, line } = left {
            if name.chars().next().unwrap().is_ascii_uppercase() {
                return Err(CompileError {
                    message: format!("Cannot modify constant {}", name),
                    line: *line,
                });
            }
            if let Some(dest) = self.resolve_local(name) {
                match self.evaluate_expr(right)? {
                    ExprResult::Register(r) => {
                        self.write_op_move(r, dest, *line);
                        Ok(ExprResult::Register(dest))
                    }
                    res => {
                        self.store_in_accumulator(res, *line)?;
                        self.write_op_store_register(dest, *line);
                        Ok(ExprResult::Accumulator)
                    }
                }
            } else {
                let global = self
                    .get_global(name)
                    .unwrap_or_else(|| self.new_global(name));
                let res = self.evaluate_expr(right)?;
                self.store_in_accumulator(res, *line)?;
                self.write_op_store_global(global, *line);
                Ok(ExprResult::Accumulator)
            }
        } else {
            Err(CompileError {
                message: "Invalid target for assignment".to_string(),
                line,
            })
        }
    }

    binary_op!(add, AddRegister, write_op_add_int, add, checked_add);
    binary_op!(
        subtract,
        SubtractRegister,
        write_op_subtract_int,
        sub,
        checked_sub
    );
    binary_op!(
        multiply,
        MultiplyRegister,
        write_op_multiply_int,
        mul,
        checked_mul
    );
    binary_op!(
        divide,
        DivideRegister,
        write_op_divide_int,
        div,
        checked_div
    );
}

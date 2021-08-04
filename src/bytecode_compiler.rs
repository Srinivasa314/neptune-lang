use rustc_hash::FxHashMap;

use crate::bytecode;
use crate::bytecode::BytecodeWriter;
use crate::bytecode::Op;
use crate::gc;
use crate::gc::GC;
use crate::parser::Statement;
use crate::parser::Substring;
use crate::CompileError;
use crate::CompileResult;
use crate::{parser::Expr, scanner::TokenType, value::Value};
use std::convert::TryFrom;
use std::convert::TryInto;
use std::ops::{Add, Div, Mul, Sub};

pub struct Compiler<'gc> {
    globals: FxHashMap<String, u32>,
    errors: Vec<CompileError>,
    gc: &'gc GC,
}

impl<'gc> Compiler<'gc> {
    pub fn new(gc: &'gc GC) -> Self {
        Self {
            gc,
            globals: FxHashMap::default(),
            errors: vec![],
        }
    }
}

pub struct BytecodeCompiler<'c, 'gc> {
    compiler: Option<&'c mut Compiler<'gc>>,
    pub writer: BytecodeWriter<'gc>,
    locals: Vec<String>,
    depth: u32,
    regcount: u16,
}

impl<'c, 'gc> BytecodeCompiler<'c, 'gc> {
    pub fn new(c: &'c mut Compiler<'gc>) -> Self {
        Self {
            writer: bytecode::BytecodeWriter::new(),
            locals: vec![],
            regcount: 0,
            depth: 0,
            compiler: Some(c),
        }
    }

    fn push_register(&mut self) -> Option<u16> {
        if self.regcount == u16::MAX {
            None
        } else {
            self.regcount += 1;
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
        let g = &mut self.compiler.as_mut().unwrap().globals;
        let len = g.len() as u32;
        g.insert(name.to_string(), len);
        len
    }

    fn error(&mut self, e: CompileError) {
        self.compiler.as_mut().unwrap().errors.push(e)
    }

    fn gc(&self) -> &'gc GC {
        self.compiler.as_ref().unwrap().gc
    }
}

macro_rules! binary_op_register {
    ($fn_name:ident,$inst_name:ident) => {
        fn $fn_name(&mut self, reg: u16, line: u32) {
            if let Ok(reg) = u8::try_from(reg) {
                self.writer.write_op(Op::$inst_name, line);
                self.writer.write_u8(reg)
            } else {
                self.writer.write_op(Op::Wide, line);
                self.writer.write_op(Op::$inst_name, line);
                self.writer.write_u16(reg)
            }
        }
    };
}

macro_rules! binary_op_int {
    ($fn_name:ident,$inst_name:ident) => {
        fn $fn_name(&mut self, i: i32, line: u32) {
            if let Ok(i) = i8::try_from(i) {
                self.writer.write_op(Op::$inst_name, line);
                self.writer.write_i8(i);
            } else if let Ok(i) = i16::try_from(i) {
                self.writer.write_op(Op::Wide, line);
                self.writer.write_op(Op::$inst_name, line);
                self.writer.write_i16(i);
            } else {
                self.writer.write_op(Op::ExtraWide, line);
                self.writer.write_op(Op::$inst_name, line);
                self.writer.write_i32(i);
            }
        }
    };
}

impl<'c, 'gc> BytecodeCompiler<'c, 'gc> {
    fn write_op_load_register(&mut self, reg: u16, line: u32) {
        if let Ok(reg) = u8::try_from(reg) {
            self.writer.write_op(Op::LoadRegister, line);
            self.writer.write_u8(reg)
        } else {
            self.writer.write_op(Op::Wide, line);
            self.writer.write_op(Op::LoadRegister, line);
            self.writer.write_u16(reg)
        }
    }

    fn write_op_store_register(&mut self, reg: u16, line: u32) {
        match reg {
            0..=15 => self
                .writer
                .write_op((Op::StoreR0 as u8 + reg as u8).try_into().unwrap(), line),
            16..=255 => {
                self.writer.write_op(Op::StoreRegister, line);
                self.writer.write_u8(reg as u8)
            }
            _ => {
                self.writer.write_op(Op::Wide, line);
                self.writer.write_op(Op::StoreRegister, line);
                self.writer.write_u16(reg)
            }
        }
    }

    binary_op_int!(write_op_load_int, LoadInt);

    fn write_op_move(&mut self, reg1: u16, reg2: u16, line: u32) {
        match (u8::try_from(reg1), u8::try_from(reg2)) {
            (Ok(reg1), Ok(reg2)) => {
                self.writer.write_op(Op::Move, line);
                self.writer.write_u8(reg1);
                self.writer.write_u8(reg2)
            }
            _ => {
                self.writer.write_op(Op::Wide, line);
                self.writer.write_op(Op::Move, line);
                self.writer.write_u16(reg1);
                self.writer.write_u16(reg2)
            }
        }
    }

    fn write_op_store_global(&mut self, global: u32, line: u32) {
        if let Ok(global) = u8::try_from(global) {
            self.writer.write_op(Op::StoreGlobal, line);
            self.writer.write_u8(global);
        } else if let Ok(global) = u16::try_from(global) {
            self.writer.write_op(Op::Wide, line);
            self.writer.write_op(Op::StoreGlobal, line);
            self.writer.write_u16(global);
        } else {
            self.writer.write_op(Op::ExtraWide, line);
            self.writer.write_op(Op::StoreGlobal, line);
            self.writer.write_u32(global);
        }
    }

    fn write_op_load_global(&mut self, global: u32, line: u32) {
        if let Ok(global) = u8::try_from(global) {
            self.writer.write_op(Op::LoadGlobal, line);
            self.writer.write_u8(global);
        } else if let Ok(global) = u16::try_from(global) {
            self.writer.write_op(Op::Wide, line);
            self.writer.write_op(Op::LoadGlobal, line);
            self.writer.write_u16(global);
        } else {
            self.writer.write_op(Op::ExtraWide, line);
            self.writer.write_op(Op::LoadGlobal, line);
            self.writer.write_u32(global);
        }
    }

    binary_op_register!(write_op_add_register, AddRegister);

    binary_op_register!(write_op_subtract_register, SubtractRegister);

    binary_op_register!(write_op_multiply_register, MultiplyRegister);

    binary_op_register!(write_op_divide_register, DivideRegister);

    binary_op_int!(write_op_add_int, AddInt);

    binary_op_int!(write_op_subtract_int, SubtractInt);

    binary_op_int!(write_op_multiply_int, MultiplyInt);

    binary_op_int!(write_op_divide_int, DivideInt);

    fn write_op_negate(&mut self, line: u32) {
        self.writer.write_op(Op::Negate, line)
    }
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
            let reg = self.push_register().ok_or(CompileError {
                message: "Cannot have more than 65535 locals+expressions".into(),
                line,
            })?;
            self.store_in_accumulator(left, line)?;
            self.write_op_store_register(reg, line);
            let right = self.evaluate_expr(right)?;
            self.pop_register();
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
                    self.undo_save_to_register(ExprResult::Register(r));
                    self.store_in_accumulator(right, line)?;
                    self.$register_inst(r, line);
                    Ok(ExprResult::Accumulator)
                }

                (_, right) => {
                    self.store_in_accumulator(right, line)?;
                    self.$register_inst(reg, line);
                    Ok(ExprResult::Accumulator)
                }
            }
        }
    };
}

impl<'c, 'gc> BytecodeCompiler<'c, 'gc> {
    fn undo_save_to_register(&mut self, result: ExprResult) {
        match result {
            ExprResult::Register(_) => {
                self.writer.pop_last_op();
            }
            ExprResult::Accumulator => {}
            ExprResult::Int(_) => {
                self.writer.pop_last_op();
            }
            ExprResult::Float(_) => {
                self.writer.pop_last_op();
            }
        }
        self.writer.pop_last_op();
    }

    fn store_in_accumulator(&mut self, result: ExprResult, line: u32) -> CompileResult<()> {
        match result {
            ExprResult::Register(reg) => Ok(self.write_op_load_register(reg, line)),
            ExprResult::Accumulator => Ok(()),
            ExprResult::Int(i) => Ok(self.write_op_load_int(i, line)),
            ExprResult::Float(f) => self.load_const(Value::from_f64(f), line),
        }
    }

    fn load_const(&mut self, v: Value<'gc>, line: u32) -> CompileResult<()> {
        let c = self.writer.new_constant(v).map_err(|_| CompileError {
            message: "Cannot have more than 65535 constants per function".into(),
            line,
        })?;
        if let Ok(c) = u8::try_from(c) {
            self.writer.write_op(Op::LoadConstant, line);
            self.writer.write_u8(c);
        } else if let Ok(c) = u16::try_from(c) {
            self.writer.write_op(Op::Wide, line);
            self.writer.write_op(Op::LoadConstant, line);
            self.writer.write_u16(c);
        }
        Ok(())
    }

    fn resolve_local(&self, name: &str) -> Option<u16> {
        for (index, local) in self.locals.iter().enumerate().rev() {
            if local == name {
                return Some(index as u16);
            }
        }
        None
    }

    pub fn evaluate_statments(&mut self, statements: &[Statement]) {
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
                    self.load_const(Value::from_f64(f), line)?;
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
                    self.writer.write_op(Op::LoadNull, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::True => {
                    self.writer.write_op(Op::LoadTrue, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::False => {
                    self.writer.write_op(Op::LoadFalse, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::Symbol(sym) => {
                    let sym = self.gc().intern_constant_symbol(sym);
                    self.load_const(Value::from_object(sym.into()), *line)?;
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
                        Value::from_object(gc::Object::from(
                            self.compiler.as_ref().unwrap().gc.alloc_constant(s.clone()),
                        )),
                        *line,
                    )
                    .map_err(|e| CompileError {
                        message: "too many constants".to_string(),
                        line: *line,
                    })?;
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
                                        Value::from_object(gc::Object::from(
                                            self.compiler
                                                .as_ref()
                                                .unwrap()
                                                .gc
                                                .alloc_constant(s.clone()),
                                        )),
                                        *line,
                                    )
                                    .map_err(|e| {
                                        CompileError {
                                            message: "too many constants".to_string(),
                                            line: *line,
                                        }
                                    })?
                                } else {
                                    continue;
                                }
                            }
                            Substring::Expr(expr) => {
                                let expr = self.evaluate_expr(expr)?;
                                self.store_in_accumulator(expr, *line)?;
                                self.writer.write_op(Op::ToString, *line);
                            }
                        }
                        self.write_op_add_register(reg, *line);
                        self.write_op_store_register(reg, *line);
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
                self.write_op_negate(line);
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Accumulator => {
                self.write_op_negate(line);
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

    binary_op!(
        add,
        write_op_add_register,
        write_op_add_int,
        add,
        checked_add
    );
    binary_op!(
        subtract,
        write_op_subtract_register,
        write_op_subtract_int,
        sub,
        checked_sub
    );
    binary_op!(
        multiply,
        write_op_multiply_register,
        write_op_multiply_int,
        mul,
        checked_mul
    );
    binary_op!(
        divide,
        write_op_divide_register,
        write_op_divide_int,
        div,
        checked_div
    );
}

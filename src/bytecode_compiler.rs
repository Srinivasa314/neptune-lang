use cxx::Exception;

use crate::parser::Statement;
use crate::parser::Substring;
use crate::vm::FunctionInfoWriter;
use crate::vm::Op;
use crate::vm::VM;
use crate::CompileError;
use crate::CompileResult;
use crate::{parser::Expr, scanner::TokenType};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ops::{Add, Div, Mul, Sub};

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

    pub fn compile(
        mut self,
        ast: Vec<Statement>,
    ) -> Result<FunctionInfoWriter<'vm>, Vec<CompileError>> {
        let mut b = BytecodeCompiler::new(&mut self);
        b.evaluate_statments(&ast);
        b.write0(Op::Exit, 0);
        let bytecode = b.bytecode;
        if self.errors.is_empty() {
            Ok(bytecode)
        } else {
            Err(self.errors)
        }
    }
}

struct BytecodeCompiler<'c, 'vm> {
    compiler: Option<&'c mut Compiler<'vm>>,
    bytecode: FunctionInfoWriter<'vm>,
    locals: Vec<String>,
    regcounts: Vec<u16>,
    max_registers: u16,
    op_positions: Vec<usize>,
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn new(c: &'c mut Compiler<'vm>) -> Self {
        Self {
            bytecode: c.vm.new_function_info(),
            locals: vec![],
            regcounts: vec![0],
            compiler: Some(c),
            max_registers: 0,
            op_positions: vec![],
        }
    }

    fn regcount(&self) -> u16 {
        self.regcounts.last().cloned().unwrap()
    }

    fn regcount_mut(&mut self) -> &mut u16 {
        self.regcounts.last_mut().unwrap()
    }

    fn push_register(&mut self, line: u32) -> CompileResult<u16> {
        if self.regcount() == u16::MAX {
            Err(CompileError {
                message: "Cannot have more than 65535 locals+expressions".into(),
                line,
            })
        } else {
            *self.regcount_mut() += 1;
            if self.regcount() > self.max_registers {
                self.max_registers = self.regcount();
            }
            Ok(self.regcount() - 1)
        }
    }

    fn pop_register(&mut self) {
        *self.regcount_mut() -= 1;
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

    fn write0(&mut self, op: Op, line: u32) {
        let pos = self.bytecode.write_op(op, line);
        self.op_positions.push(pos);
    }

    fn write1(&mut self, op: Op, u: u32, line: u32) {
        if let Ok(u) = u8::try_from(u) {
            self.write0(op, line);
            self.bytecode.write_u8(u);
        } else if let Ok(u) = u16::try_from(u) {
            self.write0(Op::Wide, line);
            self.write0(op, line);
            self.bytecode.write_u16(u);
        } else {
            self.write0(Op::ExtraWide, line);
            self.write0(op, line);
            self.bytecode.write_u32(u);
        }
    }

    fn write2(&mut self, op: Op, u1: u16, u2: u16, line: u32) {
        match (u8::try_from(u1), u8::try_from(u2)) {
            (Ok(u1), Ok(u2)) => {
                self.write0(op, line);
                self.bytecode.write_u8(u1);
                self.bytecode.write_u8(u2)
            }
            _ => {
                self.write0(Op::Wide, line);
                self.write0(op, line);
                self.bytecode.write_u16(u1);
                self.bytecode.write_u16(u2)
            }
        }
    }

    fn pop_last_op(&mut self) {
        let pos = self.op_positions.pop().unwrap();
        self.bytecode.pop_last_op(pos);
    }
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn write_op_store_register(&mut self, reg: u16, line: u32) {
        match reg {
            0..=15 => {
                self.write0(
                    Op {
                        repr: match reg {
                            0 => Op::StoreR0.repr,
                            1 => Op::StoreR1.repr,
                            2 => Op::StoreR2.repr,
                            3 => Op::StoreR3.repr,
                            4 => Op::StoreR4.repr,
                            5 => Op::StoreR5.repr,
                            6 => Op::StoreR6.repr,
                            7 => Op::StoreR7.repr,
                            8 => Op::StoreR8.repr,
                            9 => Op::StoreR9.repr,
                            10 => Op::StoreR10.repr,
                            11 => Op::StoreR11.repr,
                            12 => Op::StoreR12.repr,
                            13 => Op::StoreR13.repr,
                            14 => Op::StoreR14.repr,
                            15 => Op::StoreR15.repr,
                            _ => unreachable!(),
                        },
                    },
                    line,
                );
            }
            _ => {
                self.write1(Op::StoreRegister, reg as u32, line);
            }
        }
    }

    fn write_op_load_register(&mut self, reg: u16, line: u32) {
        match reg {
            0..=15 => {
                self.write0(
                    Op {
                        repr: match reg {
                            0 => Op::LoadR0.repr,
                            1 => Op::LoadR1.repr,
                            2 => Op::LoadR2.repr,
                            3 => Op::LoadR3.repr,
                            4 => Op::LoadR4.repr,
                            5 => Op::LoadR5.repr,
                            6 => Op::LoadR6.repr,
                            7 => Op::LoadR7.repr,
                            8 => Op::LoadR8.repr,
                            9 => Op::LoadR9.repr,
                            10 => Op::LoadR10.repr,
                            11 => Op::LoadR11.repr,
                            12 => Op::LoadR12.repr,
                            13 => Op::LoadR13.repr,
                            14 => Op::LoadR14.repr,
                            15 => Op::LoadR15.repr,
                            _ => unreachable!(),
                        },
                    },
                    line,
                );
            }
            _ => {
                self.write1(Op::LoadRegister, reg as u32, line);
            }
        }
    }

    fn load_const(&mut self, constant: Result<u16, Exception>, line: u32) -> CompileResult<()> {
        self.write1(
            Op::LoadConstant,
            constant.map_err(|_| CompileError {
                line,
                message: "Cannot have more than 65535 constants per function".into(),
            })? as u32,
            line,
        );
        Ok(())
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
            let mut reg = 0;
            if !matches!(left, ExprResult::Register(_)) {
                reg = self.push_register(line)?;
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
                (ExprResult::Register(r), right) => {
                    self.store_in_accumulator(right, line)?;
                    self.write1(Op::$register_inst, r as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (left, ExprResult::Int(i)) => {
                    self.undo_save_to_register(left);
                    self.store_in_accumulator(left, line)?;
                    self.write1(Op::$int_inst, i as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (_, right) => {
                    self.store_in_accumulator(right, line)?;
                    self.write1(Op::$register_inst, reg as u32, line);
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
            ExprResult::Register(reg) => {
                self.write_op_load_register(reg, line);
                Ok(())
            }
            ExprResult::Accumulator => Ok(()),
            ExprResult::Int(i) => {
                self.write1(Op::LoadInt, i as u32, line);
                Ok(())
            }
            ExprResult::Float(f) => {
                let c = self.bytecode.float_constant(f);
                self.load_const(c, line)
            }
        }
    }

    fn store_in_register(&mut self, result: ExprResult, line: u32) -> CompileResult<u16> {
        if let ExprResult::Register(r) = result {
            Ok(r)
        } else {
            self.store_in_accumulator(result, line)?;
            let reg = self.push_register(line)?;
            self.write_op_store_register(reg, line);
            Ok(reg)
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
        self.bytecode.shrink();
        self.bytecode.set_max_registers(self.max_registers);
    }
    fn var_declaration(&mut self, name: &str, expr: &Expr, line: u32) -> CompileResult<()> {
        if self.resolve_local(name).is_some() || self.get_global(name).is_some() {
            return Err(CompileError {
                message: "Cannot redeclare variable".into(),
                line,
            });
        }
        if self.regcounts.len() != 1 {
            let reg = u16::try_from(self.locals.len()).map_err(|_| CompileError {
                line,
                message: "Cannot have more than 65535 locals".into(),
            })?;
            self.locals.push(name.to_string());
            self.push_register(line)?;
            match self.evaluate_expr(expr)? {
                ExprResult::Register(reg2) => self.write2(Op::Move, reg, reg2, line),
                ExprResult::Accumulator => self.write_op_store_register(reg, line),
                ExprResult::Int(i) => {
                    self.write1(Op::LoadInt, i as u32, line);
                    self.write_op_store_register(reg, line)
                }
                ExprResult::Float(f) => {
                    let c = self.bytecode.float_constant(f);
                    self.load_const(c, line)?;
                    self.write_op_store_register(reg, line)
                }
            }
        } else {
            let g = self.new_global(name);
            let res = self.evaluate_expr(expr)?;
            self.store_in_accumulator(res, line)?;
            self.write1(Op::StoreGlobal, g, line);
        }
        Ok(())
    }

    fn evaluate_statement(&mut self, statement: &Statement) {
        if let Err(e) = (|| -> CompileResult<()> {
            match statement {
                Statement::Expr(expr) => match expr {
                    Expr::Binary {
                        left,
                        op,
                        right,
                        line,
                    } => {
                        if *op == TokenType::Equal {
                            self.equal(left, right, *line)?;
                        } else if *op == TokenType::PlusEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Plus,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::MinusEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Minus,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::StarEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Star,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::SlashEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Slash,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::TildeEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Tilde,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else {
                            self.evaluate_expr(expr)?;
                        }
                    }
                    _ => {
                        self.evaluate_expr(expr)?;
                    }
                },
                Statement::VarDeclaration { name, expr, line } => {
                    self.var_declaration(name, expr, *line)?;
                }
                Statement::Block(b) => {
                    let count = self.regcount();
                    self.regcounts.push(count);
                    self.evaluate_statments(b);
                    self.regcounts.pop();
                    self.locals.truncate(self.regcount() as usize);
                }
            };
            Ok(())
        })() {
            self.error(e)
        }
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> CompileResult<ExprResult> {
        match expr {
            Expr::Literal { inner, line } => match inner {
                TokenType::IntLiteral(i) => Ok(ExprResult::Int(*i)),
                TokenType::FloatLiteral(f) => Ok(ExprResult::Float(*f)),
                TokenType::Null => {
                    self.write0(Op::LoadNull, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::True => {
                    self.write0(Op::LoadTrue, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::False => {
                    self.write0(Op::LoadFalse, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::Symbol(sym) => {
                    let sym = self.bytecode.symbol_constant(sym.as_str().into());
                    self.load_const(sym, *line)?;
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
                TokenType::Equal => Err(CompileError {
                    message: "= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::PlusEqual => Err(CompileError {
                    message: "+= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::MinusEqual => Err(CompileError {
                    message: "-= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::StarEqual => Err(CompileError {
                    message: "*= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::SlashEqual => Err(CompileError {
                    message: "/= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::Tilde => self.concat(left, right, *line),
                TokenType::TildeEqual => Err(CompileError {
                    message: "~= is not an expression".to_string(),
                    line: *line,
                }),
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
                    self.write1(Op::LoadGlobal, global, *line);
                    Ok(ExprResult::Accumulator)
                }
            },
            Expr::String { inner, line } => {
                if let Substring::String(s) = &inner[0] {
                    let str = self.bytecode.string_constant(s.as_str().into());
                    self.load_const(str, *line)?;
                } else {
                    unreachable!()
                }
                if inner.len() > 1 {
                    let reg = self.push_register(*line)?;
                    self.write_op_store_register(reg, *line);
                    for i in &inner[1..] {
                        match i {
                            Substring::String(s) => {
                                if !s.is_empty() {
                                    let str = self.bytecode.string_constant(s.as_str().into());
                                    self.load_const(str, *line)?;
                                } else {
                                    continue;
                                }
                            }
                            Substring::Expr(expr) => {
                                let expr = self.evaluate_expr(expr)?;
                                self.store_in_accumulator(expr, *line)?;
                                self.write0(Op::ToString, *line);
                            }
                        }
                        self.write1(Op::ConcatRegister, reg as u32, *line);
                    }
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::Array { inner, line } => {
                let array_reg = self.push_register(*line)?;
                self.write2(
                    Op::NewArray,
                    u16::try_from(inner.len()).map_err(|_| CompileError {
                        message: "Array literal too large".to_string(),
                        line: *line,
                    })?,
                    array_reg,
                    *line,
                );
                for (index, expr) in inner.iter().enumerate() {
                    let expr_res = self.evaluate_expr(expr)?;
                    self.store_in_accumulator(expr_res, *line)?;
                    self.write2(
                        Op::StoreArrayUnchecked,
                        array_reg,
                        u16::try_from(index).unwrap(),
                        *line,
                    );
                }
                self.write_op_load_register(array_reg, *line);
                self.pop_register();
                Ok(ExprResult::Accumulator)
            }
            Expr::Subscript {
                object,
                subscript,
                line,
            } => {
                let res = self.evaluate_expr(object)?;
                let reg = self.store_in_register(res, *line)?;
                let subscript = self.evaluate_expr(subscript)?;
                self.store_in_accumulator(subscript, *line)?;
                self.write1(Op::LoadSubscript, reg as u32, *line);
                if !matches!(res, ExprResult::Register(_)) {
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
                self.write0(Op::Negate, line);
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Accumulator => {
                self.write0(Op::Negate, line);
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
                reg = self.push_register(line)?;
                self.write_op_store_register(reg, line)
            }
            ExprResult::Int(_) | ExprResult::Float(_) => {
                return Err(CompileError {
                    message:
                        "Can only perform concat operation on string. Consider using interpolation"
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
                self.store_in_accumulator(right, line)?;
                self.write1(Op::ConcatRegister, reg as u32, line);
                Ok(ExprResult::Accumulator)
            }
        }
    }

    fn equal(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<()> {
        if let Expr::Subscript {
            object,
            subscript,
            line,
        } = left
        {
            let object = self.evaluate_expr(object)?;
            let object_reg = self.store_in_register(object, *line)?;
            let subscript = self.evaluate_expr(subscript)?;
            let subscript_reg = self.store_in_register(subscript, *line)?;
            let right = self.evaluate_expr(right)?;
            self.store_in_accumulator(right, *line)?;
            self.write2(Op::StoreSubscript, object_reg, subscript_reg, *line);
            if !matches!(subscript, ExprResult::Register(_)) {
                self.pop_register();
            }
            if !matches!(object, ExprResult::Register(_)) {
                self.pop_register();
            }
            return Ok(());
        }
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
                        self.write2(Op::Move, r, dest, *line);
                        Ok(())
                    }
                    res => {
                        self.store_in_accumulator(res, *line)?;
                        self.write_op_store_register(dest, *line);
                        Ok(())
                    }
                }
            } else {
                let global = self
                    .get_global(name)
                    .unwrap_or_else(|| self.new_global(name));
                let res = self.evaluate_expr(right)?;
                self.store_in_accumulator(res, *line)?;
                self.write1(Op::StoreGlobal, global, *line);
                Ok(())
            }
        } else {
            Err(CompileError {
                message: "Invalid target for assignment".to_string(),
                line,
            })
        }
    }

    binary_op!(add, AddRegister, AddInt, add, checked_add);
    binary_op!(subtract, SubtractRegister, SubtractInt, sub, checked_sub);
    binary_op!(multiply, MultiplyRegister, MultiplyInt, mul, checked_mul);
    binary_op!(divide, DivideRegister, DivideInt, div, checked_div);
}

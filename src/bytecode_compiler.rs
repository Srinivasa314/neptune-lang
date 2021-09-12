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

pub struct Compiler<'vm> {
    globals: &'vm mut HashMap<String, u32>,
    errors: Vec<CompileError>,
    vm: &'vm VM,
}

impl<'vm> Compiler<'vm> {
    pub fn new(vm: &'vm VM, globals: &'vm mut HashMap<String, u32>) -> Self {
        Self {
            vm,
            globals,
            errors: vec![],
        }
    }

    pub fn exec(
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

    pub fn can_eval(ast: &[Statement]) -> Option<&Expr> {
        match ast {
            [Statement::Expr(e)] => match e {
                Expr::Binary { op, .. } => match op {
                    TokenType::Equal
                    | TokenType::PlusEqual
                    | TokenType::MinusEqual
                    | TokenType::StarEqual
                    | TokenType::SlashEqual
                    | TokenType::TildeEqual => None,
                    _ => Some(e),
                },
                _ => Some(e),
            },
            _ => None,
        }
    }

    pub fn eval(mut self, ast: &Expr) -> Result<FunctionInfoWriter<'vm>, Vec<CompileError>> {
        let mut b = BytecodeCompiler::new(&mut self);
        match b.evaluate_expr(ast) {
            Ok(er) => b.store_in_accumulator(er, 0),
            Err(e) => b.error(e),
        }
        b.bytecode.shrink();
        b.bytecode.set_max_registers(b.max_registers);
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
}

macro_rules! binary_op {
    ($op:ident,$register_inst:ident,$int_inst:ident,$op_fn:ident,$op_checked_fn:ident,$op_name:tt) => {
        fn $op(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
            let left = self.evaluate_expr(left)?;
            let mut reg = 0;
            if !matches!(left, ExprResult::Register(_)) {
                reg = self.push_register(line)?;
                self.store_in_accumulator(left, line);
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
                                $op_name, i1, i2
                            ),
                            line,
                        },
                    )?))
                }
                (ExprResult::Register(r), right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$register_inst, r as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (left, ExprResult::Int(i)) => {
                    self.undo_save_to_register(left);
                    self.store_in_accumulator(left, line);
                    self.write1(Op::$int_inst, signed_to_unsigned(i), line);
                    Ok(ExprResult::Accumulator)
                }
                (_, right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$register_inst, reg as u32, line);
                    Ok(ExprResult::Accumulator)
                }
            }
        }
    };
}

macro_rules! comparing_binary_op {
    ($op:ident,$inst:ident,$op_symbol:tt) => {
        fn $op(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
            let left = self.evaluate_expr(left)?;
            let mut reg = 0;
            if !matches!(left, ExprResult::Register(_)) {
                reg = self.push_register(line)?;
                self.store_in_accumulator(left, line);
                self.write_op_store_register(reg, line);
            }
            let right = self.evaluate_expr(right)?;
            if !matches!(left, ExprResult::Register(_)) {
                self.pop_register();
            }
            match (left, right) {
                (ExprResult::Int(i1), ExprResult::Int(i2)) => {
                    self.undo_save_to_register(ExprResult::Int(i1));
                    self.write0(if i1 $op_symbol i2{Op::LoadTrue} else {Op::LoadFalse},line);
                    Ok(ExprResult::Accumulator)
                }
                (ExprResult::Register(r), right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$inst, r as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (_, right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$inst, reg as u32, line);
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
        }
        self.pop_last_op();
    }

    fn store_in_accumulator(&mut self, result: ExprResult, line: u32) {
        match result {
            ExprResult::Register(reg) => {
                self.write_op_load_register(reg, line);
            }
            ExprResult::Accumulator => {}
            ExprResult::Int(i) => {
                self.write1(Op::LoadInt, signed_to_unsigned(i), line);
            }
        }
    }

    fn store_in_register(&mut self, result: ExprResult, line: u32) -> CompileResult<u16> {
        if let ExprResult::Register(r) = result {
            Ok(r)
        } else {
            self.store_in_accumulator(result, line);
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
        if self.regcounts.len() != 1 {
            // Shadowing cannot be done within the same block
            let last_block = self.regcounts[self.regcounts.len() - 2];
            let this_block = self.regcount();
            if self.locals[((last_block) as usize)..(this_block as usize)]
                .iter()
                .any(|local| local == name)
            {
                return Err(CompileError {
                    message: format!("Cannot redeclare variable {}", name),
                    line,
                });
            }
            let reg = u16::try_from(self.locals.len()).map_err(|_| CompileError {
                line,
                message: "Cannot have more than 65535 locals".into(),
            })?;
            self.locals.push(name.to_string());
            self.push_register(line)?;
            match self.evaluate_expr(expr)? {
                ExprResult::Register(reg2) => self.write2(Op::Move, reg2, reg, line),
                ExprResult::Accumulator => self.write_op_store_register(reg, line),
                ExprResult::Int(i) => {
                    self.write1(Op::LoadInt, signed_to_unsigned(i), line);
                    self.write_op_store_register(reg, line)
                }
            }
        } else {
            if self.get_global(name).is_some() {
                return Err(CompileError {
                    message: format!("Cannot redeclare variable {}", name),
                    line,
                });
            }
            let g = self.new_global(name);
            let res = self.evaluate_expr(expr)?;
            self.store_in_accumulator(res, line);
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
                    for stmt in b {
                        self.evaluate_statement(stmt)
                    }
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
                TokenType::FloatLiteral(f) => Ok({
                    let c = self.bytecode.float_constant(*f);
                    self.load_const(c, *line)?;
                    ExprResult::Accumulator
                }),
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
                TokenType::EqualEqual => self.equal_equal(left, right, *line),
                TokenType::EqualEqualEqual => self.equal_equal_equal(left, right, *line),
                TokenType::BangEqualEqual => self.not_equal_equal(left, right, *line),
                TokenType::BangEqual => self.not_equal(left, right, *line),
                TokenType::Greater => self.greater_than(left, right, *line),
                TokenType::Less => self.lesser_than(left, right, *line),
                TokenType::GreaterEqual => self.greater_than_or_equal(left, right, *line),
                TokenType::LessEqual => self.lesser_than_or_equal(left, right, *line),
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
                match &inner[0] {
                    Substring::String(s) => {
                        let str = self.bytecode.string_constant(s.as_str().into());
                        self.load_const(str, *line)?;
                    }
                    Substring::Expr(e) => {
                        let expr_res = self.evaluate_expr(e)?;
                        self.store_in_accumulator(expr_res, *line);
                        self.write0(Op::ToString, *line);
                    }
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
                                self.store_in_accumulator(expr, *line);
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
                if inner.is_empty() {
                    self.write0(Op::EmptyArray, *line);
                    return Ok(ExprResult::Accumulator);
                }
                let array_reg = self.push_register(*line)?;
                self.write2(
                    Op::NewArray,
                    u16::try_from(inner.len()).map_err(|_| CompileError {
                        message: "Array literal can have upto 65535 elements".to_string(),
                        line: *line,
                    })?,
                    array_reg,
                    *line,
                );
                for (index, expr) in inner.iter().enumerate() {
                    let expr_res = self.evaluate_expr(expr)?;
                    self.store_in_accumulator(expr_res, expr.line());
                    self.write2(
                        Op::StoreArrayUnchecked,
                        array_reg,
                        u16::try_from(index).unwrap(),
                        expr.line(),
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
                self.store_in_accumulator(subscript, *line);
                self.write1(Op::LoadSubscript, reg as u32, *line);
                if !matches!(res, ExprResult::Register(_)) {
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::Map { inner, line } => {
                if inner.is_empty() {
                    self.write0(Op::EmptyMap, *line);
                    return Ok(ExprResult::Accumulator);
                }
                let map_reg = self.push_register(*line)?;
                self.write2(
                    Op::NewMap,
                    u16::try_from(inner.len()).map_err(|_| CompileError {
                        message: "Map literal can have up to 65535 elements".to_string(),
                        line: *line,
                    })?,
                    map_reg,
                    *line,
                );
                for (key, val) in inner.iter() {
                    let key_res = self.evaluate_expr(key)?;
                    let key_reg = self.store_in_register(key_res, key.line())?;
                    let val_res = self.evaluate_expr(val)?;
                    self.store_in_accumulator(val_res, val.line());
                    self.write2(Op::StoreSubscript, map_reg, key_reg, val.line());
                    if !(matches!(key_res, ExprResult::Register(_))) {
                        self.pop_register();
                    }
                }
                self.write_op_load_register(map_reg, *line);
                self.pop_register();
                Ok(ExprResult::Accumulator)
            }
        }
    }
    fn negate(&mut self, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        if let Expr::Literal {
            inner: TokenType::FloatLiteral(f),
            line,
        } = right
        {
            let c = self.bytecode.float_constant(-f);
            self.load_const(c, *line)?;
            return Ok(ExprResult::Accumulator);
        }
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
            ExprResult::Int(_) => {
                return Err(CompileError {
                    message:
                        "Can only perform concat operation on strings. Consider using interpolation"
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
            ExprResult::Int(_) => {
                return Err(CompileError {
                    message:
                        "Can only perform concat operation on strings. Consider using interpolation"
                            .into(),
                    line,
                })
            }
            right => {
                self.store_in_accumulator(right, line);
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
            self.store_in_accumulator(right, *line);
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
                        self.store_in_accumulator(res, *line);
                        self.write_op_store_register(dest, *line);
                        Ok(())
                    }
                }
            } else {
                let global = self
                    .get_global(name)
                    .unwrap_or_else(|| self.new_global(name));
                let res = self.evaluate_expr(right)?;
                self.store_in_accumulator(res, *line);
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

    binary_op!(add, AddRegister, AddInt, add, checked_add,"add");
    binary_op!(subtract, SubtractRegister, SubtractInt, sub, checked_sub,"subtract");
    binary_op!(multiply, MultiplyRegister, MultiplyInt, mul, checked_mul,"multiply");
    binary_op!(divide, DivideRegister, DivideInt, div, checked_div,"divide");

    comparing_binary_op!(equal_equal,Equal,==);
    comparing_binary_op!(equal_equal_equal,StrictEqual,==);
    comparing_binary_op!(not_equal,NotEqual,!=);
    comparing_binary_op!(not_equal_equal,StrictNotEqual,!=);
    comparing_binary_op!(greater_than,GreaterThan,>);
    comparing_binary_op!(lesser_than,LesserThan,<);
    comparing_binary_op!(greater_than_or_equal,GreaterThanOrEqual,>=);
    comparing_binary_op!(lesser_than_or_equal,LesserThanOrEqual,<=);
}

fn signed_to_unsigned(i: i32) -> u32 {
    match i8::try_from(i) {
        Ok(i) => i as u8 as u32,
        Err(_) => match i16::try_from(i) {
            Ok(i) => i as u16 as u32,
            Err(_) => i as u32,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::Parser, scanner::Scanner, vm::new_vm};

    use super::*;
    #[test]
    fn test() {
        let tests: Vec<String> =
            serde_json::from_str(include_str!("../tests/compiler_tests/tests.json")).unwrap();
        for test in tests {
            let s = std::fs::read_to_string(format!("tests/compiler_tests/{}.np", test)).unwrap();
            let s = Scanner::new(&s);
            let tokens = s.scan_tokens();
            let parser = Parser::new(tokens.into_iter(), false);
            let (stmts, errors) = parser.parse();
            assert!(errors.is_empty(), "{:?}", errors);
            let vm = new_vm();
            let mut globals = HashMap::default();
            let compiler = Compiler::new(&vm, &mut globals);
            let fw = compiler.exec(stmts).unwrap();
            if std::env::var("GENERATE_TESTS").is_ok() {
                std::fs::write(
                    format!("tests/compiler_tests/{}.bc", test),
                    fw.to_cxx_string().as_bytes(),
                )
                .unwrap();
            } else {
                let expected =
                    std::fs::read_to_string(format!("tests/compiler_tests/{}.bc", test)).unwrap();
                assert_eq!(expected, fw.to_cxx_string().to_str().unwrap());
            }
        }
    }

    #[test]
    fn error() {
        let tests: Vec<String> =
            serde_json::from_str(include_str!("../tests/compiler_tests/errors.json")).unwrap();
        for test in tests {
            let s = std::fs::read_to_string(format!("tests/compiler_tests/{}.np", test)).unwrap();
            let s = Scanner::new(&s);
            let tokens = s.scan_tokens();
            let parser = Parser::new(tokens.into_iter(), false);
            let (stmts, errors) = parser.parse();
            assert!(errors.is_empty());
            let vm = new_vm();
            let mut globals = HashMap::default();
            let compiler = Compiler::new(&vm, &mut globals);
            let errors = compiler.exec(stmts).unwrap_err();
            if std::env::var("GENERATE_TESTS").is_ok() {
                std::fs::write(
                    format!("tests/compiler_tests/{}.json", test),
                    serde_json::to_string_pretty(&errors).unwrap(),
                )
                .unwrap();
            } else {
                let expected =
                    std::fs::read_to_string(format!("tests/compiler_tests/{}.json", test)).unwrap();
                assert_eq!(expected, serde_json::to_string_pretty(&errors).unwrap());
            }
        }
    }
}

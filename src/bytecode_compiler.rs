use crate::bytecode::BytecodeWriter;
use crate::bytecode::Local;
use crate::bytecode::Op;
use crate::parser::Statement;
use crate::{bytecode::ExceededMaxConstants, parser::Expr, scanner::TokenType, value::Value};
use std::convert::TryFrom;
use std::ops::{Add, Div, Mul, Sub};

macro_rules! binary_op_register {
    ($fn_name:ident,$inst_name:ident) => {
        fn $fn_name(&mut self, reg: u16, line: u32) {
            if let Ok(reg) = u8::try_from(reg) {
                self.write_op(Op::$inst_name, line);
                self.write_u8(reg)
            } else {
                self.write_op(Op::Wide, line);
                self.write_op(Op::$inst_name, line);
                self.write_u16(reg)
            }
        }
    };
}

macro_rules! binary_op_int {
    ($fn_name:ident,$inst_name:ident) => {
        fn $fn_name(&mut self, i: Int, line: u32) {
            match i {
                Int::I8(i) => {
                    self.write_op(Op::$inst_name, line);
                    self.write_i8(i)
                }
                Int::I16(i) => {
                    self.write_op(Op::Wide, line);
                    self.write_op(Op::$inst_name, line);
                    self.write_i16(i)
                }
                Int::I32(i) => {
                    self.write_op(Op::ExtraWide, line);
                    self.write_op(Op::$inst_name, line);
                    self.write_i32(i)
                }
            }
        }
    };
}
impl BytecodeWriter {
    fn write_op_load_register(&mut self, reg: u16, line: u32) {
        if let Ok(reg) = u8::try_from(reg) {
            self.write_op(Op::LoadRegister, line);
            self.write_u8(reg)
        } else {
            self.write_op(Op::Wide, line);
            self.write_op(Op::LoadRegister, line);
            self.write_u16(reg)
        }
    }

    fn write_op_store_register(&mut self, reg: u16, line: u32) {
        match reg {
            0 => self.write_op(Op::StoreR0, line),
            1 => self.write_op(Op::StoreR1, line),
            2 => self.write_op(Op::StoreR2, line),
            3 => self.write_op(Op::StoreR3, line),
            4 => self.write_op(Op::StoreR4, line),
            _ => todo!(),
        }
    }

    binary_op_int!(write_op_load_int, LoadInt);

    fn write_op_move(&mut self, reg1: u16, reg2: u16, line: u32) {
        match (u8::try_from(reg1), u8::try_from(reg2)) {
            (Ok(reg1), Ok(reg2)) => {
                self.write_op(Op::Move, line);
                self.write_u8(reg1);
                self.write_u8(reg2)
            }
            _ => todo!(),
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
        self.write_op(Op::Negate, line)
    }
}

#[derive(Clone, Copy, Debug)]
enum Int {
    I8(i8),
    I16(i16),
    I32(i32),
}
impl Int {
    fn add(self, rhs: Self) -> Result<Int, ArithmeticError> {
        Ok(Int::from(
            i32::from(self)
                .checked_add(i32::from(rhs))
                .ok_or(ArithmeticError)?,
        ))
    }

    fn sub(self, rhs: Self) -> Result<Int, ArithmeticError> {
        Ok(Int::from(
            i32::from(self)
                .checked_sub(i32::from(rhs))
                .ok_or(ArithmeticError)?,
        ))
    }

    fn mul(self, rhs: Self) -> Result<Int, ArithmeticError> {
        Ok(Int::from(
            i32::from(self)
                .checked_mul(i32::from(rhs))
                .ok_or(ArithmeticError)?,
        ))
    }

    fn div(self, rhs: Self) -> Result<Int, ArithmeticError> {
        Ok(Int::from(
            i32::from(self)
                .checked_div(i32::from(rhs))
                .ok_or(ArithmeticError)?,
        ))
    }

    fn neg(self) -> Result<Int, ArithmeticError> {
        Ok(Int::from(
            i32::from(self).checked_neg().ok_or(ArithmeticError)?,
        ))
    }
}

impl From<Int> for i32 {
    fn from(i: Int) -> Self {
        match i {
            Int::I8(i) => i as i32,
            Int::I16(i) => i as i32,
            Int::I32(i) => i as i32,
        }
    }
}
impl From<i32> for Int {
    fn from(i: i32) -> Self {
        if let Ok(i) = i8::try_from(i) {
            Int::I8(i)
        } else if let Ok(i) = i16::try_from(i) {
            Int::I16(i)
        } else {
            Int::I32(i)
        }
    }
}

impl From<Int> for f64 {
    fn from(i: Int) -> Self {
        f64::from(i32::from(i))
    }
}
#[derive(Debug)]
pub enum ExprResult {
    Register(u16),
    Accumulator,
    Int(Int),
    Float(f64),
}
pub struct ArithmeticError;
pub struct ExceededMaxLocals;
#[derive(Debug)]
pub enum BytecodeCompilerError {
    ArithmeticError,
    ExceededMaxLocals,
    ExceededMaxConstants,
}
impl From<ArithmeticError> for BytecodeCompilerError {
    fn from(_: ArithmeticError) -> Self {
        BytecodeCompilerError::ArithmeticError
    }
}

impl From<ExceededMaxConstants> for BytecodeCompilerError {
    fn from(_: ExceededMaxConstants) -> Self {
        BytecodeCompilerError::ExceededMaxConstants
    }
}

impl From<ExceededMaxLocals> for BytecodeCompilerError {
    fn from(_: ExceededMaxLocals) -> Self {
        BytecodeCompilerError::ExceededMaxLocals
    }
}

macro_rules! binary_op {
    ($op:ident,$register_inst:ident,$int_inst:ident,$op_fn:ident) => {
        fn $op(
            &mut self,
            left: &Expr,
            right: &Expr,
            line: u32,
        ) -> Result<ExprResult, BytecodeCompilerError> {
            match self.evaluate_expr(left)? {
                ExprResult::Register(r1) => match self.evaluate_expr(right)? {
                    ExprResult::Register(r2) => {
                        self.write_op_load_register(r1, line);
                        self.$register_inst(r2, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$register_inst(r1, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i) => {
                        self.write_op_load_register(r1, line);
                        self.$int_inst(i, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Float(f) => {
                        self.write_value(Value::from_f64(f), line);
                        self.$register_inst(r1, line);
                        Ok(ExprResult::Accumulator)
                    }
                },
                ExprResult::Accumulator => {
                    let reg = self.push_register();
                    self.write_op_store_register(reg, line);
                    match self.evaluate_expr(right)? {
                        ExprResult::Register(r) => {
                            self.pop_last_op();
                            self.pop_register();
                            self.$register_inst(r, line);
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Accumulator => {
                            self.$register_inst(reg, line);
                            self.pop_register();

                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Int(i) => {
                            self.pop_last_op();
                            self.pop_register();
                            self.$int_inst(i, line);
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Float(f) => {
                            self.write_value(Value::from_f64(f), line)?;
                            self.$register_inst(reg, line);
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                    }
                }
                ExprResult::Int(i) => match self.evaluate_expr(right)? {
                    ExprResult::Register(r) => {
                        self.write_op_load_register(r, line);
                        self.$int_inst(i, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$int_inst(i, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i2) => Ok(ExprResult::Int(i.$op_fn(i2)?)),
                    ExprResult::Float(f) => Ok(ExprResult::Float(f64::from(i).$op_fn(f))),
                },
                ExprResult::Float(f) => match self.evaluate_expr(right)? {
                    ExprResult::Register(r) => {
                        self.write_value(Value::from_f64(f), line)?;
                        self.$register_inst(r, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        let reg = self.push_register();
                        self.write_op_store_register(reg, line);
                        self.write_value(Value::from_f64(f), line)?;
                        self.$register_inst(reg, line);
                        self.pop_register();
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i2) => Ok(ExprResult::Float(f64::from(i2).$op_fn(f))),
                    ExprResult::Float(f2) => Ok(ExprResult::Float(f.$op_fn(f2))),
                },
            }
        }
    };
}

macro_rules! binary_op_non_commutative {
    ($op:ident,$register_inst:ident,$int_inst:ident,$op_fn:ident) => {
        fn $op(
            &mut self,
            left: &Expr,
            right: &Expr,
            line: u32,
        ) -> Result<ExprResult, BytecodeCompilerError> {
            match self.evaluate_expr(left)? {
                ExprResult::Register(r1) => match self.evaluate_expr(right)? {
                    ExprResult::Register(r2) => {
                        self.write_op_load_register(r2, line);
                        self.$register_inst(r1, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$register_inst(r1, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i) => {
                        self.write_op_load_register(r1, line);
                        self.$int_inst(i, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Float(f) => {
                        self.write_value(Value::from_f64(f), line)?;
                        self.$register_inst(r1, line);
                        Ok(ExprResult::Accumulator)
                    }
                },
                ExprResult::Accumulator => {
                    let reg = self.push_register();
                    self.write_op_store_register(reg, line);
                    match self.evaluate_expr(right)? {
                        ExprResult::Register(r) => {
                            self.write_op_load_register(r, line);
                            self.$register_inst(reg, line);
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Accumulator => {
                            self.$register_inst(reg, line);
                            self.pop_register();

                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Int(i) => {
                            self.pop_last_op();
                            self.pop_register();
                            self.$int_inst(i, line);
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Float(f) => {
                            self.write_value(Value::from_f64(f), line)?;
                            self.$register_inst(reg, line);
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                    }
                }
                ExprResult::Int(i) => match self.evaluate_expr(right)? {
                    ExprResult::Register(r) => {
                        self.write_op_load_register(r, line);
                        self.$int_inst(i, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$int_inst(i, line);
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i2) => Ok(ExprResult::Int(i.$op_fn(i2)?)),
                    ExprResult::Float(f) => Ok(ExprResult::Float(f64::from(i).$op_fn(f))),
                },
                ExprResult::Float(f) => {
                    self.write_value(Value::from_f64(f), line)?;
                    let reg = self.push_register();
                    self.write_op_store_register(reg, line);
                    match self.evaluate_expr(right)? {
                        ExprResult::Register(r) => {
                            self.write_op_load_register(r, line);
                            self.$register_inst(reg, line);
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Accumulator => {
                            self.$register_inst(reg, line);
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Int(i2) => {
                            self.pop_last_op();
                            self.pop_register();
                            Ok(ExprResult::Float(f64::from(i2).$op_fn(f)))
                        }
                        ExprResult::Float(f2) => {
                            self.pop_last_op();
                            self.pop_register();
                            Ok(ExprResult::Float(f.$op_fn(f2)))
                        }
                    }
                }
            }
        }
    };
}
impl BytecodeWriter {
    pub fn resolve_variable(&self, name: &str) -> Option<u16> {
        for (index, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(index as u16);
            }
        }
        None
    }

    pub fn evaluate_statements(
        &mut self,
        statements: &[Statement],
    ) -> Result<(), BytecodeCompilerError> {
        Ok(for statement in statements {
            self.evaluate_statement(statement)?
        })
    }

    pub fn evaluate_statement(
        &mut self,
        statement: &Statement,
    ) -> Result<(), BytecodeCompilerError> {
        match statement {
            Statement::Expr(expr) => self.evaluate_expr(expr).map(|e| ()),
            Statement::VarDeclaration {
                name,
                mutability,
                expr,
                line,
            } => Ok({
                let reg = u16::try_from(self.locals.len()).map_err(|e| ExceededMaxLocals)?;
                self.locals.push(Local {
                    name: name.to_string(),
                });
                self.push_register();
                match self.evaluate_expr(expr)? {
                    ExprResult::Register(reg2) => self.write_op_move(reg, reg2, *line),
                    ExprResult::Accumulator => self.write_op_store_register(reg, *line),
                    ExprResult::Int(i) => {
                        self.write_op_load_int(i, *line);
                        self.write_op_store_register(reg, *line)
                    }
                    ExprResult::Float(f) => {
                        self.write_value(Value::from_f64(f), *line)?;
                        self.write_op_store_register(reg, *line)
                    }
                }
            }),
        }
    }
    pub fn evaluate_expr(&mut self, expr: &Expr) -> Result<ExprResult, BytecodeCompilerError> {
        match expr {
            Expr::Literal { inner, line } => match inner {
                TokenType::IntLiteral(i) => Ok(ExprResult::Int(Int::from(*i))),
                TokenType::FloatLiteral(f) => Ok(ExprResult::Float(*f)),
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
                _ => todo!(),
            },
            Expr::Unary { op, right, line } => match op {
                TokenType::Minus => self.negate(right, *line),
                _ => todo!(),
            },
            Expr::Variable { name, line } => match self.resolve_variable(name) {
                Some(index) => Ok(ExprResult::Register(index)),
                None => todo!(),
            },
        }
    }
    fn negate(&mut self, right: &Expr, line: u32) -> Result<ExprResult, BytecodeCompilerError> {
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
            ExprResult::Int(i) => Ok(ExprResult::Int(i.neg()?)),
            ExprResult::Float(f) => Ok(ExprResult::Float(-f)),
        }
    }

    binary_op!(add, write_op_add_register, write_op_add_int, add);
    binary_op_non_commutative!(
        subtract,
        write_op_subtract_register,
        write_op_subtract_int,
        sub
    );
    binary_op!(
        multiply,
        write_op_multiply_register,
        write_op_multiply_int,
        mul
    );
    binary_op_non_commutative!(divide, write_op_divide_register, write_op_divide_int, div);
}

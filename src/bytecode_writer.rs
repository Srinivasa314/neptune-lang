use crate::bytecode::{BytecodeMaxSizeExceeded, BytecodeWriter};
use crate::{bytecode::ConstantInsertionError, parser::Expr, scanner::TokenType, value::Value};
use std::convert::TryFrom;
use std::ops::{Add, Div, Mul, Sub};
pub struct ArithmeticError;
#[derive(Clone, Copy, Debug)]
pub enum Int {
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
#[derive(Debug)]
pub enum BytecodeWriterError {
    BytecodeMaxSizeExceeded,
    ArithmeticError,
    ConstantInsertionError,
}
impl From<ArithmeticError> for BytecodeWriterError {
    fn from(_: ArithmeticError) -> Self {
        BytecodeWriterError::ArithmeticError
    }
}

impl From<BytecodeMaxSizeExceeded> for BytecodeWriterError {
    fn from(_: BytecodeMaxSizeExceeded) -> Self {
        BytecodeWriterError::BytecodeMaxSizeExceeded
    }
}

impl From<ConstantInsertionError> for BytecodeWriterError {
    fn from(_: ConstantInsertionError) -> Self {
        BytecodeWriterError::ConstantInsertionError
    }
}

macro_rules! binary_op {
    ($op:ident,$register_inst:ident,$int_inst:ident,$op_fn:ident) => {
        fn $op(&mut self, left: &Expr, right: &Expr) -> Result<ExprResult, BytecodeWriterError> {
            match self.evaluate(left)? {
                ExprResult::Register(r1) => match self.evaluate(right)? {
                    ExprResult::Register(r2) => {
                        self.write_op_load_register(r1)?;
                        self.$register_inst(r2)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$register_inst(r1)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i) => {
                        self.write_op_load_register(r1)?;
                        self.$int_inst(i)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Float(f) => {
                        self.write_value(Value::from_f64(f))?;
                        self.$register_inst(r1)?;
                        Ok(ExprResult::Accumulator)
                    }
                },
                ExprResult::Accumulator => {
                    let reg = self.push_register();
                    self.write_op_store_register(reg)?;
                    match self.evaluate(right)? {
                        ExprResult::Register(r) => {
                            self.pop_last_op();
                            self.pop_register();
                            self.$register_inst(r)?;
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Accumulator => {
                            self.$register_inst(reg)?;
                            self.pop_register();

                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Int(i) => {
                            self.pop_last_op();
                            self.pop_register();
                            self.$int_inst(i)?;
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Float(f) => {
                            self.write_value(Value::from_f64(f))?;
                            self.$register_inst(reg)?;
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                    }
                }
                ExprResult::Int(i) => match self.evaluate(right)? {
                    ExprResult::Register(r) => {
                        self.write_op_load_register(r)?;
                        self.$int_inst(i)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$int_inst(i)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i2) => Ok(ExprResult::Int(i.$op_fn(i2)?)),
                    ExprResult::Float(f) => Ok(ExprResult::Float(f64::from(i).$op_fn(f))),
                },
                ExprResult::Float(f) => match self.evaluate(right)? {
                    ExprResult::Register(r) => {
                        self.write_value(Value::from_f64(f))?;
                        self.$register_inst(r)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        let reg = self.push_register();
                        self.write_op_store_register(reg)?;
                        self.write_value(Value::from_f64(f))?;
                        self.$register_inst(reg)?;
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
        fn $op(&mut self, left: &Expr, right: &Expr) -> Result<ExprResult, BytecodeWriterError> {
            match self.evaluate(left)? {
                ExprResult::Register(r1) => match self.evaluate(right)? {
                    ExprResult::Register(r2) => {
                        self.write_op_load_register(r2)?;
                        self.$register_inst(r1)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$register_inst(r1)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i) => {
                        self.write_op_load_register(r1)?;
                        self.$int_inst(i)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Float(f) => {
                        self.write_value(Value::from_f64(f))?;
                        self.$register_inst(r1)?;
                        Ok(ExprResult::Accumulator)
                    }
                },
                ExprResult::Accumulator => {
                    let reg = self.push_register();
                    self.write_op_store_register(reg)?;
                    match self.evaluate(right)? {
                        ExprResult::Register(r) => {
                            self.write_op_load_register(r)?;
                            self.$register_inst(reg)?;
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Accumulator => {
                            self.$register_inst(reg)?;
                            self.pop_register();

                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Int(i) => {
                            self.pop_last_op();
                            self.pop_register();
                            self.$int_inst(i)?;
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Float(f) => {
                            self.write_value(Value::from_f64(f))?;
                            self.$register_inst(reg)?;
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                    }
                }
                ExprResult::Int(i) => match self.evaluate(right)? {
                    ExprResult::Register(r) => {
                        self.write_op_load_register(r)?;
                        self.$int_inst(i)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Accumulator => {
                        self.$int_inst(i)?;
                        Ok(ExprResult::Accumulator)
                    }
                    ExprResult::Int(i2) => Ok(ExprResult::Int(i.$op_fn(i2)?)),
                    ExprResult::Float(f) => Ok(ExprResult::Float(f64::from(i).$op_fn(f))),
                },
                ExprResult::Float(f) => {
                    self.write_value(Value::from_f64(f))?;
                    let reg = self.push_register();
                    self.write_op_store_register(reg)?;
                    match self.evaluate(right)? {
                        ExprResult::Register(r) => {
                            self.write_op_load_register(r)?;
                            self.$register_inst(reg)?;
                            self.pop_register();
                            Ok(ExprResult::Accumulator)
                        }
                        ExprResult::Accumulator => {
                            self.$register_inst(reg)?;
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
    pub fn evaluate(&mut self, expr: &Expr) -> Result<ExprResult, BytecodeWriterError> {
        match expr {
            Expr::Literal(l) => match l {
                TokenType::IntLiteral(i) => Ok(ExprResult::Int(Int::from(*i))),
                TokenType::FloatLiteral(f) => Ok(ExprResult::Float(*f)),
                _ => todo!(),
            },
            Expr::Binary { left, op, right } => match op {
                TokenType::Plus => self.add(left, right),
                TokenType::Minus => self.subtract(left, right),
                TokenType::Star => self.multiply(left, right),
                TokenType::Slash => self.divide(left, right),
                _ => todo!(),
            },
            Expr::Unary { op, right } => match op {
                TokenType::Minus => self.negate(right),
                _ => todo!(),
            },
        }
    }
    pub fn negate(&mut self, right: &Expr) -> Result<ExprResult, BytecodeWriterError> {
        match self.evaluate(right)? {
            ExprResult::Register(r) => {
                self.write_op_load_register(r)?;
                self.write_op_negate()?;
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Accumulator => {
                self.write_op_negate()?;
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Int(i) => Ok(ExprResult::Int(i.neg()?)),
            ExprResult::Float(f) => Ok(ExprResult::Float(f)),
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

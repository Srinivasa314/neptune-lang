use std::{
    cell::Cell,
    ops::{Add, Div, Mul, Sub},
};

use arrayvec::ArrayVec;

use crate::{
    bytecode::{Bytecode, BytecodeReader, Op},
    gc::{self, GCSession, GC},
    value::{ArithmeticError, RootedValue, Value},
};

struct VM<'gc> {
    acc: Cell<RootedValue<'gc>>,
    stack: Vec<RootedValue<'gc>>,
    bp: *mut RootedValue<'gc>,
    stack_end: *mut RootedValue<'gc>,
    frames: ArrayVec<Frame<'gc>, 1024>,
    globals: Vec<Cell<RootedValue<'gc>>>,
    gc: GCSession<'gc>,
}

struct StackOverflowError;

struct Frame<'gc> {
    reader: BytecodeReader<'gc>,
    bp: *mut RootedValue<'gc>,
}

macro_rules! binary_int_op {
    ($self:ident,$br:ident,$op:ident,$op_name:ident,$type:ty) => {{
        let a = $self.geta();
        if let Some(i) = a.as_i32() {
            $self.seta(Value::from_i32(i.$op($br.read::<$type>() as i32)))
        } else if let Some(f) = a.as_f64() {
            $self.seta(Value::from_f64(f.$op($br.read::<$type>() as f64)))
        } else {
            return Err(format!(
                "Cannot {} types {} and int",
                stringify!(op_name),
                a.type_string()
            ));
        }
    }};
}

macro_rules! binary_reg_op {
    ($self:ident,$br:ident,$op:ident,$op_name:ident,$type:ty) => {{
        let left = $self.getr($br.read::<$type>() as u16);
        let right = $self.geta();
        match left.$op(right) {
            Ok(v) => $self.seta(v),
            Err(ArithmeticError::TypeError) => {
                return Err(format!(
                    "Cannot {} types {} and {}",
                    stringify!(op_name),
                    left.type_string(),
                    right.type_string()
                ))
            }
            Err(ArithmeticError::OverflowError) => {
                return Err(format!(
                    "Overflow when doing operation {} on {} and {}",
                    stringify!($op_name),
                    left,
                    right
                ))
            }
        }
    }};
}

impl<'gc> VM<'gc> {
    pub fn new(gc: &'gc GC) -> Self {
        let mut stack = vec![RootedValue::from(Value::null()); 1024 * 128];
        let bp = stack.as_mut_ptr();
        let stack_end = unsafe { bp.add(stack.len()) };
        Self {
            acc: Cell::new(RootedValue::from(Value::null())),
            stack,
            bp,
            stack_end,
            frames: ArrayVec::new(),
            globals: vec![],
            gc: GCSession::new(gc),
        }
    }

    // The Values returned by the following functions have reduced lifetime
    // as they are unrooted and do not survive garbage collection. Allocation which
    // triggers garbage collection borrows the VM as mutable
    fn getr<'a>(&'a self, index: u16) -> Value<'a> {
        let ptr = unsafe { self.bp.add(index as usize) };
        debug_assert!(ptr < self.stack_end);
        unsafe { ptr.read().get_inner() }
    }

    fn setr<'a, 'b>(&'a self, index: u16, v: Value<'b>) {
        let ptr = unsafe { self.bp.add(index as usize) };
        debug_assert!(ptr < self.stack_end);
        unsafe { ptr.write(RootedValue::from(v)) }
    }

    fn geta<'a>(&'a self) -> Value<'a> {
        unsafe { self.acc.get().get_inner() }
    }

    fn seta<'a, 'b>(&'a self, v: Value<'b>) {
        self.acc.set(RootedValue::from(v))
    }

    fn get_global<'a>(&'a self, index: u32) -> Option<Value<'a>> {
        debug_assert!((index as usize) < self.globals.len());
        let v = unsafe { self.globals.get_unchecked(index as usize).get().get_inner() };
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    fn set_global<'a, 'b>(&'a self, index: u32, v: Value<'b>) {
        debug_assert!((index as usize) < self.globals.len());
        unsafe {
            self.globals
                .get_unchecked(index as usize)
                .set(RootedValue::from(v))
        };
    }

    fn extend_bp(&mut self, by: u16, regcount: u16) -> Result<(), StackOverflowError> {
        let p = self.bp.wrapping_add(by as usize);
        if p.wrapping_add(regcount as usize) > self.stack_end {
            Err(StackOverflowError)
        } else {
            self.bp = p;
            Ok(())
        }
    }

    //TODO: Return uncaught exception in future
    pub fn run(&mut self, bc: &'gc Bytecode<'gc>) -> Result<(), String> {
        let mut br = BytecodeReader::new(bc);
        self.frames.push(Frame {
            reader: br,
            bp: self.bp,
        });
        unsafe {
            loop {
                match br.read_op() {
                    Op::Wide => todo!(),
                    Op::ExtraWide => todo!(),
                    Op::LoadRegister => self.seta(self.getr(br.read_u8() as u16)),
                    Op::LoadInt => self.seta(Value::from_i32(br.read_i8() as i32)),
                    Op::LoadConstant => self.seta(br.read_value::<u8>()),
                    Op::StoreRegister => self.setr(br.read_u8() as u16, self.geta()),
                    Op::Move => {
                        let left = br.read_u8();
                        let right = br.read_u8();
                        self.setr(left as u16, self.getr(right as u16))
                    }
                    Op::LoadGlobal => self.seta(
                        self.get_global(br.read_u8() as u32)
                            .ok_or_else(|| "todo".to_string())?,
                    ),
                    Op::StoreGlobal => self.set_global(br.read_u8() as u32, self.geta()),
                    Op::AddRegister => binary_reg_op!(self, br, add, add, u8),
                    Op::SubtractRegister => binary_reg_op!(self, br, sub, subtract, u8),
                    Op::MultiplyRegister => binary_reg_op!(self, br, mul, multiply, u8),
                    Op::DivideRegister => binary_reg_op!(self, br, div, divide, u8),
                    Op::AddInt => binary_int_op!(self, br, add, add, i8),
                    Op::SubtractInt => binary_int_op!(self, br, sub, subtract, i8),
                    Op::MultiplyInt => binary_int_op!(self, br, mul, multiply, i8),
                    Op::DivideInt => binary_int_op!(self, br, div, divide, i8),
                    Op::Increment => todo!(),
                    Op::Negate => todo!(),
                    Op::Call => todo!(),
                    Op::Call0Argument => todo!(),
                    Op::Call1Argument => todo!(),
                    Op::Call2Argument => todo!(),
                    Op::Less => todo!(),
                    Op::Jump => todo!(),
                    Op::JumpBack => todo!(),
                    Op::JumpIfFalse => todo!(),
                    Op::Return => todo!(),
                    Op::Exit => return Ok(()),
                    Op::StoreR0 => self.setr(0, self.geta()),
                    Op::StoreR1 => self.setr(1, self.geta()),
                    Op::StoreR2 => self.setr(2, self.geta()),
                    Op::StoreR3 => self.setr(3, self.geta()),
                    Op::StoreR4 => self.setr(4, self.geta()),
                    Op::StoreR5 => self.setr(5, self.geta()),
                    Op::StoreR6 => self.setr(6, self.geta()),
                    Op::StoreR7 => self.setr(7, self.geta()),
                    Op::StoreR8 => self.setr(8, self.geta()),
                    Op::StoreR9 => self.setr(9, self.geta()),
                    Op::StoreR10 => self.setr(10, self.geta()),
                    Op::StoreR11 => self.setr(11, self.geta()),
                    Op::StoreR12 => self.setr(12, self.geta()),
                    Op::StoreR13 => self.setr(13, self.geta()),
                    Op::StoreR14 => self.setr(14, self.geta()),
                    Op::StoreR15 => self.setr(15, self.geta()),
                }
            }
        }
    }
}

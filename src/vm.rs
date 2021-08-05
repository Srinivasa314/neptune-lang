use std::{
    cell::Cell,
    fmt::Write,
    ops::{Add, Div, Mul, Sub},
};

use arrayvec::ArrayVec;

use crate::{
    bytecode::{Bytecode, BytecodeReader, Op},
    gc::{self, GCSession, ObjectTrait, Root, GC},
    objects::NString,
    util::unreachable,
    value::{ArithmeticError, RootedValue, Value},
};

struct VM<'gc> {
    acc: Cell<RootedValue<'gc>>,
    stack: Vec<RootedValue<'gc>>,
    bp: *mut RootedValue<'gc>,
    stack_end: *mut RootedValue<'gc>,
    frames: ArrayVec<Frame<'gc>, 1024>,
    globals: Vec<(Cell<RootedValue<'gc>>, String)>,
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
            let ts = a.type_string();
            $self.throw(
                format!("Cannot {} types {} and int", stringify!(op_name), ts),
                &mut $br,
            )?;
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
                let (lts, rts) = (left.type_string(), right.type_string());
                $self.throw(
                    format!("Cannot {} types {} and {}", stringify!(op_name), lts, rts),
                    &mut $br,
                )?;
            }
            Err(ArithmeticError::OverflowError) => {
                let err_msg = format!(
                    "Overflow when doing operation {} on {} and {}",
                    stringify!($op_name),
                    left,
                    right
                );
                $self.throw(err_msg, &mut $br)?;
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
    // triggers garbage collection borrows the VM as mutable. The functions below that are
    // unsafe do not do bounds checking, other than that they are safe.
    unsafe fn getr<'a>(&'a self, index: u16) -> Value<'a> {
        let ptr = unsafe { self.bp.add(index as usize) };
        debug_assert!(ptr < self.stack_end);
        ptr.read().get_inner()
    }

    unsafe fn setr<'a, 'b>(&'a self, index: u16, v: Value<'b>) {
        let ptr = unsafe { self.bp.add(index as usize) };
        debug_assert!(ptr < self.stack_end);
        ptr.write(RootedValue::from(v))
    }

    fn geta<'a>(&'a self) -> Value<'a> {
        unsafe { self.acc.get().get_inner() }
    }

    fn seta<'a, 'b>(&'a self, v: Value<'b>) {
        self.acc.set(RootedValue::from(v))
    }

    unsafe fn get_global<'a>(&'a self, index: u32) -> Option<Value<'a>> {
        debug_assert!((index as usize) < self.globals.len());
        let v = self
            .globals
            .get_unchecked(index as usize)
            .0
            .get()
            .get_inner();
        if v.is_empty() {
            None
        } else {
            Some(v)
        }
    }

    unsafe fn get_global_name(&self, index: u32) -> &str {
        debug_assert!((index as usize) < self.globals.len());
        &self.globals.get_unchecked(index as usize).1
    }

    unsafe fn set_global<'a, 'b>(&'a self, index: u32, v: Value<'b>) {
        debug_assert!((index as usize) < self.globals.len());

        self.globals
            .get_unchecked(index as usize)
            .0
            .set(RootedValue::from(v))
    }

    fn allocate<T: ObjectTrait>(&mut self, t: T) {
        self.acc.set(self.gc.allocate(t, self))
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

    // Throws an exception. Returns Err if there are no exception handlers.
    // Todo: return Value and do unwinding.
    #[cold]
    #[inline(never)]
    fn throw(&mut self, s: String, br: &mut BytecodeReader) -> Result<(), String> {
        Err(s)
    }

    //TODO: Return uncaught exception in future
    pub fn run(&mut self, bc: &'gc Bytecode<'gc>) -> Result<(), String> {
        let mut br = BytecodeReader::new(bc);
        unsafe {
            loop {
                match br.read_op() {
                    Op::Wide => match br.read_op() {
                        Op::LoadRegister => self.seta(self.getr(br.read_u16())),
                        Op::LoadInt => self.seta(Value::from_i32(br.read_i16() as i32)),
                        Op::LoadConstant => self.seta(br.read_value::<u16>()),
                        Op::StoreRegister => self.setr(br.read_u16(), self.geta()),
                        Op::Move => {
                            let left = br.read_u16();
                            let right = br.read_u16();
                            self.setr(left, self.getr(right))
                        }
                        Op::LoadGlobal => {
                            let g = br.read_u16() as u32;
                            match self.get_global(g) {
                                Some(v) => self.seta(v),
                                None => {
                                    self.throw(
                                        format!("{} is not defined", self.get_global_name(g)),
                                        &mut br,
                                    )?;
                                }
                            }
                        }
                        Op::StoreGlobal => self.set_global(br.read_u16() as u32, self.geta()),
                        Op::AddRegister => binary_reg_op!(self, br, add, add, u16),
                        Op::SubtractRegister => binary_reg_op!(self, br, sub, subtract, u16),
                        Op::MultiplyRegister => binary_reg_op!(self, br, mul, multiply, u16),
                        Op::DivideRegister => binary_reg_op!(self, br, div, divide, u16),
                        Op::ConcatRegister => todo!(),
                        Op::AddInt => binary_int_op!(self, br, add, add, i16),
                        Op::SubtractInt => binary_int_op!(self, br, sub, subtract, i16),
                        Op::MultiplyInt => binary_int_op!(self, br, mul, multiply, i16),
                        Op::DivideInt => binary_int_op!(self, br, div, divide, i16),
                        Op::Call => todo!(),
                        Op::Call0Argument => todo!(),
                        Op::Call1Argument => todo!(),
                        Op::Call2Argument => todo!(),
                        Op::Less => todo!(),
                        Op::Jump => todo!(),
                        Op::JumpBack => todo!(),
                        Op::JumpIfFalse => todo!(),
                        _ => unreachable(),
                    },
                    Op::ExtraWide => match br.read_op() {
                        Op::LoadInt => self.seta(Value::from_i32(br.read_i32())),
                        Op::LoadGlobal => {
                            let g = br.read_u32();
                            match self.get_global(g) {
                                Some(v) => self.seta(v),
                                None => {
                                    self.throw(
                                        format!("{} is not defined", self.get_global_name(g)),
                                        &mut br,
                                    )?;
                                }
                            }
                        }
                        Op::StoreGlobal => self.set_global(br.read_u32(), self.geta()),
                        Op::AddInt => binary_int_op!(self, br, add, add, i32),
                        Op::SubtractInt => binary_int_op!(self, br, sub, subtract, i32),
                        Op::MultiplyInt => binary_int_op!(self, br, mul, multiply, i32),
                        Op::DivideInt => binary_int_op!(self, br, div, divide, i32),
                        _ => unreachable(),
                    },
                    Op::LoadRegister => self.seta(self.getr(br.read_u8() as u16)),
                    Op::LoadInt => self.seta(Value::from_i32(br.read_i8() as i32)),
                    Op::LoadNull => self.seta(Value::null()),
                    Op::LoadTrue => self.seta(Value::new_true()),
                    Op::LoadFalse => self.seta(Value::new_false()),
                    Op::LoadConstant => self.seta(br.read_value::<u8>()),
                    Op::StoreRegister => self.setr(br.read_u8() as u16, self.geta()),
                    Op::Move => {
                        let left = br.read_u8();
                        let right = br.read_u8();
                        self.setr(left as u16, self.getr(right as u16))
                    }
                    Op::LoadGlobal => {
                        let g = br.read_u8() as u32;
                        match self.get_global(g) {
                            Some(v) => self.seta(v),
                            None => {
                                self.throw(
                                    format!("{} is not defined", self.get_global_name(g)),
                                    &mut br,
                                )?;
                            }
                        }
                    }
                    Op::StoreGlobal => self.set_global(br.read_u8() as u32, self.geta()),
                    Op::AddRegister => binary_reg_op!(self, br, add, add, u8),
                    Op::SubtractRegister => binary_reg_op!(self, br, sub, subtract, u8),
                    Op::MultiplyRegister => binary_reg_op!(self, br, mul, multiply, u8),
                    Op::DivideRegister => binary_reg_op!(self, br, div, divide, u8),
                    Op::ConcatRegister => {
                        let left = self.getr(br.read_u8() as u16);
                        let right = self.geta();
                        match (left.as_object(), right.as_object()) {
                            (Some(o1), Some(o2)) => {
                                match (o1.cast::<NString>(), o2.cast::<NString>()) {
                                    (Some(s1), Some(s2)) => {
                                        let result = s1.clone() + s2;
                                        self.allocate(result);
                                    }
                                    _ => {
                                        let msg = format!(
                                            "Cannot concat types {} and {}",
                                            left.type_string(),
                                            right.type_string()
                                        );
                                        self.throw(msg, &mut br)?;
                                    }
                                }
                            }
                            _ => {
                                let msg = format!(
                                    "Cannot concat types {} and {}",
                                    left.type_string(),
                                    right.type_string()
                                );
                                self.throw(msg, &mut br)?;
                            }
                        }
                    }
                    Op::AddInt => binary_int_op!(self, br, add, add, i8),
                    Op::SubtractInt => binary_int_op!(self, br, sub, subtract, i8),
                    Op::MultiplyInt => binary_int_op!(self, br, mul, multiply, i8),
                    Op::DivideInt => binary_int_op!(self, br, div, divide, i8),
                    Op::Negate => {
                        let a = self.geta();
                        if let Some(i) = a.as_i32() {
                            if let Some(res) = i.checked_neg() {
                                self.seta(Value::from_i32(res))
                            } else {
                                self.throw(format!("Overflow on negating {}", i), &mut br)?;
                            }
                        } else if let Some(f) = a.as_f64() {
                            self.seta(Value::from_f64(-f))
                        } else {
                            let ts = a.type_string();
                            self.throw(format!("Cannot negate type {}", ts), &mut br)?;
                        }
                    }
                    Op::Call => todo!(),
                    Op::Call0Argument => todo!(),
                    Op::Call1Argument => todo!(),
                    Op::Call2Argument => todo!(),
                    Op::Less => todo!(),
                    Op::ToString => {
                        let a = self.geta();
                        if let Some(o) = a.as_object() {
                            if !o.is::<NString>() {
                                let mut s = NString::new();
                                let res = write!(s, "{}", o);
                                debug_assert!(res.is_ok(), "It should never fail");
                                self.allocate(s)
                            }
                        } else {
                            let mut s = NString::new();
                            let res = write!(s, "{}", a);
                            debug_assert!(res.is_ok(), "It should never fail");
                            self.allocate(s)
                        }
                    }
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

unsafe impl<'gc> Root for VM<'gc> {}

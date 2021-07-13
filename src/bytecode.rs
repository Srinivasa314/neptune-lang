use crate::value::Value;
use num_enum::TryFromPrimitive;
use std::fmt;
use std::{convert::TryInto, marker::PhantomData};
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
#[repr(u8)]
pub enum Op {
    Wide,
    ExtraWide,
    LoadInt,
    LoadRegister,
    StoreR0,
    StoreR1,
    StoreR2,
    StoreR3,
    StoreR4,
    Move,
    Increment,
    Negate,
    AddRegister,
    AddInt,
    SubtractRegister,
    SubtractInt,
    MultiplyRegister,
    MultiplyInt,
    DivideRegister,
    DivideInt,
    ModInt,
    Less,
    LoadConstant,
    Print, //TODO: remove
    Return,
    Jump,
    JumpBack,
    JumpIfFalse,
    Call1Argument,
    GetGlobal,
    Exit,
}

#[derive(Default)]
pub struct Bytecode {
    pub inner: Vec<u8>,
    pub constants: Vec<Value<'static>>,
}

pub struct ExceededMaxConstants;
#[derive(Debug)]
pub struct BytecodeMaxSizeExceeded;

pub enum ConstantInsertionError {
    ExceededMaxConstants,
    BytecodeMaxSizeExceeded,
}

impl From<BytecodeMaxSizeExceeded> for ConstantInsertionError {
    fn from(_: BytecodeMaxSizeExceeded) -> Self {
        Self::BytecodeMaxSizeExceeded
    }
}

pub struct BytecodeWriter {
    b: Bytecode,
    last_op: Vec<usize>,
    no_registers: u16,
}

impl Default for BytecodeWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeWriter {
    pub fn new() -> Self {
        Self {
            b: Bytecode::default(),
            last_op: vec![],
            no_registers: 0,
        }
    }

    pub fn get_no_registers(&self) -> u16 {
        self.no_registers
    }

    pub fn push_register(&mut self) -> u16 {
        self.no_registers += 1;
        self.no_registers - 1
    }

    pub fn pop_register(&mut self) {
        self.no_registers -= 1;
    }

    pub fn pop_last_op(&mut self) {
        self.last_op.pop();
        self.b.inner.resize(*self.last_op.last().unwrap(), 0);
    }

    pub fn write_op(&mut self, op: Op) -> Result<(), BytecodeMaxSizeExceeded> {
        self.last_op.push(self.b.inner.len());
        self.write_u8(op as u8)
    }
    pub fn write_u8(&mut self, u: u8) -> Result<(), BytecodeMaxSizeExceeded> {
        if self.b.inner.len() == (1 << 16) {
            Err(BytecodeMaxSizeExceeded)
        } else {
            self.b.inner.push(u);
            Ok(())
        }
    }

    pub fn write_u16(&mut self, u: u16) -> Result<(), BytecodeMaxSizeExceeded> {
        for byte in u.to_ne_bytes() {
            self.write_u8(byte)?
        }
        Ok(())
    }

    pub fn write_u32(&mut self, u: u32) -> Result<(), BytecodeMaxSizeExceeded> {
        for byte in u.to_ne_bytes() {
            self.write_u8(byte)?
        }
        Ok(())
    }

    pub fn write_i8(&mut self, i: i8) -> Result<(), BytecodeMaxSizeExceeded> {
        if self.b.inner.len() == (1 << 16) {
            Err(BytecodeMaxSizeExceeded)
        } else {
            self.b.inner.push(i as u8);
            Ok(())
        }
    }

    pub fn write_i16(&mut self, i: i16) -> Result<(), BytecodeMaxSizeExceeded> {
        for byte in i.to_ne_bytes() {
            self.write_u8(byte)?
        }
        Ok(())
    }

    pub fn write_i32(&mut self, i: i32) -> Result<(), BytecodeMaxSizeExceeded> {
        for byte in i.to_ne_bytes() {
            self.write_u8(byte)?
        }
        Ok(())
    }

    pub fn patch_jump(&mut self, bytecode_index: u16, offset: u16) {
        self.b.inner[bytecode_index as usize] = offset as u8;
        self.b.inner[(bytecode_index + 1) as usize] = (offset >> 8) as u8;
    }

    // The lifetime is static as it should be in the constant table
    pub fn write_value(&mut self, v: Value<'static>) -> Result<(), ConstantInsertionError> {
        self.write_op(Op::LoadConstant)?;
        for (i, constant) in self.b.constants.iter().enumerate() {
            if *constant == v {
                self.write_u16(i as u16)?;
                return Ok(());
            }
        }
        if self.b.constants.len() == 1 << 16 {
            Err(ConstantInsertionError::ExceededMaxConstants)
        } else {
            self.b.constants.push(v);
            self.write_u16((self.b.constants.len() - 1) as u16)?;
            Ok(())
        }
    }

    pub fn last_op(&self) -> Option<Op> {
        self.last_op
            .get(0)
            .cloned()
            .map(|lo| self.b.inner[lo].try_into().unwrap())
    }

    pub fn set_last_op(&mut self, op: Op) {
        if let Some(lo) = self.last_op.last().cloned() {
            self.b.inner[lo] = op as u8;
        } else {
            panic!("Last op doesnt exist!")
        }
    }

    pub fn get_jmp_index(&self) -> Option<u16> {
        self.last_op.last().cloned().map(|lo| (lo + 1) as u16)
    }

    pub fn get_op_index(&self) -> Option<u16> {
        self.last_op.last().cloned().map(|lo| lo as u16)
    }

    pub fn bytecode(self) -> Bytecode {
        self.b
    }
}

#[derive(Clone, Copy)]
pub struct BytecodeReader<'a> {
    ptr: *const u8,
    start: *const u8,
    end: *const u8,
    constants: &'a [Value<'static>],
    _marker: PhantomData<&'a [u8]>,
}

impl fmt::Debug for Bytecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut reader = BytecodeReader::new(self);
        unsafe {
            Ok(while !reader.is_at_end() {
                write!(f, "{}: ", reader.offset())?;
                match reader.read_op() {
                    Op::LoadInt => {
                        writeln!(f, "LoadI8 {}", reader.read_i8())?;
                    }
                    Op::LoadRegister => {
                        writeln!(f, "LoadRegister r{}", reader.read_u8())?;
                    }
                    Op::StoreR0 => {
                        writeln!(f, "StoreR0")?;
                    }
                    Op::StoreR1 => {
                        writeln!(f, "StoreR1")?;
                    }
                    Op::StoreR2 => {
                        writeln!(f, "StoreR2")?;
                    }
                    Op::StoreR3 => {
                        writeln!(f, "StoreR3")?;
                    }
                    Op::StoreR4 => {
                        writeln!(f, "StoreR4")?;
                    }
                    Op::Move => {
                        let dest = reader.read_u8();
                        let src = reader.read_u8();
                        writeln!(f, "Move r{},r{}", dest, src)?;
                    }
                    Op::Increment => {
                        writeln!(f, "Increment")?;
                    }
                    Op::SubtractInt => {
                        writeln!(f, "SubtractInt {}", reader.read_i8())?;
                    }
                    Op::ModInt => {
                        writeln!(f, "ModI8 {}", reader.read_i8())?;
                    }
                    Op::Less => {
                        writeln!(f, "Less r{}", reader.read_u8())?;
                    }
                    Op::LoadConstant => {
                        writeln!(f, "LoadConstant {}", reader.read_u16())?;
                    }
                    Op::AddRegister => {
                        writeln!(f, "AddRegister r{}", reader.read_u8())?;
                    }
                    Op::AddInt => {
                        writeln!(f, "AddInt {}", reader.read_i8())?;
                    }
                    Op::Print => {
                        writeln!(f, "Print")?;
                    }
                    Op::Return => {
                        writeln!(f, "Return")?;
                    }
                    Op::Jump => {
                        let offset = reader.read_u16();
                        writeln!(
                            f,
                            "Jump {} (to {})",
                            offset,
                            reader.offset() + (offset as isize)
                        )?;
                    }
                    Op::JumpBack => {
                        let offset = reader.read_u16();
                        writeln!(
                            f,
                            "JumpBack {} (to {})",
                            offset,
                            reader.offset() - (offset as isize)
                        )?;
                    }
                    Op::JumpIfFalse => {
                        let offset = reader.read_u16();
                        writeln!(
                            f,
                            "JumpIfFalse {} (to {})",
                            offset,
                            reader.offset() + (offset as isize)
                        )?;
                    }
                    Op::Call1Argument => {
                        let fun = reader.read_u8();
                        let arg0 = reader.read_u8();
                        writeln!(f, "Call1Argument r{} r{}", fun, arg0)?;
                    }
                    Op::GetGlobal => {
                        writeln!(f, "GetGlobal {}", reader.read_u8())?;
                    }
                    Op::Exit => {
                        writeln!(f, "Exit")?;
                    }
                    Op::Wide => todo!(),
                    Op::ExtraWide => todo!(),
                    Op::SubtractRegister => {
                        writeln!(f, "SubtractRegister r{}", reader.read_u8())?;
                    }
                    Op::MultiplyRegister => {
                        writeln!(f, "MultiplyRegister r{}", reader.read_u8())?;
                    }
                    Op::MultiplyInt => {
                        writeln!(f, "MultiplyInt {}", reader.read_i8())?;
                    }
                    Op::DivideRegister => {
                        writeln!(f, "DivideRegister r{}", reader.read_u8())?;
                    }
                    Op::DivideInt => {
                        writeln!(f, "Divide {}", reader.read_i8())?;
                    }
                    Op::Negate => writeln!(f, "Negate")?,
                }
            })
        }
    }
}

impl<'a> BytecodeReader<'a> {
    pub fn new(bytecode: &'a Bytecode) -> BytecodeReader<'a> {
        BytecodeReader {
            ptr: bytecode.inner.as_ptr(),
            constants: bytecode.constants.as_ref(),
            _marker: PhantomData,
            start: bytecode.inner.as_ptr(),
            end: unsafe { bytecode.inner.as_ptr().add(bytecode.inner.len()) },
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.ptr >= self.end
    }

    pub fn offset(&self) -> isize {
        unsafe { self.ptr.offset_from(self.start) }
    }

    // The below functions are unsafe as they require the caller to make sure that
    // the next item in the bytecode is what the caller wants to read and that
    // it is not at the end
    pub unsafe fn read_op(&mut self) -> Op {
        let op = self.ptr.cast::<Op>().read();
        self.ptr = self.ptr.add(1);
        op
    }
    pub unsafe fn read_u8(&mut self) -> u8 {
        let u = self.ptr.read();
        self.ptr = self.ptr.add(1);
        u
    }
    pub unsafe fn read_u16(&mut self) -> u16 {
        let ret = u16::from_ne_bytes([self.ptr.read(), self.ptr.add(1).read()]);
        self.ptr = self.ptr.add(2);
        ret
    }

    pub unsafe fn read_u32(&mut self) -> u32 {
        let ret = u32::from_ne_bytes([
            self.ptr.read(),
            self.ptr.add(1).read(),
            self.ptr.add(2).read(),
            self.ptr.add(3).read(),
        ]);
        self.ptr = self.ptr.add(4);
        ret
    }

    pub unsafe fn read_i8(&mut self) -> i8 {
        let u = self.ptr.read();
        self.ptr = self.ptr.add(1);
        u as i8
    }
    pub unsafe fn read_i16(&mut self) -> i16 {
        let ret = i16::from_ne_bytes([self.ptr.read(), self.ptr.add(1).read()]);
        self.ptr = self.ptr.add(2);
        ret
    }

    pub unsafe fn read_i32(&mut self) -> i32 {
        let ret = i32::from_ne_bytes([
            self.ptr.read(),
            self.ptr.add(1).read(),
            self.ptr.add(2).read(),
            self.ptr.add(3).read(),
        ]);
        self.ptr = self.ptr.add(4);
        ret
    }

    // The lifetime is static as it should be in the constant table
    pub unsafe fn read_value(&mut self) -> Value<'static> {
        *self.constants.get_unchecked(self.read_u16() as usize)
    }

    // The below functions are unsafe as they require the caller to give an offset
    // in bounds
    pub unsafe fn jump(&mut self, offset: u16) {
        self.ptr = self.ptr.add(offset as usize)
    }

    pub unsafe fn back_jump(&mut self, offset: u16) {
        self.ptr = self.ptr.sub(offset as usize)
    }
}

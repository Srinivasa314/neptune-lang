use crate::value::Value;
use num_enum::{TryFromPrimitive, UnsafeFromPrimitive};
use std::convert::TryFrom;
use std::fmt;
use std::marker::PhantomData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive, UnsafeFromPrimitive)]
#[repr(u8)]
pub enum Op {
    Wide,
    ExtraWide,
    LoadRegister,
    LoadInt,
    LoadNull,
    LoadTrue,
    LoadFalse,
    LoadConstant,
    StoreRegister,
    Move,
    LoadGlobal,
    StoreGlobal,
    AddRegister,
    SubtractRegister,
    MultiplyRegister,
    DivideRegister,
    AddInt,
    SubtractInt,
    MultiplyInt,
    DivideInt,
    Increment,
    Negate,
    Call,
    Call0Argument,
    Call1Argument,
    Call2Argument,
    Less,
    Jump,
    JumpBack,
    JumpIfFalse,
    Return,
    Exit,
    StoreR0,
    StoreR1,
    StoreR2,
    StoreR3,
    StoreR4,
    StoreR5,
    StoreR6,
    StoreR7,
    StoreR8,
    StoreR9,
    StoreR10,
    StoreR11,
    StoreR12,
    StoreR13,
    StoreR14,
    StoreR15,
    ToString,
}

#[derive(Default)]
pub struct Bytecode<'gc> {
    code: Vec<u8>,
    constants: Vec<Value<'gc>>,
    lines: Vec<LineStart>,
}

struct LineStart {
    offset: u32,
    line: u32,
}

pub struct ExceededMaxConstants;

#[derive(Default)]
pub struct BytecodeWriter<'gc> {
    b: Bytecode<'gc>,
    op_positions: Vec<usize>,
}

impl<'gc> BytecodeWriter<'gc> {
    pub fn new() -> Self {
        Self {
            b: Bytecode::default(),
            op_positions: vec![],
        }
    }

    pub fn pop_last_op(&mut self) {
        self.b.code.resize(*self.op_positions.last().unwrap(), 0);
        let pos = self.op_positions.pop().unwrap();
        if let Some(LineStart { offset, .. }) = self.b.lines.last() {
            if pos == *offset as usize {
                self.b.lines.pop();
            }
        }
    }

    pub fn write_op(&mut self, op: Op, line: u32) {
        self.op_positions.push(self.b.code.len());
        if let Some(LineStart {
            line: last_line, ..
        }) = self.b.lines.last()
        {
            if line != *last_line {
                self.b.lines.push(LineStart {
                    line,
                    offset: self.b.code.len() as u32,
                })
            }
        } else {
            self.b.lines.push(LineStart {
                line,
                offset: self.b.code.len() as u32,
            })
        }
        self.write_u8(op as u8)
    }

    pub fn write_u8(&mut self, u: u8) {
        self.b.code.push(u);
    }

    pub fn write_u16(&mut self, u: u16) {
        for byte in u.to_ne_bytes() {
            self.write_u8(byte)
        }
    }

    pub fn write_u32(&mut self, u: u32) {
        for byte in u.to_ne_bytes() {
            self.write_u8(byte)
        }
    }

    pub fn write_i8(&mut self, i: i8) {
        for byte in i.to_ne_bytes() {
            self.write_u8(byte)
        }
    }

    pub fn write_i16(&mut self, i: i16) {
        for byte in i.to_ne_bytes() {
            self.write_u8(byte)
        }
    }

    pub fn write_i32(&mut self, i: i32) {
        for byte in i.to_ne_bytes() {
            self.write_u8(byte)
        }
    }

    pub fn patch_jump(&mut self, bytecode_index: usize, offset: u16) {
        self.b.code[bytecode_index] = offset as u8;
        self.b.code[(bytecode_index + 1)] = (offset >> 8) as u8;
    }

    pub fn new_constant(&mut self, v: Value<'gc>) -> Result<u16, ExceededMaxConstants> {
        for (i, constant) in self.b.constants.iter().enumerate() {
            if *constant == v {
                return Ok(i as u16);
            }
        }
        if self.b.constants.len() == 1 << 16 {
            Err(ExceededMaxConstants)
        } else {
            self.b.constants.push(v);
            Ok((self.b.constants.len() - 1) as u16)
        }
    }

    pub fn get_jmp_index(&self) -> Option<usize> {
        self.op_positions.last().cloned().map(|lo| (lo + 1))
    }

    pub fn bytecode(mut self) -> Bytecode<'gc> {
        self.b.code.shrink_to_fit();
        self.b.constants.shrink_to_fit();
        self.b.lines.shrink_to_fit();
        self.b
    }
}

#[derive(Clone, Copy)]
pub struct BytecodeReader<'a> {
    ptr: *const u8,
    start: *const u8,
    end: *const u8,
    constants: &'a [Value<'a>],
    lines: &'a [LineStart],
    _marker: PhantomData<&'a [u8]>,
}

impl<'gc> fmt::Debug for Bytecode<'gc> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut reader = BytecodeReader::new(self);
        let mut lines_index = 0;
        unsafe {
            Ok(while !reader.is_at_end() {
                if reader.offset() == reader.lines[lines_index].offset as usize {
                    write!(f, "line {}: ", reader.lines[lines_index].line)?;
                    if lines_index != reader.lines.len() - 1 {
                        lines_index += 1
                    };
                }
                write!(f, "{}: ", reader.offset())?;
                match reader.read_op() {
                    Op::Wide => {
                        todo!()
                    }
                    Op::ExtraWide => {
                        todo!()
                    }
                    Op::LoadRegister => {
                        writeln!(f, "LoadRegister r{}", reader.read_u8())?;
                    }
                    Op::LoadInt => {
                        writeln!(f, "LoadInt {}", reader.read_i8())?;
                    }
                    Op::LoadNull => writeln!(f, "LoadNull")?,
                    Op::LoadTrue => writeln!(f, "LoadTrue")?,
                    Op::LoadFalse => writeln!(f, "LoadFalse")?,
                    Op::LoadConstant => {
                        writeln!(f, "LoadConstant {}", reader.read_u8())?;
                    }
                    Op::StoreRegister => {
                        todo!()
                    }
                    Op::Move => {
                        let dest = reader.read_u8();
                        let src = reader.read_u8();
                        writeln!(f, "Move r{},r{}", dest, src)?;
                    }
                    Op::LoadGlobal => {
                        writeln!(f, "LoadGlobal {}", reader.read_u8())?;
                    }
                    Op::StoreGlobal => {
                        writeln!(f, "StoreGlobal {}", reader.read_u8())?;
                    }
                    Op::AddRegister => {
                        writeln!(f, "AddRegister r{}", reader.read_u8())?;
                    }
                    Op::SubtractRegister => {
                        writeln!(f, "SubtractRegister r{}", reader.read_u8())?;
                    }
                    Op::MultiplyRegister => {
                        writeln!(f, "MultiplyRegister r{}", reader.read_u8())?;
                    }
                    Op::DivideRegister => {
                        writeln!(f, "DivideRegister r{}", reader.read_u8())?;
                    }
                    Op::AddInt => {
                        writeln!(f, "AddInt {}", reader.read_i8())?;
                    }
                    Op::SubtractInt => {
                        writeln!(f, "SubtractInt {}", reader.read_i8())?;
                    }
                    Op::MultiplyInt => {
                        writeln!(f, "MultiplyInt {}", reader.read_i8())?;
                    }
                    Op::DivideInt => {
                        writeln!(f, "Divide {}", reader.read_i8())?;
                    }
                    Op::Increment => {
                        writeln!(f, "Increment")?;
                    }
                    Op::Negate => writeln!(f, "Negate")?,
                    Op::Call => {
                        todo!()
                    }
                    Op::Call1Argument => {
                        let fun = reader.read_u8();
                        let arg0 = reader.read_u8();
                        writeln!(f, "Call1Argument r{} r{}", fun, arg0)?;
                    }
                    Op::Call0Argument => {
                        todo!()
                    }
                    Op::Call2Argument => {
                        todo!()
                    }
                    Op::Less => {
                        writeln!(f, "Less r{}", reader.read_u8())?;
                    }
                    Op::Jump => {
                        let offset = reader.read_u16();
                        writeln!(
                            f,
                            "Jump {} (to {})",
                            offset,
                            reader.offset() + (offset as usize)
                        )?;
                    }
                    Op::JumpBack => {
                        let offset = reader.read_u16();
                        writeln!(
                            f,
                            "JumpBack {} (to {})",
                            offset,
                            reader.offset() - (offset as usize)
                        )?;
                    }
                    Op::JumpIfFalse => {
                        let offset = reader.read_u16();
                        writeln!(
                            f,
                            "JumpIfFalse {} (to {})",
                            offset,
                            reader.offset() + (offset as usize)
                        )?;
                    }
                    Op::Return => {
                        writeln!(f, "Return")?;
                    }
                    Op::Exit => {
                        writeln!(f, "Exit")?;
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
                    Op::StoreR5 => {
                        writeln!(f, "StoreR5")?;
                    }
                    Op::StoreR6 => {
                        writeln!(f, "StoreR6")?;
                    }
                    Op::StoreR7 => {
                        writeln!(f, "StoreR7")?;
                    }
                    Op::StoreR8 => {
                        writeln!(f, "StoreR8")?;
                    }
                    Op::StoreR9 => {
                        writeln!(f, "StoreR9")?;
                    }
                    Op::StoreR10 => {
                        writeln!(f, "StoreR10")?;
                    }
                    Op::StoreR11 => {
                        writeln!(f, "StoreR11")?;
                    }
                    Op::StoreR12 => {
                        writeln!(f, "StoreR12")?;
                    }
                    Op::StoreR13 => {
                        writeln!(f, "StoreR13")?;
                    }
                    Op::StoreR14 => {
                        writeln!(f, "StoreR14")?;
                    }
                    Op::StoreR15 => {
                        writeln!(f, "StoreR15")?;
                    }
                    Op::ToString => {
                        writeln!(f, "ToString")?;
                    }
                }
            })
        }
    }
}

impl<'a> BytecodeReader<'a> {
    pub fn new(bytecode: &'a Bytecode) -> BytecodeReader<'a> {
        BytecodeReader {
            ptr: bytecode.code.as_ptr(),
            constants: bytecode.constants.as_ref(),
            _marker: PhantomData,
            start: bytecode.code.as_ptr(),
            lines: bytecode.lines.as_ref(),
            end: unsafe { bytecode.code.as_ptr().add(bytecode.code.len()) },
        }
    }

    fn is_at_end(&self) -> bool {
        self.ptr == self.end
    }

    pub unsafe fn read<T>(&mut self) -> T {
        debug_assert!(
            self.ptr >= self.start && self.ptr.wrapping_add(std::mem::size_of::<T>()) <= self.end
        );
        let ret = self.ptr.cast::<T>().read_unaligned();
        self.ptr = self.ptr.add(std::mem::size_of::<T>());
        ret
    }

    fn offset(&self) -> usize {
        unsafe { self.ptr.offset_from(self.start) as usize }
    }

    pub unsafe fn read_op(&mut self) -> Op {
        let op = self.read::<u8>();
        debug_assert!(Op::try_from(op).is_ok());
        Op::from_unchecked(op)
    }

    pub unsafe fn read_u8(&mut self) -> u8 {
        self.read::<u8>()
    }
    pub unsafe fn read_u16(&mut self) -> u16 {
        self.read::<u16>()
    }

    pub unsafe fn read_u32(&mut self) -> u32 {
        self.read::<u32>()
    }

    pub unsafe fn read_i8(&mut self) -> i8 {
        self.read::<i8>()
    }
    pub unsafe fn read_i16(&mut self) -> i16 {
        self.read::<i16>()
    }

    pub unsafe fn read_i32(&mut self) -> i32 {
        self.read::<i32>()
    }

    pub unsafe fn read_value<T: Into<usize>>(&mut self) -> Value<'a> {
        let u = self.read::<T>().into();
        debug_assert!(u < self.constants.len());
        *self.constants.get_unchecked(u)
    }

    pub unsafe fn jump(&mut self, offset: u16) {
        self.ptr = self.ptr.add(offset as usize);
        debug_assert!(self.ptr < self.end);
    }

    pub unsafe fn back_jump(&mut self, offset: u16) {
        self.ptr = self.ptr.sub(offset as usize);
        debug_assert!(self.ptr >= self.start);
    }
}

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
    code: Vec<u8>,
    constants: Vec<Value<'static>>,
    lines: Vec<LineStart>,
}

struct LineStart {
    offset: u32,
    line: u32,
}

pub struct ExceededMaxConstants;

pub struct BytecodeWriter {
    b: Bytecode,
    op_positions: Vec<usize>,
    regcount: u16,
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
            op_positions: vec![],
            regcount: 0,
        }
    }

    pub fn regcount(&self) -> u16 {
        self.regcount
    }

    pub fn push_register(&mut self) -> u16 {
        self.regcount += 1;
        self.regcount - 1
    }

    pub fn pop_register(&mut self) {
        self.regcount -= 1;
    }

    pub fn pop_last_op(&mut self) {
        let pos = self.op_positions.pop().unwrap();
        self.b.code.resize(*self.op_positions.last().unwrap(), 0);
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
            if line == *last_line {
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

    // The lifetime is static as it should be in the constant table
    pub fn write_value(
        &mut self,
        v: Value<'static>,
        line: u32,
    ) -> Result<(), ExceededMaxConstants> {
        self.write_op(Op::LoadConstant, line);
        for (i, constant) in self.b.constants.iter().enumerate() {
            if constant.strict_eq(v) {
                self.write_u16(i as u16);
                return Ok(());
            }
        }
        if self.b.constants.len() == 1 << 16 {
            Err(ExceededMaxConstants)
        } else {
            self.b.constants.push(v);
            self.write_u16((self.b.constants.len() - 1) as u16);
            Ok(())
        }
    }

    pub fn get_jmp_index(&self) -> Option<usize> {
        self.op_positions.last().cloned().map(|lo| (lo + 1))
    }

    pub fn bytecode(mut self) -> Bytecode {
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
    constants: &'a [Value<'static>],
    lines: &'a [LineStart],
    _marker: PhantomData<&'a [u8]>,
}

impl fmt::Debug for Bytecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut reader = BytecodeReader::new(self);
        let mut lines_index = 0;
        unsafe {
            Ok(while !reader.is_at_end() {
                if reader.offset() == reader.lines[lines_index].offset as usize {
                    write!(f, "line {}: ", reader.lines[lines_index].line);
                    if lines_index != reader.lines.len() - 1 {
                        lines_index += 1
                    };
                }
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
            ptr: bytecode.code.as_ptr(),
            constants: bytecode.constants.as_ref(),
            _marker: PhantomData,
            start: bytecode.code.as_ptr(),
            lines: bytecode.lines.as_ref(),
            end: unsafe { bytecode.code.as_ptr().add(bytecode.code.len()) },
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.ptr == self.end
    }

    pub unsafe fn read<T>(&mut self) -> T {
        debug_assert!(self.ptr >= self.start && self.ptr.add(std::mem::size_of::<T>()) <= self.end);
        let ret = self.ptr.cast::<T>().read_unaligned();
        self.ptr = self.ptr.add(std::mem::size_of::<T>());
        ret
    }

    pub fn offset(&self) -> usize {
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

    // The lifetime is static as it should be in the constant table
    pub unsafe fn read_value(&mut self) -> Value<'static> {
        let u = self.read_u16() as usize;
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

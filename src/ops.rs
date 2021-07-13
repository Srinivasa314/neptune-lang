use crate::{
    bytecode::{BytecodeMaxSizeExceeded, BytecodeWriter, Op},
    bytecode_writer::Int,
};
use std::convert::TryFrom;

macro_rules! binary_op_register {
    ($fn_name:ident,$inst_name:ident) => {
        pub fn $fn_name(&mut self, reg: u16) -> Result<(), BytecodeMaxSizeExceeded> {
            if let Ok(reg) = u8::try_from(reg) {
                self.write_op(Op::$inst_name)?;
                self.write_u8(reg)
            } else {
                self.write_op(Op::Wide)?;
                self.write_op(Op::$inst_name)?;
                self.write_u16(reg)
            }
        }
    };
}

macro_rules! binary_op_int {
    ($fn_name:ident,$inst_name:ident) => {
        pub fn $fn_name(&mut self, i: Int) -> Result<(), BytecodeMaxSizeExceeded> {
            match i {
                Int::I8(i) => {
                    self.write_op(Op::$inst_name)?;
                    self.write_i8(i)
                }
                Int::I16(i) => {
                    self.write_op(Op::Wide)?;
                    self.write_op(Op::$inst_name)?;
                    self.write_i16(i)
                }
                Int::I32(i) => {
                    self.write_op(Op::ExtraWide)?;
                    self.write_op(Op::$inst_name)?;
                    self.write_i32(i)
                }
            }
        }
    };
}
impl BytecodeWriter {
    pub fn write_op_load_register(&mut self, reg: u16) -> Result<(), BytecodeMaxSizeExceeded> {
        if let Ok(reg) = u8::try_from(reg) {
            self.write_op(Op::LoadRegister)?;
            self.write_u8(reg)
        } else {
            self.write_op(Op::Wide)?;
            self.write_op(Op::LoadRegister)?;
            self.write_u16(reg)
        }
    }

    pub fn write_op_store_register(&mut self, reg: u16) -> Result<(), BytecodeMaxSizeExceeded> {
        match reg {
            0 => self.write_op(Op::StoreR0),
            1 => self.write_op(Op::StoreR1),
            2 => self.write_op(Op::StoreR2),
            3 => self.write_op(Op::StoreR3),
            4 => self.write_op(Op::StoreR4),
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

    pub fn write_op_negate(&mut self) -> Result<(), BytecodeMaxSizeExceeded> {
        self.write_op(Op::Negate)
    }

    pub fn write_op_int(&mut self, i: i32) -> Int {
        if let Ok(i) = i8::try_from(i) {
            Int::I8(i)
        } else if let Ok(i) = i16::try_from(i) {
            Int::I16(i)
        } else {
            Int::I32(i)
        }
    }
}

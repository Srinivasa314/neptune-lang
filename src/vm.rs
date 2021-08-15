use cxx::{type_id, ExternType};
use std::marker::PhantomData;

struct StringSlice<'a> {
    data: *const u8,
    len: usize,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> From<&'a str> for StringSlice<'a> {
    fn from(s: &'a str) -> Self {
        Self {
            data: s.as_ptr(),
            len: s.len(),
            _marker: PhantomData,
        }
    }
}

unsafe impl<'a> ExternType for StringSlice<'a> {
    type Id = type_id!("neptune_vm::StringSlice");
    type Kind = cxx::kind::Trivial;
}

#[cxx::bridge(namespace = neptune_vm)]
mod ffi {
    enum Op {
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
        ConcatRegister,
        AddInt,
        SubtractInt,
        MultiplyInt,
        DivideInt,
        Negate,
        Call,
        Call0Argument,
        Call1Argument,
        Call2Argument,
        Less,
        ToString,
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
        LoadR0,
        LoadR1,
        LoadR2,
        LoadR3,
        LoadR4,
        LoadR5,
        LoadR6,
        LoadR7,
        LoadR8,
        LoadR9,
        LoadR10,
        LoadR11,
        LoadR12,
        LoadR13,
        LoadR14,
        LoadR15,
    }

    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.h");
        type StringSlice<'a> = super::StringSlice<'a>;
        type Op;
        type FunctionInfo;
        type VM;
        fn write_op(self: Pin<&mut FunctionInfo>, op: Op, line: u32) -> usize;
        fn write_u8(self: Pin<&mut FunctionInfo>, u: u8);
        fn write_u16(self: Pin<&mut FunctionInfo>, u: u16);
        fn write_u32(self: Pin<&mut FunctionInfo>, u: u32);
        fn write_i8(self: Pin<&mut FunctionInfo>, i: i8);
        fn write_i16(self: Pin<&mut FunctionInfo>, i: i16);
        fn write_i32(self: Pin<&mut FunctionInfo>, i: i32);
        fn float_constant(self: Pin<&mut FunctionInfo>, f: f64) -> Result<u16>;
        fn string_constant(self: Pin<&mut FunctionInfo>, s: StringSlice, vm: &VM) -> Result<u16>;
        fn symbol_constant(self: Pin<&mut FunctionInfo>, s: StringSlice, vm: &VM) -> Result<u16>;
        fn shrink(self: Pin<&mut FunctionInfo>);
        fn pop_last_op(self: Pin<&mut FunctionInfo>, last_op_pos: usize);
        fn new_function_info() -> UniquePtr<FunctionInfo>;
        fn add_global(self: &VM, name: StringSlice);
    }
}

pub use ffi::{new_function_info, FunctionInfo, Op, VM};

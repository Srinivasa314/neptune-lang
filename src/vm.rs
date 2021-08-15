use cxx::{type_id, ExternType};
use std::{ffi::c_void, marker::PhantomData};

pub struct StringSlice<'a> {
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
#[repr(C)]
pub struct FunctionInfoHandle {
    inner: *const c_void,
}

unsafe impl ExternType for FunctionInfoHandle {
    type Id = type_id!("neptune_vm::FunctionInfoHandle");
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
        type FunctionInfoHandle = super::FunctionInfoHandle;
        type VM;
        fn write_op(self: &FunctionInfoHandle, op: Op, line: u32) -> usize;
        fn write_u8(self: &FunctionInfoHandle, u: u8);
        fn write_u16(self: &FunctionInfoHandle, u: u16);
        fn write_u32(self: &FunctionInfoHandle, u: u32);
        fn write_i8(self: &FunctionInfoHandle, i: i8);
        fn write_i16(self: &FunctionInfoHandle, i: i16);
        fn write_i32(self: &FunctionInfoHandle, i: i32);
        fn float_constant(self: &FunctionInfoHandle, f: f64) -> Result<u16>;
        fn string_constant(self: &FunctionInfoHandle, s: StringSlice, vm: &VM) -> Result<u16>;
        fn symbol_constant(self: &FunctionInfoHandle, s: StringSlice, vm: &VM) -> Result<u16>;
        fn shrink(self: &FunctionInfoHandle);
        fn pop_last_op(self: &FunctionInfoHandle, last_op_pos: usize);
        fn add_global(self: &VM, name: StringSlice);
        fn new_function_info(self: &VM) -> FunctionInfoHandle;
    }
}

pub use ffi::{Op, VM};

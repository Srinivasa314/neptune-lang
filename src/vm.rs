use cxx::{type_id, ExternType};
use std::{ffi::c_void, marker::PhantomData};

#[repr(C)]
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
pub struct FunctionInfoWriter<'vm> {
    handle: *const c_void,
    vm: *const c_void,
    _marker: PhantomData<&'vm ()>,
}

unsafe impl<'vm> ExternType for FunctionInfoWriter<'vm> {
    type Id = type_id!("neptune_vm::FunctionInfoWriter");
    type Kind = cxx::kind::Trivial;
}

impl<'vm> Drop for FunctionInfoWriter<'vm> {
    fn drop(&mut self) {
        unsafe { self.release() }
    }
}

#[cxx::bridge(namespace = neptune_vm)]
mod ffi {
    enum Op {
        Wide,
        ExtraWide,
        LoadRegister,
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
        LoadInt,
        LoadNull,
        LoadTrue,
        LoadFalse,
        LoadConstant,
        StoreRegister,
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
    }

    enum VMResult {
        Success,
        Error,
    }

    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.h");
        type StringSlice<'a> = super::StringSlice<'a>;
        type Op;
        type VMResult;
        type VM;
        type FunctionInfoWriter<'a> = super::FunctionInfoWriter<'a>;
        fn write_op(self: &mut FunctionInfoWriter, op: Op, line: u32) -> usize;
        fn run(self: &mut FunctionInfoWriter) -> VMResult;
        fn write_u8(self: &mut FunctionInfoWriter, u: u8);
        fn write_u16(self: &mut FunctionInfoWriter, u: u16);
        fn write_u32(self: &mut FunctionInfoWriter, u: u32);
        fn float_constant(self: &mut FunctionInfoWriter, f: f64) -> Result<u16>;
        fn string_constant<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            s: StringSlice<'s>,
        ) -> Result<u16>;
        fn symbol_constant<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            s: StringSlice<'s>,
        ) -> Result<u16>;
        fn shrink(self: &mut FunctionInfoWriter);
        fn pop_last_op(self: &mut FunctionInfoWriter, last_op_pos: usize);
        fn add_global<'vm, 's>(self: &'vm VM, name: StringSlice<'s>);
        fn new_function_info<'vm>(self: &'vm VM) -> FunctionInfoWriter<'vm>;
        fn new_vm() -> UniquePtr<VM>;
        unsafe fn release(self: &mut FunctionInfoWriter);
    }
}

pub use ffi::{new_vm, Op, VMResult, VM};

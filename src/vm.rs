use cxx::{type_id, ExternType};
use std::{
    ffi::c_void,
    fmt::{Debug, Display},
    marker::PhantomData,
};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct StringSlice<'a> {
    data: *const u8,
    len: usize,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> StringSlice<'a> {
    fn as_str(self) -> &'a str {
        unsafe {
            let s = std::slice::from_raw_parts(self.data, self.len);
            std::str::from_utf8_unchecked(s)
        }
    }
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

impl<'a> Display for StringSlice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for FunctionInfoWriter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_cxx_string())
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
    #[repr(u8)]
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
        LoadSubscript,
        StoreArrayUnchecked,
        StoreSubscript,
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
        Equal,
        NotEqual,
        StrictEqual,
        StrictNotEqual,
        GreaterThan,
        LesserThan,
        GreaterThanOrEqual,
        LesserThanOrEqual,
        Call,
        Call0Argument,
        Call1Argument,
        Call2Argument,
        ToString,
        NewArray,
        NewMap,
        Jump,
        JumpBack,
        JumpIfFalse,
        Return,
        Exit,
    }

    #[repr(u8)]
    enum VMStatus {
        Success,
        Error,
    }

    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.h");
        type StringSlice<'a> = super::StringSlice<'a>;
        type Op;
        type VMResult;
        type VMStatus;
        type VM;
        type FunctionInfoWriter<'a> = super::FunctionInfoWriter<'a>;
        fn write_op(self: &mut FunctionInfoWriter, op: Op, line: u32) -> usize;
        // The bytecode should be valid
        unsafe fn run(self: &mut FunctionInfoWriter) -> UniquePtr<VMResult>;
        fn to_cxx_string(self: &FunctionInfoWriter) -> UniquePtr<CxxString>;
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
        fn set_max_registers(self: &mut FunctionInfoWriter, max_registers: u16);
        fn add_global<'vm, 's>(self: &'vm VM, name: StringSlice<'s>);
        fn new_function_info<'vm>(self: &'vm VM) -> FunctionInfoWriter<'vm>;
        fn new_vm() -> UniquePtr<VM>;
        // This must only be called by drop
        unsafe fn release(self: &mut FunctionInfoWriter);
        fn get_result<'a>(self: &'a VMResult) -> StringSlice<'a>;
        fn get_status(self: &VMResult) -> VMStatus;
    }
}

pub use ffi::{new_vm, Op, VMStatus, VM};

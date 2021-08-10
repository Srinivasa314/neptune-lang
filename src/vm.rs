use std::os::raw::c_char;
use std::pin::Pin;

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
    }

    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.h");
        type Op;
        type Value;
        type FunctionInfo;
        fn write_op(self: Pin<&mut FunctionInfo>, op: Op, line: u32);
        fn write_u8(self: Pin<&mut FunctionInfo>, u: u8);
        fn write_u16(self: Pin<&mut FunctionInfo>, u: u16);
        fn write_u32(self: Pin<&mut FunctionInfo>, u: u32);
        fn write_i8(self: Pin<&mut FunctionInfo>, i: i8);
        fn write_i16(self: Pin<&mut FunctionInfo>, i: i16);
        fn write_i32(self: Pin<&mut FunctionInfo>, i: i32);
        fn float_constant(self: Pin<&mut FunctionInfo>) -> Result<u16>;
        unsafe fn string_constant(
            self: Pin<&mut FunctionInfo>,
            s: *const c_char,
            len: usize,
        ) -> Result<u16>;
        unsafe fn symbol_constant(
            self: Pin<&mut FunctionInfo>,
            s: *const c_char,
            len: usize,
        ) -> Result<u16>;
        fn shrink(self: Pin<&mut FunctionInfo>);
        fn shrink_to(self: Pin<&mut FunctionInfo>, size: usize);
    }
}
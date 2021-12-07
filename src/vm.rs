use cxx::{type_id, ExternType};
use std::{ffi::c_void, fmt::Display, marker::PhantomData};

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

unsafe impl<'a> ExternType for StringSlice<'a> {
    type Id = type_id!("neptune_vm::StringSlice");
    type Kind = cxx::kind::Trivial;
}

#[repr(C)]
pub struct FunctionInfoWriter<'vm> {
    handle: *mut c_void,
    vm: *mut c_void,
    constants: *mut c_void,
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

#[repr(C)]
pub struct ModuleVariable {
    pub position: u32,
    pub mutable: bool,
    pub exported: bool,
}

unsafe impl ExternType for ModuleVariable {
    type Id = type_id!("neptune_vm::ModuleVariable");
    type Kind = cxx::kind::Trivial;
}
#[repr(C)]
pub struct FunctionContext<'a> {
    vm: *const c_void,
    task: *const c_void,
    value: *const c_void,
    _marker: PhantomData<&'a ()>,
}

unsafe impl<'a> ExternType for FunctionContext<'a> {
    type Id = type_id!("neptune_vm::FunctionContext");
    type Kind = cxx::kind::Trivial;
}

#[allow(dead_code)]
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
        LoadSmallInt,
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
        LoadModuleVariable,
        StoreModuleVariable,
        LoadProperty,
        StoreProperty,
        LoadUpvalue,
        StoreUpvalue,
        LoadSubscript,
        StoreArrayUnchecked,
        StoreSubscript,
        AddRegister,
        SubtractRegister,
        MultiplyRegister,
        DivideRegister,
        ModRegister,
        ConcatRegister,
        AddInt,
        SubtractInt,
        MultiplyInt,
        DivideInt,
        ModInt,
        Negate,
        Not,
        Equal,
        NotEqual,
        StrictEqual,
        StrictNotEqual,
        GreaterThan,
        LesserThan,
        GreaterThanOrEqual,
        LesserThanOrEqual,
        Call,
        CallMethod,
        SuperCall,
        Construct,
        ToString,
        NewArray,
        NewMap,
        NewObject,
        EmptyArray,
        EmptyMap,
        EmptyObject,
        MakeFunction,
        MakeClass,
        ForLoop,
        Jump,
        JumpIfFalseOrNull,
        JumpIfNotFalseOrNull,
        BeginForLoop,
        JumpBack,
        JumpConstant,
        JumpIfFalseOrNullConstant,
        JumpIfNotFalseOrNullConstant,
        BeginForLoopConstant,
        Close,
        Return,
        Panic,
    }

    #[repr(u8)]
    enum VMStatus {
        Success,
        Error,
    }

    #[repr(u8)]
    enum EFuncStatus {
        Ok,
        TypeError,
        Underflow,
        OutOfBounds,
        PropertyError,
    }

    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.h");
        type StringSlice<'a> = super::StringSlice<'a>;
        type ModuleVariable = super::ModuleVariable;
        type Op;
        type VMStatus;
        type EFuncStatus;
        type VM;
        type FunctionInfoWriter<'a> = super::FunctionInfoWriter<'a>;
        type FunctionContext<'a> = super::FunctionContext<'a>;
        fn write_op(self: &mut FunctionInfoWriter, op: Op, line: u32) -> usize;
        // The bytecode should be valid
        unsafe fn run(self: &mut FunctionInfoWriter) -> VMStatus;
        fn write_u8(self: &mut FunctionInfoWriter, u: u8);
        fn write_u16(self: &mut FunctionInfoWriter, u: u16);
        fn write_u32(self: &mut FunctionInfoWriter, u: u32);
        fn reserve_constant(self: &mut FunctionInfoWriter) -> Result<u16>;
        fn float_constant(self: &mut FunctionInfoWriter, f: f64) -> Result<u16>;
        fn string_constant<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            s: StringSlice<'s>,
        ) -> Result<u16>;
        fn symbol_constant<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            s: StringSlice<'s>,
        ) -> Result<u16>;
        fn int_constant(self: &mut FunctionInfoWriter, i: i32) -> Result<u16>;
        fn fun_constant(self: &mut FunctionInfoWriter, f: FunctionInfoWriter) -> Result<u16>;
        fn shrink(self: &mut FunctionInfoWriter);
        fn pop_last_op(self: &mut FunctionInfoWriter, last_op_pos: usize);
        fn set_max_registers(self: &mut FunctionInfoWriter, max_registers: u16);
        fn class_constant<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            s: StringSlice<'s>,
        ) -> Result<u16>;
        fn add_method<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            class_: u16,
            name: StringSlice<'s>,
            f: FunctionInfoWriter,
        );
        fn add_module_variable<'vm, 's>(
            self: &'vm VM,
            module: StringSlice<'s>,
            name: StringSlice<'s>,
            mutable_: bool,
            exported: bool,
        ) -> bool;
        fn get_module_variable(
            self: &VM,
            module_name: StringSlice,
            name: StringSlice,
        ) -> Result<ModuleVariable>;
        fn new_function_info<'vm>(
            self: &'vm VM,
            module: StringSlice,
            name: StringSlice,
            arity: u8,
        ) -> FunctionInfoWriter<'vm>;
        fn new_vm() -> UniquePtr<VM>;
        // This must only be called by drop
        unsafe fn release(self: &mut FunctionInfoWriter);
        fn patch_jump(self: &mut FunctionInfoWriter, op_position: usize, jump_offset: u32);
        fn add_upvalue(self: &mut FunctionInfoWriter, index: u16, is_local: bool);
        fn add_exception_handler(
            self: &mut FunctionInfoWriter,
            try_begin: u32,
            try_end: u32,
            error_reg: u16,
            catch_begin: u32,
        );
        fn size(self: &FunctionInfoWriter) -> usize;
        fn get_stack_trace(self: &VM) -> String;
        fn get_result(self: &VM) -> String;
        fn create_module(self: &VM, module_name: StringSlice);
        fn create_module_with_prelude(self: &VM, module_name: StringSlice);
        fn module_exists(self: &VM, module_name: StringSlice) -> bool;

        fn push_int(self: &mut FunctionContext, i: i32);
        fn push_float(self: &mut FunctionContext, f: f64);
        fn push_bool(self: &mut FunctionContext, b: bool);
        fn push_null(self: &mut FunctionContext);
        fn push_string(self: &mut FunctionContext, s: StringSlice);
        fn push_symbol(self: &mut FunctionContext, s: StringSlice);
        fn push_empty_array(self: &mut FunctionContext);
        fn push_to_array(self: &mut FunctionContext) -> EFuncStatus;
        fn push_empty_object(self: &mut FunctionContext);
        fn set_object_property(self: &mut FunctionContext, s: StringSlice) -> EFuncStatus;
        fn as_int(self: &mut FunctionContext, i: &mut i32) -> EFuncStatus;
        fn as_float(self: &mut FunctionContext, d: &mut f64) -> EFuncStatus;
        fn as_bool(self: &mut FunctionContext, b: &mut bool) -> EFuncStatus;
        fn is_null(self: &mut FunctionContext) -> EFuncStatus;
        fn as_string(self: &mut FunctionContext, s: &mut String) -> EFuncStatus;
        fn as_symbol(self: &mut FunctionContext, s: &mut String) -> EFuncStatus;
        fn get_array_length(self: &FunctionContext, len: &mut usize) -> EFuncStatus;
        fn get_array_element(self: &mut FunctionContext, pos: usize) -> EFuncStatus;
        fn is_object(self: &FunctionContext) -> EFuncStatus;
        fn get_object_property(self: &mut FunctionContext, prop: StringSlice) -> EFuncStatus;
        fn pop(self: &mut FunctionContext) -> bool;
    }
}

pub use ffi::{new_vm, Op, VMStatus, VM};

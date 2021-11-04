use cxx::{type_id, ExternType};
use std::{ffi::c_void, fmt::Display, marker::PhantomData, ops::Index};

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
}

unsafe impl ExternType for ModuleVariable {
    type Id = type_id!("neptune_vm::ModuleVariable");
    type Kind = cxx::kind::Trivial;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FunctionContextInner {
    vm: *mut ffi::VM,
    slots: *mut c_void,
    max_slots: u16,
}

unsafe impl ExternType for FunctionContextInner {
    type Id = type_id!("neptune_vm::FunctionContext");
    type Kind = cxx::kind::Trivial;
}

pub struct FunctionContext(FunctionContextInner);

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
        Call0Argument,
        Call1Argument,
        Call2Argument,
        Call3Argument,
        ToString,
        NewArray,
        NewMap,
        EmptyArray,
        EmptyMap,
        MakeFunction,
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
        Exit,
    }

    #[repr(u8)]
    enum VMStatus {
        Success,
        Error,
    }

    #[repr(u8)]
    enum NativeFunctionStatus {
        Ok,
        InvalidSlotError,
        TypeError,
    }

    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.h");
        type StringSlice<'a> = super::StringSlice<'a>;
        type ModuleVariable = super::ModuleVariable;
        type Op;
        type VMStatus;
        type VM;
        type Data;
        type NativeFunctionCallback;
        type FreeDataCallback;
        type NativeFunctionStatus;
        type FunctionInfoWriter<'a> = super::FunctionInfoWriter<'a>;
        type FunctionContext = super::FunctionContextInner;
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
        fn add_module_variable<'vm, 's>(
            self: &'vm VM,
            module: StringSlice<'s>,
            name: StringSlice<'s>,
            mutable_: bool,
        ) -> bool;
        fn get_module_variable<'vm, 's>(
            self: &'vm VM,
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
        unsafe fn declare_native_function(
            self: &VM,
            module: StringSlice,
            name: StringSlice,
            arity: u8,
            extra_slots: u16,
            callback: *mut NativeFunctionCallback,
            data: *mut Data,
            free_data: *mut FreeDataCallback,
        ) -> bool;
        fn get_stack_trace(self: &VM) -> String;
        fn get_result(self: &VM) -> String;
        fn return_value(self: &FunctionContext, slot: u16) -> NativeFunctionStatus;
        fn as_string(self: &FunctionContext, slot: u16, s: &mut String) -> NativeFunctionStatus;
        fn to_string(self: &FunctionContext, dest: u16, source: u16) -> NativeFunctionStatus;
        fn null(self: &FunctionContext, slot: u16) -> NativeFunctionStatus;
        // For safety whenever control is returned to the VM, the bytecode must be valid
        unsafe fn function(
            self: &FunctionContext,
            slot: u16,
            fw: FunctionInfoWriter,
        ) -> NativeFunctionStatus;
        fn error(self: &FunctionContext, slot: u16, error: StringSlice) -> NativeFunctionStatus;
        fn create_module(self: &VM, module_name: StringSlice);
        fn module_exists(self: &VM, module_name: StringSlice) -> bool;
    }
}

use ffi::{Data, FreeDataCallback, NativeFunctionCallback, NativeFunctionStatus};

impl VM {
    pub fn declare_native_rust_function<F>(
        &self,
        module: &str,
        name: &str,
        arity: u8,
        extra_slots: u16,
        callback: F,
    ) -> bool
    where
        F: FnMut(FunctionContext) -> Result<u16, u16> + 'static,
    {
        let data = Box::into_raw(Box::new(callback));
        unsafe {
            self.declare_native_function(
                module.into(),
                name.into(),
                arity,
                extra_slots,
                trampoline::<F> as *mut NativeFunctionCallback,
                data as *mut Data,
                free_data::<F> as *mut FreeDataCallback,
            )
        }
    }
}

unsafe extern "C" fn trampoline<F>(ctx: FunctionContextInner, data: *mut Data) -> bool
where
    F: FnMut(FunctionContext) -> Result<u16, u16> + 'static,
{
    let callback = data as *mut F;
    let callback = &mut *callback;
    //https://github.com/rust-lang/rust/issues/52652#issuecomment-695034481
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        match callback(FunctionContext(ctx)) {
            Ok(slot) => {
                if ctx.return_value(slot) == NativeFunctionStatus::InvalidSlotError {
                    panic!("Attempt to return invalid slot");
                }
                true
            }
            Err(slot) => {
                if ctx.return_value(slot) == NativeFunctionStatus::InvalidSlotError {
                    panic!("Attempt to return invalid slot");
                }
                false
            }
        }
    }))
    .unwrap_or_else(|_| std::process::abort())
}

unsafe extern "C" fn free_data<F>(data: *mut Data) {
    Box::from_raw(data as *mut F);
}

impl FunctionContext {
    pub fn as_string(&self, slot: u16) -> Option<String> {
        let mut s = String::new();
        match self.0.as_string(slot, &mut s) {
            NativeFunctionStatus::InvalidSlotError => panic!("Attempt to access invalid slot"),
            NativeFunctionStatus::TypeError => None,
            _ => Some(s),
        }
    }

    pub fn to_string(&self, dest: u16, source: u16) {
        if self.0.to_string(dest, source) == NativeFunctionStatus::InvalidSlotError {
            panic!("Attempt to access invalid slot")
        }
    }

    pub fn null(&self, slot: u16) {
        if self.0.null(slot) == NativeFunctionStatus::InvalidSlotError {
            panic!("Attempt to access invalid slot")
        }
    }

    pub fn error(&self, slot: u16, error: &str) {
        if self.0.error(slot, error.into()) == NativeFunctionStatus::InvalidSlotError {
            panic!("Attempt to access invalid slot")
        }
    }

    pub(crate) fn vm(&self) -> &VM {
        unsafe { &*self.0.vm }
    }

    // For safety whenever control is returned to the VM, the bytecode must be valid
    pub(crate) unsafe fn function(self: &FunctionContext, slot: u16, fw: FunctionInfoWriter) {
        if self.0.function(slot, fw) == NativeFunctionStatus::InvalidSlotError {
            panic!("Attempt to access invalid slot")
        }
    }
}

pub use ffi::{new_vm, Op, VMStatus, VM};

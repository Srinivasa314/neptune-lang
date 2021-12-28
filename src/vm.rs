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
pub struct EFuncContextInner<'a> {
    vm: *const VM,
    task: *mut c_void,
    value: *mut c_void,
    _marker: PhantomData<&'a ()>,
}

unsafe impl<'a> ExternType for EFuncContextInner<'a> {
    type Id = type_id!("neptune_vm::EFuncContext");
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
        NewArray,
        NewMap,
        NewObject,
        MakeFunction,
        MakeClass,
        Range,
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
        Throw,
    }

    #[repr(u8)]
    enum VMStatus {
        Success,
        Error,
    }

    #[derive(Debug)]
    #[repr(u8)]
    enum EFuncStatus {
        Ok,
        TypeError,
        Underflow,
        OutOfBoundsError,
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
        type EFuncContext<'a> = super::EFuncContextInner<'a>;
        type EFuncCallback;
        type FreeDataCallback;
        type Data;

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
        fn get_result(self: &VM) -> String;
        fn create_module(self: &VM, module_name: StringSlice);
        fn create_module_with_prelude(self: &VM, module_name: StringSlice);
        fn module_exists(self: &VM, module_name: StringSlice) -> bool;
        //Safe as long as functions of the correct type are passed
        unsafe fn create_efunc(
            self: &VM,
            name: StringSlice,
            callback: *mut EFuncCallback,
            data: *mut Data,
            free_data: *mut FreeDataCallback,
        ) -> bool;

        fn push_int(self: &mut EFuncContext, i: i32);
        fn push_float(self: &mut EFuncContext, f: f64);
        fn push_bool(self: &mut EFuncContext, b: bool);
        fn push_null(self: &mut EFuncContext);
        fn push_string(self: &mut EFuncContext, s: StringSlice);
        fn push_symbol(self: &mut EFuncContext, s: StringSlice);
        fn push_empty_array(self: &mut EFuncContext);
        fn push_to_array(self: &mut EFuncContext) -> EFuncStatus;
        fn push_empty_object(self: &mut EFuncContext);
        fn set_object_property(self: &mut EFuncContext, s: StringSlice) -> EFuncStatus;
        fn as_int(self: &mut EFuncContext, i: &mut i32) -> EFuncStatus;
        fn as_float(self: &mut EFuncContext, d: &mut f64) -> EFuncStatus;
        fn as_bool(self: &mut EFuncContext, b: &mut bool) -> EFuncStatus;
        fn is_null(self: &mut EFuncContext) -> EFuncStatus;
        fn as_string<'a>(self: &'a mut EFuncContext, s: &mut StringSlice<'a>) -> EFuncStatus;
        fn as_symbol<'a>(self: &'a mut EFuncContext, s: &mut StringSlice<'a>) -> EFuncStatus;
        fn get_array_length(self: &EFuncContext, len: &mut usize) -> EFuncStatus;
        fn get_array_element(self: &mut EFuncContext, pos: usize) -> EFuncStatus;
        fn get_object_property(self: &mut EFuncContext, prop: StringSlice) -> EFuncStatus;
        fn pop(self: &mut EFuncContext) -> bool;
        fn push_empty_map(self: &mut EFuncContext);
        fn insert_in_map(self: &mut EFuncContext) -> EFuncStatus;
        //Function must contain valid bytecode
        unsafe fn push_function(self: &mut EFuncContext, fw: FunctionInfoWriter);
    }
}

use ffi::EFuncStatus;
pub use ffi::{new_vm, Op, VMStatus, VM};

impl VM {
    pub fn create_efunc_safe<'vm, F>(&'vm self, name: &str, callback: F) -> bool
    where
        F: FnMut(&'vm VM, EFuncContext) -> bool + 'static,
    {
        unsafe {
            self.create_efunc(
                name.into(),
                trampoline::<F> as *mut ffi::EFuncCallback,
                Box::into_raw(Box::new(callback)) as *mut ffi::Data,
                free_data::<F> as *mut ffi::FreeDataCallback,
            )
        }
    }
}

unsafe extern "C" fn trampoline<'vm, F>(cx: EFuncContext, data: *mut c_void) -> bool
where
    F: FnMut(&'vm VM, EFuncContext) -> bool + 'static,
{
    let callback = &mut *(data as *mut F);
    // https://github.com/rust-lang/rust/issues/52652#issuecomment-695034481
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(&*cx.0.vm, cx)))
        .unwrap_or_else(|_| std::process::abort())
}

unsafe extern "C" fn free_data<F>(data: *mut c_void) {
    Box::from_raw(data as *mut F);
}

#[derive(Debug)]
pub enum EFuncError {
    TypeError,
    PropertyError,
    OutOfBoundsError,
    Underflow,
}

#[repr(transparent)]
pub struct EFuncContext<'a>(EFuncContextInner<'a>);

impl<'a> EFuncContext<'a> {
    pub fn int(&mut self, i: i32) {
        self.0.push_int(i)
    }

    pub fn float(&mut self, f: f64) {
        self.0.push_float(f)
    }

    pub fn bool(&mut self, b: bool) {
        self.0.push_bool(b)
    }

    pub fn null(&mut self) {
        self.0.push_null()
    }

    pub fn string(&mut self, s: &str) {
        self.0.push_string(s.into())
    }

    pub fn symbol(&mut self, s: &str) {
        self.0.push_symbol(s.into())
    }

    pub fn array(&mut self) {
        self.0.push_empty_array()
    }

    pub fn push_to_array(&mut self) -> Result<(), EFuncError> {
        match self.0.push_to_array() {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn object(&mut self) {
        self.0.push_empty_object()
    }

    pub fn set_object_property(&mut self, prop: &str) -> Result<(), EFuncError> {
        match self.0.set_object_property(prop.into()) {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn as_int(&mut self) -> Result<i32, EFuncError> {
        let mut i = 0;
        match self.0.as_int(&mut i) {
            EFuncStatus::Ok => Ok(i),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn as_float(&mut self) -> Result<f64, EFuncError> {
        let mut f = 0.0;
        match self.0.as_float(&mut f) {
            EFuncStatus::Ok => Ok(f),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn as_bool(&mut self) -> Result<bool, EFuncError> {
        let mut b = false;
        match self.0.as_bool(&mut b) {
            EFuncStatus::Ok => Ok(b),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn is_null(&mut self) -> Result<bool, EFuncError> {
        match self.0.is_null() {
            EFuncStatus::Ok => Ok(true),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Ok(false),
            _ => unreachable!(),
        }
    }

    pub fn as_string<'b>(&'b mut self) -> Result<&'b str, EFuncError> {
        let mut s = StringSlice::from("");
        match self.0.as_string(&mut s) {
            EFuncStatus::Ok => Ok(s.as_str()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn as_symbol<'b>(&'b mut self) -> Result<&'b str, EFuncError> {
        let mut s = StringSlice::from("");
        match self.0.as_symbol(&mut s) {
            EFuncStatus::Ok => Ok(s.as_str()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn array_length(&mut self) -> Result<usize, EFuncError> {
        let mut size = 0;
        match self.0.get_array_length(&mut size) {
            EFuncStatus::Ok => Ok(size),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    pub fn get_element(&mut self, index: usize) -> Result<(), EFuncError> {
        match self.0.get_array_element(index) {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            EFuncStatus::OutOfBoundsError => Err(EFuncError::OutOfBoundsError),
            _ => unreachable!(),
        }
    }

    pub fn get_property(&mut self, prop: &str) -> Result<(), EFuncError> {
        match self.0.get_object_property(prop.into()) {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            EFuncStatus::PropertyError => Err(EFuncError::PropertyError),
            _ => unreachable!(),
        }
    }

    pub fn pop(&mut self) -> Result<(), EFuncError> {
        if self.0.pop() {
            Ok(())
        } else {
            Err(EFuncError::Underflow)
        }
    }

    pub fn map(&mut self) {
        self.0.push_empty_map()
    }

    pub fn insert_in_map(&mut self) -> Result<(), EFuncError> {
        match self.0.insert_in_map() {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    //Function must contain valid bytecode
    pub(crate) unsafe fn function(&mut self, fw: FunctionInfoWriter) {
        self.0.push_function(fw)
    }
}

pub trait ToNeptuneValue {
    fn to_neptune_value(self, cx: &mut EFuncContext);
}

impl ToNeptuneValue for i32 {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.int(self)
    }
}

impl ToNeptuneValue for f64 {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.float(self)
    }
}

impl ToNeptuneValue for bool {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.bool(self)
    }
}

impl ToNeptuneValue for () {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.null()
    }
}

impl<T: ToNeptuneValue> ToNeptuneValue for Option<T> {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        match self {
            Some(t) => t.to_neptune_value(cx),
            None => cx.null(),
        }
    }
}

impl ToNeptuneValue for &str {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.string(self)
    }
}

impl ToNeptuneValue for String {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.string(&self)
    }
}

impl<T: ToNeptuneValue> ToNeptuneValue for Vec<T> {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.array();
        for elem in self {
            elem.to_neptune_value(cx);
            cx.push_to_array().unwrap();
        }
    }
}

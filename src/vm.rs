use cxx::{type_id, ExternType};
use futures::{stream::FuturesUnordered, Future};
use std::any::TypeId;
use std::cell::RefCell;
use std::{ffi::c_void, fmt::Display, marker::PhantomData, pin::Pin};
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
    pub reuse_constants: bool,
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
pub struct TaskHandle<'vm> {
    handle: *mut c_void,
    vm: *mut c_void,
    _marker: PhantomData<&'vm ()>,
}

unsafe impl<'vm> ExternType for TaskHandle<'vm> {
    type Id = type_id!("neptune_vm::TaskHandle");
    type Kind = cxx::kind::Trivial;
}

impl<'vm> Drop for TaskHandle<'vm> {
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
        Switch,
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
        Suspend,
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

    extern "Rust" {
        type UserData<'a>;
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
        type TaskHandle<'a> = super::TaskHandle<'a>;
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
        fn reserve_constant(self: &mut FunctionInfoWriter) -> u32;
        fn float_constant(self: &mut FunctionInfoWriter, f: f64) -> u32;
        fn string_constant<'vm, 's>(self: &mut FunctionInfoWriter<'vm>, s: StringSlice<'s>) -> u32;
        fn symbol_constant<'vm, 's>(self: &mut FunctionInfoWriter<'vm>, s: StringSlice<'s>) -> u32;
        fn int_constant(self: &mut FunctionInfoWriter, i: i32) -> u32;
        fn fun_constant(self: &mut FunctionInfoWriter, f: FunctionInfoWriter) -> u32;
        fn shrink(self: &mut FunctionInfoWriter);
        fn pop_last_op(self: &mut FunctionInfoWriter, last_op_pos: usize);
        fn set_max_registers(self: &mut FunctionInfoWriter, max_registers: u32);
        fn class_constant<'vm, 's>(self: &mut FunctionInfoWriter<'vm>, s: StringSlice<'s>) -> u32;
        fn bool_constant(self: &mut FunctionInfoWriter, b: bool) -> u32;
        fn null_constant(self: &mut FunctionInfoWriter) -> u32;
        fn add_method<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            class_: u32,
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
        fn new_vm(user_data: Box<UserData<'static>>) -> UniquePtr<VM>;
        fn get_user_data<'vm>(self: &'vm VM) -> &'vm UserData;
        // This must only be called by drop
        unsafe fn release(self: &mut FunctionInfoWriter);
        fn patch_jump(self: &mut FunctionInfoWriter, op_position: usize, jump_offset: u32);
        fn add_upvalue(self: &mut FunctionInfoWriter, index: u32, is_local: bool);
        fn add_exception_handler(
            self: &mut FunctionInfoWriter,
            try_begin: u32,
            try_end: u32,
            error_reg: u32,
            catch_begin: u32,
        );
        fn jump_table(self: &mut FunctionInfoWriter) -> u32;
        fn insert_in_jump_table(
            self: &mut FunctionInfoWriter,
            jump_table: u32,
            offset: u32,
        ) -> bool;
        fn size(self: &FunctionInfoWriter) -> usize;
        fn get_result(self: &VM) -> String;
        fn create_module(self: &VM, module_name: StringSlice);
        fn create_module_with_prelude(self: &VM, module_name: StringSlice);
        fn module_exists(self: &VM, module_name: StringSlice) -> bool;
        /*functions of the correct type should be passed and the functions must
        not exhibit undefined behaviour if data is passed to them*/
        unsafe fn create_efunc(
            self: &VM,
            name: StringSlice,
            callback: *mut EFuncCallback,
            data: *mut Data,
            free_data: *mut FreeDataCallback,
        ) -> bool;
        fn kill_main_task(self: &VM, error: StringSlice, message: StringSlice) -> String;

        fn push_int(self: &mut EFuncContext, i: i32);
        fn push_float(self: &mut EFuncContext, f: f64);
        fn push_bool(self: &mut EFuncContext, b: bool);
        fn push_null(self: &mut EFuncContext);
        fn push_string(self: &mut EFuncContext, s: StringSlice);
        fn push_symbol(self: &mut EFuncContext, s: StringSlice);
        fn push_empty_array(self: &mut EFuncContext);
        fn push_to_array(self: &mut EFuncContext) -> EFuncStatus;
        fn push_empty_object(self: &mut EFuncContext);
        fn push_error(
            self: &mut EFuncContext,
            module: StringSlice,
            error_class: StringSlice,
            message: StringSlice,
        ) -> EFuncStatus;
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
        fn get_vm<'a>(self: &EFuncContext<'a>) -> &'a VM;
        //Function must contain valid bytecode
        unsafe fn push_function(self: &mut EFuncContext, fw: FunctionInfoWriter);
        // This must only be called by drop
        unsafe fn release(self: &mut TaskHandle);
        fn get_current_task<'vm>(self: &'vm VM) -> TaskHandle<'vm>;
        /*callbacks should have correct type and must not exhibit undefined behaviour
        if data is passed to it*/
        unsafe fn resume(
            self: &mut TaskHandle,
            callback: *mut EFuncCallback,
            data: *mut Data,
        ) -> VMStatus;
        /*callback should have correct type and must not exhibit undefined behaviour
        if data is passed to it*/
        unsafe fn push_resource(
            self: &mut EFuncContext,
            data: *mut Data,
            free_data: *mut FreeDataCallback,
        );
        fn as_resource(self: &mut EFuncContext, status: &mut EFuncStatus) -> *mut Data;
    }
}

use ffi::EFuncStatus;
pub use ffi::{new_vm, Data, FreeDataCallback, Op, VMStatus, VM};

use crate::{CompileError, CompileErrorList};

pub struct UserData<'vm> {
    pub futures: RefCell<FuturesUnordered<NeptuneFuture<'vm>>>,
}

type NeptuneFuture<'vm> =
    Pin<Box<dyn Future<Output = (Box<dyn FnOnce(EFuncContext) -> bool>, TaskHandle<'vm>)> + 'vm>>;

impl VM {
    pub fn create_efunc_safe<F>(&self, name: &str, callback: F) -> bool
    where
        F: FnMut(EFuncContext) -> bool + 'static,
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

    pub fn create_efunc_async<F, Fut, T1, T2>(&self, name: &str, callback: F) -> bool
    where
        F: (FnMut(&mut EFuncContext) -> Fut) + 'static,
        Fut: Future<Output = Result<T1, T2>> + 'static,
        T1: ToNeptuneValue + 'static,
        T2: ToNeptuneValue + 'static,
    {
        unsafe {
            self.create_efunc(
                name.into(),
                async_trampoline::<F, Fut, T1, T2> as *mut ffi::EFuncCallback,
                Box::into_raw(Box::new(callback)) as *mut ffi::Data,
                free_data::<F> as *mut ffi::FreeDataCallback,
            )
        }
    }
}

impl<'vm> TaskHandle<'vm> {
    pub fn resume_safe<F>(&mut self, callback: F) -> VMStatus
    where
        F: FnOnce(EFuncContext) -> bool + 'static,
    {
        unsafe {
            self.resume(
                resume_trampoline::<F> as *mut ffi::EFuncCallback,
                Box::into_raw(Box::new(callback)) as *mut ffi::Data,
            )
        }
    }
}

// data must contain a valid pointer to a callback of type F
unsafe extern "C" fn trampoline<F>(cx: EFuncContext, data: *mut c_void) -> VMStatus
where
    F: FnMut(EFuncContext) -> bool + 'static,
{
    let callback = &mut *(data as *mut F);
    // https://github.com/rust-lang/rust/issues/52652#issuecomment-695034481
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(cx)))
        .unwrap_or_else(|_| std::process::abort())
    {
        VMStatus::Success
    } else {
        VMStatus::Error
    }
}

// data must contain a valid pointer to a callback of type F
unsafe extern "C" fn resume_trampoline<F>(cx: EFuncContext, data: *mut c_void) -> VMStatus
where
    F: FnOnce(EFuncContext) -> bool + 'static,
{
    let callback = Box::from_raw(data as *mut F);
    // https://github.com/rust-lang/rust/issues/52652#issuecomment-695034481
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| callback(cx)))
        .unwrap_or_else(|_| std::process::abort())
    {
        VMStatus::Success
    } else {
        VMStatus::Error
    }
}

// data must contain a valid pointer to a callback of type F
unsafe extern "C" fn async_trampoline<F, Fut, T1, T2>(
    mut cx: EFuncContext,
    data: *mut c_void,
) -> VMStatus
where
    F: (FnMut(&mut EFuncContext) -> Fut) + 'static,
    Fut: Future<Output = Result<T1, T2>> + 'static,
    T1: ToNeptuneValue + 'static,
    T2: ToNeptuneValue + 'static,
{
    let callback = &mut *(data as *mut F);
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let fut = callback(&mut cx);
        register_future(&cx, fut);
    }))
    .unwrap_or_else(|_| std::process::abort());
    VMStatus::Suspend
}

fn register_future<T1, T2>(cx: &EFuncContext, fut: impl Future<Output = Result<T1, T2>> + 'static)
where
    T1: ToNeptuneValue + 'static,
    T2: ToNeptuneValue + 'static,
{
    let vm = cx.vm();
    let user_data = vm.get_user_data();
    let task = vm.get_current_task();
    let fut = async move {
        let closure: Box<dyn FnOnce(EFuncContext) -> bool> = match fut.await {
            Ok(value) => Box::new(move |mut ctx| {
                value.to_neptune_value(&mut ctx);
                true
            }),
            Err(value) => Box::new(move |mut ctx| {
                value.to_neptune_value(&mut ctx);
                false
            }),
        };
        (closure, task)
    };
    user_data.futures.borrow_mut().push(Box::pin(fut));
}

// data must contain a valid pointer to a boxed callback of type F and must only be called once
pub unsafe extern "C" fn free_data<F>(data: *mut c_void) {
    Box::from_raw(data as *mut F);
}

#[derive(Debug)]
pub enum EFuncError {
    TypeError,
    PropertyError,
    OutOfBoundsError,
    Underflow,
    ResourceClosed,
}

impl std::fmt::Display for EFuncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EFuncError::{:?}", self)
    }
}

impl std::error::Error for EFuncError {}

#[repr(transparent)]
pub struct EFuncContext<'a>(EFuncContextInner<'a>);

#[repr(C)]
struct ThinAny<T: 'static> {
    type_id: TypeId,
    data: T,
}

impl<'a> EFuncContext<'a> {
    /// Pushes an int to the stack
    pub fn int(&mut self, i: i32) {
        self.0.push_int(i)
    }

    /// Pushes a float to the stack
    pub fn float(&mut self, f: f64) {
        self.0.push_float(f)
    }

    /// Pushes a bool to the stack
    pub fn bool(&mut self, b: bool) {
        self.0.push_bool(b)
    }

    /// Pushes null to the stack
    pub fn null(&mut self) {
        self.0.push_null()
    }

    /// Pushes a string to the stack
    pub fn string(&mut self, s: &str) {
        self.0.push_string(s.into())
    }

    /// Pushes a symbol to the stack
    pub fn symbol(&mut self, s: &str) {
        self.0.push_symbol(s.into())
    }

    /// Pushes an empty array to the stack
    pub fn array(&mut self) {
        self.0.push_empty_array()
    }

    /// Pops a value from the stack and pushes it to the array at the top of the stack
    pub fn push_to_array(&mut self) -> Result<(), EFuncError> {
        match self.0.push_to_array() {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pushes an empty object instance to the stack
    pub fn object(&mut self) {
        self.0.push_empty_object()
    }

    /// Pops a value from the stack and sets the property prop for the object instance at the top of the stack as the popped value
    pub fn set_object_property(&mut self, prop: &str) -> Result<(), EFuncError> {
        match self.0.set_object_property(prop.into()) {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pops an int from the stack
    pub fn as_int(&mut self) -> Result<i32, EFuncError> {
        let mut i = 0;
        match self.0.as_int(&mut i) {
            EFuncStatus::Ok => Ok(i),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pops a float from the stack
    pub fn as_float(&mut self) -> Result<f64, EFuncError> {
        let mut f = 0.0;
        match self.0.as_float(&mut f) {
            EFuncStatus::Ok => Ok(f),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pops a bool from the stack
    pub fn as_bool(&mut self) -> Result<bool, EFuncError> {
        let mut b = false;
        match self.0.as_bool(&mut b) {
            EFuncStatus::Ok => Ok(b),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pops a value from the stack and checks if it is null
    pub fn is_null(&mut self) -> Result<bool, EFuncError> {
        match self.0.is_null() {
            EFuncStatus::Ok => Ok(true),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Ok(false),
            _ => unreachable!(),
        }
    }

    /// Pops a string from the top of the stack
    pub fn as_string(&mut self) -> Result<&str, EFuncError> {
        let mut s = StringSlice::from("");
        match self.0.as_string(&mut s) {
            EFuncStatus::Ok => Ok(s.as_str()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pops a symbol from the top of the stack
    pub fn as_symbol(&mut self) -> Result<&str, EFuncError> {
        let mut s = StringSlice::from("");
        match self.0.as_symbol(&mut s) {
            EFuncStatus::Ok => Ok(s.as_str()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Gets the length of the array at the top of the stack
    pub fn array_length(&mut self) -> Result<usize, EFuncError> {
        let mut size = 0;
        match self.0.get_array_length(&mut size) {
            EFuncStatus::Ok => Ok(size),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pushes the value at `index` of the array at the top of the stack
    pub fn get_element(&mut self, index: usize) -> Result<(), EFuncError> {
        match self.0.get_array_element(index) {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            EFuncStatus::OutOfBoundsError => Err(EFuncError::OutOfBoundsError),
            _ => unreachable!(),
        }
    }

    /// Pushes the value of property prop of the object at the top of the stack
    pub fn get_property(&mut self, prop: &str) -> Result<(), EFuncError> {
        match self.0.get_object_property(prop.into()) {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            EFuncStatus::PropertyError => Err(EFuncError::PropertyError),
            _ => unreachable!(),
        }
    }

    /// Pops the top of the stack
    pub fn pop(&mut self) -> Result<(), EFuncError> {
        if self.0.pop() {
            Ok(())
        } else {
            Err(EFuncError::Underflow)
        }
    }

    /// Pushes an empty map to the stack
    pub fn map(&mut self) {
        self.0.push_empty_map()
    }

    /// Pops the value and then key from the stack and inserts it in the map
    /// at the top of the stack
    pub fn insert_in_map(&mut self) -> Result<(), EFuncError> {
        match self.0.insert_in_map() {
            EFuncStatus::Ok => Ok(()),
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }

    /// Pushes an error with class `error_class` within module `module` and with message `message`
    pub fn error(
        &mut self,
        module: &str,
        error_class: &str,
        message: &str,
    ) -> Result<(), EFuncError> {
        match self
            .0
            .push_error(module.into(), error_class.into(), message.into())
        {
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            EFuncStatus::Ok => Ok(()),
            _ => unreachable!(),
        }
    }

    // Function must contain valid bytecode
    pub(crate) unsafe fn function(&mut self, fw: FunctionInfoWriter) {
        self.0.push_function(fw)
    }

    pub(crate) fn vm(&self) -> &'a VM {
        self.0.get_vm()
    }

    pub fn resource<T: 'static>(&mut self, t: T) {
        let data = Box::into_raw(Box::new(ThinAny {
            type_id: TypeId::of::<T>(),
            data: t,
        }));
        unsafe {
            self.0.push_resource(
                data as *mut Data,
                free_data::<ThinAny<T>> as *mut FreeDataCallback,
            );
        }
    }

    pub fn as_resource<T: 'static>(&mut self) -> Result<&mut T, EFuncError> {
        let mut status = EFuncStatus::Ok;
        let data = self.0.as_resource(&mut status);
        match status {
            EFuncStatus::Ok => {
                if data.is_null() {
                    Err(EFuncError::ResourceClosed)
                } else {
                    unsafe {
                        if *(data as *mut TypeId) == TypeId::of::<T>() {
                            Ok(&mut (*(data as *mut ThinAny<T>)).data)
                        } else {
                            Err(EFuncError::TypeError)
                        }
                    }
                }
            }
            EFuncStatus::Underflow => Err(EFuncError::Underflow),
            EFuncStatus::TypeError => Err(EFuncError::TypeError),
            _ => unreachable!(),
        }
    }
}

/// Types that can be converted to Neptune values implement this trait
/// Example:
/// ```
/// use neptune_lang::*;
///
/// struct Point {
///     x: i32,
///     y: i32
/// }
///
/// impl ToNeptuneValue for Point {
///     fn to_neptune_value(self, cx: &mut EFuncContext) {
///         cx.object();    // push an empty object to the stack
///         cx.int(self.x); // push self.x to the stack
///         cx.set_object_property("x").unwrap(); // pop self.x and set it as property x
///         self.y.to_neptune_value(cx); // an alternate way to push to the stack
///         cx.set_object_property("y").unwrap();
///     }
/// }
/// ```
pub trait ToNeptuneValue {
    /// Pushes the value on the stack
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

impl ToNeptuneValue for EFuncError {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.error(
            "<prelude>",
            "EFuncError",
            match self {
                EFuncError::TypeError => "TypeError",
                EFuncError::PropertyError => "PropertyError",
                EFuncError::OutOfBoundsError => "OutOfBoundsError",
                EFuncError::Underflow => "Underflow",
                EFuncError::ResourceClosed => "ResourceClosed",
            },
        )
        .unwrap();
    }
}

impl ToNeptuneValue for CompileError {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.object();
        cx.int(self.line as i32);
        cx.set_object_property("line").unwrap();
        cx.string(&self.message);
        cx.set_object_property("message").unwrap();
    }
}

impl ToNeptuneValue for CompileErrorList {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        use std::fmt::Write;
        let mut message = "".to_owned();
        writeln!(message, "In module {}", &self.module).unwrap();
        for c in &self.errors {
            writeln!(message, "{}", c).unwrap();
        }
        message.pop();
        cx.error("<prelude>", "CompileError", &message).unwrap();
        self.errors.to_neptune_value(cx);
        cx.set_object_property("errors").unwrap();
        cx.string(&self.module);
        cx.set_object_property("module").unwrap();
    }
}

/// Represents a resource. It can be used efuncs that return resources
pub struct Resource<T: 'static>(pub T);

impl<T: 'static> ToNeptuneValue for Resource<T> {
    fn to_neptune_value(self, cx: &mut EFuncContext) {
        cx.resource(self.0)
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

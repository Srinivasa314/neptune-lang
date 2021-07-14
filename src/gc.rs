use rustc_hash::FxHashSet;
use std::{any::TypeId, borrow::Borrow, cell::Cell, hash::Hash, marker::PhantomData};

use crate::value::Value;

use self::stack::{BasePointer, Stack, StackOverflowError};
mod stack;

pub struct GCAllocator {
    bytes_allocated: usize,
    constants: Vec<Object<'static>>,
    first: *mut ObjectHeader,
    threshold: usize,
    symbol_table: SymbolTable,
    stack: Stack,
    globals: Vec<Cell<Value<'static>>>,
    accumulator: Value<'static>,
}

impl Default for GCAllocator {
    fn default() -> Self {
        Self::new()
    }
}

impl GCAllocator {
    pub fn new() -> Self {
        Self {
            bytes_allocated: 0,
            constants: vec![],
            first: std::ptr::null_mut(),
            threshold: 1024 * 1024,
            symbol_table: SymbolTable::default(),
            stack: Stack::new(),
            globals: vec![],
            accumulator: Value::null(),
        }
    }

    pub fn get_accum<'a>(&'a self) -> Value<'a> {
        self.accumulator
    }

    pub fn set_accum<'a, 'b>(&'a mut self, v: Value<'b>) {
        unsafe { self.accumulator = v.make_static() }
    }

    pub fn allocate<T: ObjectTrait>(&mut self, t: T) {
        let o = self.allocate_internal(t);
        self.set_accum(Value::from_object(Object::from_header(unsafe {
            &mut *(o as *mut ObjectHeader)
        })))
    }
    fn collect(&mut self) {
        todo!()
    }

    fn allocate_internal<T: ObjectTrait>(&mut self, t: T) -> *mut ObjectInner<T> {
        self.bytes_allocated += std::mem::size_of::<ObjectInner<T>>();
        if self.bytes_allocated > self.threshold {
            self.collect();
        }
        let o = Box::into_raw(Box::new(ObjectInner {
            inner: t,
            header: ObjectHeader {
                type_id: TypeId::of::<T>(),
                is_dark: false,
                next: self.first,
            },
        }));
        self.first = o as *mut ObjectHeader;
        o
    }

    pub fn make_constant<T: ObjectTrait>(&mut self, t: T) -> TypedObject<'static, T> {
        let t = TypedObject {
            inner: self.allocate_internal(t),
            _marker: PhantomData,
        };
        self.constants.push(t.into());
        t
    }
    pub fn get_bp(&self) -> BasePointer {
        self.stack.get_bp()
    }

    pub unsafe fn set_bp(&self, bp: BasePointer) {
        self.stack.set_bp(bp)
    }

    pub fn extend_bp(&self, by: u16) -> Result<(), StackOverflowError> {
        self.stack.extend_bp(by)
    }

    // The lifetime is 'static as it is stored in the stack or the constants immediately
    fn intern_internal(&mut self, s: &str) -> TypedObject<'static, Symbol> {
        match self.symbol_table.inner.get(s) {
            Some(e) => TypedObject {
                inner: e.0.inner,
                _marker: PhantomData,
            },
            None => TypedObject {
                inner: self.allocate_internal(Symbol(NString::from(s))),
                _marker: PhantomData,
            },
        }
    }

    pub fn string_constant(&mut self, s: &str) -> Value<'static> {
        if let Some(t) = self.constants.iter().find(|o| {
            if let Some(ns) = o.cast::<NString>() {
                ns == s
            } else {
                false
            }
        }) {
            Value::from_object(t.as_typed_object::<NString>().unwrap().into())
        } else {
            Value::from_object(self.make_constant(NString::from(s)).into())
        }
    }

    pub fn intern_constant(&mut self, s: &str) -> TypedObject<'static, Symbol> {
        let sym = self.intern_internal(s);
        if !self.constants.iter().any(|o| *o == Object::from(sym)) {
            self.constants.push(sym.into());
        }
        sym
    }

    // This is safe as long as a valid index is passed
    pub unsafe fn get_global<'a>(&'a self, index: u32) -> Option<Value<'a>> {
        debug_assert!(index < self.globals.len() as u32);
        let v = self.globals.get_unchecked(index as usize);
        if v.get().is_empty() {
            None
        } else {
            Some(v.get())
        }
    }

    // This is safe as long as a valid index is passed
    pub unsafe fn set_global<'a, 'b>(&'a self, index: u32, v: Value<'b>) {
        debug_assert!(index < self.globals.len() as u32);
        self.globals
            .get_unchecked(index as usize)
            .set(v.make_static());
    }

    pub unsafe fn getr<'a>(&'a self, index: u8) -> Value<'a> {
        self.stack.getr(index)
    }

    pub unsafe fn setr<'a, 'b>(&'a self, index: u8, v: Value<'b>) {
        self.stack.setr(index, v.make_static());
    }
}

//TODO: Include class
pub struct ObjectHeader {
    type_id: TypeId,
    is_dark: bool,
    next: *mut ObjectHeader,
}

#[derive(Debug, Clone, Copy)]
pub struct Object<'a> {
    inner: *mut ObjectHeader,
    _marker: PhantomData<&'a ObjectHeader>,
}

pub struct TypedObject<'a, T: ObjectTrait> {
    inner: *mut ObjectInner<T>,
    _marker: PhantomData<&'a ObjectInner<T>>,
}

impl<'a, T: ObjectTrait> Clone for TypedObject<'a, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<'a, T: ObjectTrait> Copy for TypedObject<'a, T> {}

// To safely implement this trait put a unique TYPE_ID and properly implement the trace method
pub unsafe trait ObjectTrait: 'static {
    //Include trace method in future
}

#[repr(C)]
struct ObjectInner<T: ObjectTrait> {
    header: ObjectHeader,
    inner: T,
}

impl<'a> Object<'a> {
    pub fn from_header(o: &'a mut ObjectHeader) -> Object<'a> {
        Object {
            inner: o,
            _marker: PhantomData,
        }
    }

    pub fn as_raw_ptr(self) -> *mut ObjectHeader {
        self.inner
    }

    pub fn cast<T: ObjectTrait>(self) -> Option<&'a T> {
        unsafe {
            if self.inner.read().type_id == TypeId::of::<T>() {
                Some(&(*self.inner.cast::<ObjectInner<T>>()).inner)
            } else {
                None
            }
        }
    }

    pub fn as_typed_object<T: ObjectTrait>(self) -> Option<TypedObject<'a, T>> {
        unsafe {
            if self.inner.read().type_id == TypeId::of::<T>() {
                Some(TypedObject {
                    inner: self.inner.cast::<ObjectInner<T>>(),
                    _marker: PhantomData,
                })
            } else {
                None
            }
        }
    }

    pub fn ptr_eq<'b>(self, o: Object<'b>) -> bool {
        self.as_raw_ptr() == o.as_raw_ptr()
    }
}

impl<'a> PartialEq for Object<'a> {
    fn eq<'b>(&self, other: &Object<'b>) -> bool {
        todo!()
    }
}

impl<'a> Eq for Object<'a> {}

impl<'a, T: ObjectTrait> From<TypedObject<'a, T>> for Object<'a> {
    fn from(tobj: TypedObject<'a, T>) -> Object<'a> {
        Self {
            inner: tobj.as_raw_ptr(),
            _marker: PhantomData,
        }
    }
}

impl<'a, T: ObjectTrait> TypedObject<'a, T> {
    fn as_ref(self) -> &'a T {
        unsafe { &(*self.inner).inner }
    }

    fn as_raw_ptr(self) -> *mut ObjectHeader {
        self.inner as *mut ObjectHeader
    }
}

pub type NString = smartstring::SmartString<smartstring::LazyCompact>;

unsafe impl ObjectTrait for NString {}

pub struct Symbol(NString);

unsafe impl ObjectTrait for Symbol {}

#[derive(Default)]
struct SymbolTable {
    inner: FxHashSet<SymbolTableEntry>,
}

#[derive(Clone, Copy)]
struct SymbolTableEntry(TypedObject<'static, Symbol>);

impl Hash for SymbolTableEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.as_ref().0.hash(state)
    }
}

impl PartialEq for SymbolTableEntry {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ref().0 == other.0.as_ref().0
    }
}

impl Eq for SymbolTableEntry {}

impl Borrow<str> for SymbolTableEntry {
    fn borrow<'a>(&'a self) -> &'a str {
        &*self.0.as_ref().0
    }
}

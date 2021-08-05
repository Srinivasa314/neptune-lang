use std::{
    borrow::Borrow,
    cell::{Cell, UnsafeCell},
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
};

use rustc_hash::FxHashMap;

use crate::{
    objects::{NString, NSymbol},
    util::unreachable,
    value::{RootedValue, Value},
};

pub struct GC {
    bytes_allocated: Cell<usize>,
    constants: UnsafeCell<Vec<*mut ObjectHeader>>,
    first: Cell<*mut ObjectHeader>,
    threshold: usize,
    symbol_table: UnsafeCell<SymbolTable>,
    in_use: Cell<bool>,
}

impl GC {
    pub fn new() -> Self {
        Self {
            bytes_allocated: Cell::new(0),
            constants: UnsafeCell::new(vec![]),
            first: Cell::new(std::ptr::null_mut()),
            threshold: 1024 * 1024,
            symbol_table: UnsafeCell::new(SymbolTable::default()),
            in_use: Cell::new(false),
        }
    }

    fn collect(&self) {
        todo!()
    }

    fn allocate_internal<T: ObjectTrait>(&self, t: T) -> *mut ObjectInner<T> {
        self.bytes_allocated
            .set(self.bytes_allocated.get() + std::mem::size_of::<ObjectInner<T>>());

        let o = Box::into_raw(Box::new(ObjectInner {
            inner: t,
            header: ObjectHeader {
                type_id: T::type_id,
                is_dark: false,
                next: self.first.get(),
            },
        }));
        self.first.set(o as *mut ObjectHeader);
        o
    }

    pub fn alloc_constant<'gc, T: ObjectTrait>(&'gc self, t: T) -> TypedObject<'gc, T> {
        let raw_o = self.allocate_internal(t);
        let o = TypedObject {
            inner: raw_o,
            _marker: PhantomData,
        };
        unsafe { &mut *self.constants.get() }.push(raw_o as *mut ObjectHeader);
        o
    }

    pub fn intern_temp_symbol(&self, s: &str) {
        todo!()
    }

    pub fn intern_constant_symbol<'gc>(&'gc self, s: &str) -> TypedObject<'gc, NSymbol> {
        let symbol_table = unsafe { &mut *self.symbol_table.get() };
        if let Some((sym, stype)) = symbol_table.inner.get_key_value(s) {
            if stype.get() == SymbolType::Temporary {
                stype.set(SymbolType::Constant)
            }
            TypedObject {
                inner: sym.0,
                _marker: PhantomData,
            }
        } else {
            let raw_sym = self.allocate_internal(NSymbol::from_string(NString::from(s)));
            symbol_table
                .inner
                .insert(SymbolTableEntry(raw_sym), Cell::new(SymbolType::Constant));
            TypedObject {
                inner: raw_sym,
                _marker: PhantomData,
            }
        }
    }
}

pub struct GCSession<'gc>(pub &'gc GC);

impl<'gc> GCSession<'gc> {
    pub fn new(gc: &'gc GC) -> Self {
        if gc.in_use.get() {
            panic!("Cannot create a GCSession while another exists");
        } else {
            gc.in_use.set(true);
            Self(gc)
        }
    }

    pub fn allocate<T: ObjectTrait, R: Root>(&self, t: T, root: &R) -> RootedValue<'gc> {
        if self.0.bytes_allocated.get() > self.0.threshold {
            self.0.collect();
        }
        let o = self.0.allocate_internal(t);
        RootedValue::from(Value::from_object(Object::from_header(unsafe {
            &mut *(o as *mut ObjectHeader)
        })))
    }
}

impl<'gc> Drop for GCSession<'gc> {
    fn drop(&mut self) {
        self.0.in_use.set(false)
    }
}

// Todo include trace trait
// and explain why it is unsafe
pub unsafe trait Root {}

//TODO: Include class
pub(crate) struct ObjectHeader {
    type_id: TypeId,
    is_dark: bool,
    next: *mut ObjectHeader,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TypeId {
    NString,
    NSymbol,
}

#[derive(Clone, Copy)]
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

// To safely implement this trait properly implement the trace method
pub unsafe trait ObjectTrait {
    //TODO: Include trace method in future
    const type_id: TypeId;
}

#[repr(C)]
struct ObjectInner<T: ObjectTrait> {
    header: ObjectHeader,
    inner: T,
}

impl<'a> Object<'a> {
    pub(crate) fn from_header(o: &'a mut ObjectHeader) -> Object<'a> {
        Object {
            inner: o,
            _marker: PhantomData,
        }
    }

    pub(crate) fn as_raw_ptr(self) -> *mut ObjectHeader {
        self.inner
    }

    pub fn cast<T: ObjectTrait>(self) -> Option<&'a T> {
        unsafe {
            if (*self.inner).type_id == T::type_id {
                Some(&(*self.inner.cast::<ObjectInner<T>>()).inner)
            } else {
                None
            }
        }
    }

    pub fn as_typed_object<T: ObjectTrait>(self) -> Option<TypedObject<'a, T>> {
        unsafe {
            if (*self.inner).type_id == T::type_id {
                Some(TypedObject {
                    inner: self.inner.cast::<ObjectInner<T>>(),
                    _marker: PhantomData,
                })
            } else {
                None
            }
        }
    }

    pub fn is<T: ObjectTrait>(self) -> bool {
        unsafe { (*self.inner).type_id == T::type_id }
    }

    pub fn ptr_eq(self, o: Object) -> bool {
        self.as_raw_ptr() == o.as_raw_ptr()
    }

    // todo: change this when more types added
    pub fn type_string(self) -> &'static str {
        match unsafe { (*self.inner).type_id } {
            TypeId::NString => "string",
            TypeId::NSymbol => "symbol",
        }
    }
}

impl<'a> Debug for Object<'a> {
    // todo: change this when more types added
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ns) = self.cast::<NString>() {
            write!(f, "{:?}", ns)
        } else if let Some(sym) = self.cast::<NSymbol>() {
            write!(f, "@{}", sym.to_string())
        } else {
            unreachable()
        }
    }
}

impl<'a> Display for Object<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

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

#[derive(Default)]
struct SymbolTable {
    inner: FxHashMap<SymbolTableEntry, Cell<SymbolType>>,
}

struct SymbolTableEntry(*mut ObjectInner<NSymbol>);

#[derive(Clone, Copy, PartialEq, Eq)]
enum SymbolType {
    Constant,
    Temporary,
}

impl Hash for SymbolTableEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        unsafe { (*self.0).inner.to_string().hash(state) }
    }
}

impl PartialEq for SymbolTableEntry {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (*self.0).inner.to_string() == (*other.0).inner.to_string() }
    }
}

impl Eq for SymbolTableEntry {}

impl Borrow<str> for SymbolTableEntry {
    fn borrow(&self) -> &str {
        &unsafe { &*self.0 }.inner.to_string()
    }
}

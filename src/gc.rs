use std::{
    any::TypeId,
    borrow::Borrow,
    cell::{Cell, RefCell},
    hash::Hash,
    marker::PhantomData,
};

use rustc_hash::FxHashMap;

use crate::value::{RootedValue, Value};

pub struct GC {
    bytes_allocated: Cell<usize>,
    constants: RefCell<Vec<*mut ObjectHeader>>,
    first: Cell<*mut ObjectHeader>,
    threshold: usize,
    symbol_table: RefCell<SymbolTable>,
}

impl GC {
    pub fn new() -> Self {
        Self {
            bytes_allocated: Cell::new(0),
            constants: RefCell::new(vec![]),
            first: Cell::new(std::ptr::null_mut()),
            threshold: 1024 * 1024,
            symbol_table: RefCell::new(SymbolTable::default()),
        }
    }

    pub fn allocate<'gc, T: ObjectTrait, R: Root>(&'gc self, t: T, root: &R) -> RootedValue<'gc> {
        if self.bytes_allocated.get() > self.threshold {
            self.collect();
        }
        let o = self.allocate_internal(t);
        (Value::from_object(Object::from_header(unsafe {
            &mut *(o as *mut ObjectHeader)
        })))
        .into()
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
                type_id: TypeId::of::<T>(),
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
        self.constants.borrow_mut().push(raw_o as *mut ObjectHeader);
        o
    }

    pub fn intern_temp_symbol(&self, s: &str) {
        todo!()
    }

    pub fn intern_constant_symbol<'gc>(&'gc self, s: &str) -> TypedObject<'gc, NSymbol> {
        let mut symbol_table = self.symbol_table.borrow_mut();
        if let Some((sym, stype)) = symbol_table.inner.get_key_value(s) {
            if stype.get() == SymbolType::Temporary {
                stype.set(SymbolType::Constant)
            }
            TypedObject {
                inner: sym.0,
                _marker: PhantomData,
            }
        } else {
            let raw_sym = self.allocate_internal(NSymbol(NString::from(s)));
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

// Todo include trace trait
// and explain why it is unsafe
pub unsafe trait Root {}

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

// To safely implement this trait properly implement the trace method
pub unsafe trait ObjectTrait: 'static {
    //TODO: Include trace method in future
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
            if (*self.inner).type_id == TypeId::of::<T>() {
                Some(&(*self.inner.cast::<ObjectInner<T>>()).inner)
            } else {
                None
            }
        }
    }

    pub fn as_typed_object<T: ObjectTrait>(self) -> Option<TypedObject<'a, T>> {
        unsafe {
            if (*self.inner).type_id == TypeId::of::<T>() {
                Some(TypedObject {
                    inner: self.inner.cast::<ObjectInner<T>>(),
                    _marker: PhantomData,
                })
            } else {
                None
            }
        }
    }

    pub fn ptr_eq(self, o: Object) -> bool {
        self.as_raw_ptr() == o.as_raw_ptr()
    }
}

impl<'a> PartialEq for Object<'a> {
    fn eq(&self, other: &Object) -> bool {
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

// In future have multiple representations (like rope) ?
pub type NString = smartstring::SmartString<smartstring::LazyCompact>;

unsafe impl ObjectTrait for NString {}

pub struct NSymbol(NString);

unsafe impl ObjectTrait for NSymbol {}

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
        unsafe { (*self.0).inner.0.hash(state) }
    }
}

impl PartialEq for SymbolTableEntry {
    fn eq(&self, other: &Self) -> bool {
        unsafe { (*self.0).inner.0 == (*other.0).inner.0 }
    }
}

impl Eq for SymbolTableEntry {}

impl Borrow<str> for SymbolTableEntry {
    fn borrow(&self) -> &str {
        &unsafe { &*self.0 }.inner.0
    }
}

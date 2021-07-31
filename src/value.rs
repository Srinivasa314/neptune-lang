use crate::{gc::Object, gc::{NString, ObjectHeader}, util::unreachable};
use std::marker::PhantomData;

// Values are represented by NaN Boxing.
// The lifetime of the value is the 'static if it is a number,bool,etc.
// Otherwise its lifetime is that of the gc.For unrooted values the
//lifetime will be less than the lifetime of the gc as it may be collected
// when the next allocation  happens.
#[derive(Clone, Copy)]
#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
pub struct Value<'a>(u64, PhantomData<Object<'a>>);
#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
#[derive(Clone, Copy)] //new
pub enum Value<'a> {
    I32(i32),
    F64(f64),
    Object(Object<'a>),
    True,
    False,
    Empty,
    Null,
}

const NANBOX_MIN_NUMBER: u64 = 0x0006000000000000;
const NANBOX_HIGH16_TAG: u64 = 0xffff000000000000;
const NANBOX_DOUBLE_ENCODE_OFFSET: u64 = 0x0007000000000000;
const NANBOX_VALUE_FALSE: u64 = 0x06;
const NANBOX_VALUE_TRUE: u64 = 0x07;
const NANBOX_VALUE_EMPTY: u64 = 0x0;
const NANBOX_VALUE_DELETED: u64 = 0x5;
const NANBOX_VALUE_NULL: u64 = 0x02;
const NANBOX_MASK_POINTER: u64 = 0x0000fffffffffffc;

#[cfg(any(target_arch = "aarch64", target_arch = "x86_64"))]
impl<'a> Value<'a> {
    pub fn from_i32(i: i32) -> Value<'static> {
        Value(NANBOX_MIN_NUMBER | (i as u64), PhantomData)
    }
    pub fn from_f64(f: f64) -> Value<'static> {
        Value(f.to_bits() + NANBOX_DOUBLE_ENCODE_OFFSET, PhantomData)
    }
    pub fn from_bool(b: bool) -> Value<'static> {
        Value(
            if b {
                NANBOX_VALUE_TRUE
            } else {
                NANBOX_VALUE_FALSE
            },
            PhantomData,
        )
    }
    pub fn empty() -> Value<'static> {
        Value(NANBOX_VALUE_EMPTY, PhantomData)
    }
    pub fn null() -> Value<'static> {
        Value(NANBOX_VALUE_NULL, PhantomData)
    }
    pub fn from_object(o: Object) -> Self {
        Self(o.as_raw_ptr() as u64, PhantomData)
    }
    pub fn new_true() -> Value<'static> {
        Value(NANBOX_VALUE_TRUE, PhantomData)
    }
    pub fn new_false() -> Value<'static> {
        Value(NANBOX_VALUE_FALSE, PhantomData)
    }
    pub fn is_number(self) -> bool {
        self.0 >= NANBOX_MIN_NUMBER
    }
    pub fn is_i32(self) -> bool {
        (self.0 & NANBOX_HIGH16_TAG) == NANBOX_MIN_NUMBER
    }
    pub fn as_i32(self) -> Option<i32> {
        if self.is_i32() {
            Some(self.0 as i32)
        } else {
            None
        }
    }
    pub fn is_f64(self) -> bool {
        self.is_number() && !self.is_i32()
    }
    pub fn as_f64(self) -> Option<f64> {
        if self.is_f64() {
            Some(f64::from_bits(self.0 - NANBOX_DOUBLE_ENCODE_OFFSET))
        } else {
            None
        }
    }
    pub fn is_bool(self) -> bool {
        (self.0 & !1) == NANBOX_VALUE_FALSE
    }
    pub fn as_bool(self) -> Option<bool> {
        if self.is_bool() {
            Some((self.0 & 1) != 0)
        } else {
            None
        }
    }
    pub fn is_object(self) -> bool {
        (self.0 & !NANBOX_MASK_POINTER) == 0 && (self.0 != 0)
    }
    pub fn as_object(self) -> Option<Object<'a>> {
        if self.is_object() {
            Some(unsafe { Object::from_header(&mut *(self.0 as *mut ObjectHeader)) })
        } else {
            None
        }
    }

    pub fn is_null(self) -> bool {
        self.0 == NANBOX_VALUE_NULL
    }
    pub fn is_empty(self) -> bool {
        self.0 == NANBOX_VALUE_EMPTY
    }
    pub fn is_true(self) -> bool {
        self.0 == NANBOX_VALUE_TRUE
    }
    pub fn is_false(self) -> bool {
        self.0 == NANBOX_VALUE_FALSE
    }
}

#[cfg(not(any(target_arch = "aarch64", target_arch = "x86_64")))]
impl<'a> Value<'a> {
    pub fn from_i32(i: i32) -> Value<'static> {
        Value::I32(i)
    }
    pub fn from_f64(f: f64) -> Value<'static> {
        Value::F64(f)
    }
    pub fn from_bool(b: bool) -> Value<'static> {
        if b {
            Value::True
        } else {
            Value::False
        }
    }
    pub fn empty() -> Value<'static> {
        Value::Empty
    }
    pub fn null() -> Value<'static> {
        Value::Null
    }
    pub fn from_object(o: Object<'a>) -> Self {
        Self::Object(o)
    }
    pub fn new_true() -> Value<'static> {
        Value::True
    }
    pub fn new_false() -> Value<'static> {
        Value::False
    }
    pub fn is_number(self) -> bool {
        matches!(self, Self::I32(_) | Self::F64(_)) //new
    }
    pub fn is_i32(self) -> bool {
        matches!(self, Self::I32(_))
    }
    pub fn as_i32(self) -> Option<i32> {
        if let Self::I32(i) = self {
            Some(i)
        } else {
            None
        }
    }
    pub fn is_f64(self) -> bool {
        matches!(self, Self::F64(_))
    }
    pub fn as_f64(self) -> Option<f64> {
        if let Self::F64(f) = self {
            Some(f)
        } else {
            None
        }
    }
    pub fn is_bool(self) -> bool {
        matches!(self, Self::True | Self::False)
    }
    pub fn as_bool(self) -> Option<bool> {
        match self {
            Self::True => Some(true),
            Self::False => Some(false),
            _ => None,
        }
    }
    pub fn is_object(self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn as_object(self) -> Option<Object<'a>> {
        if let Self::Object(o) = self {
            Some(o)
        } else {
            None
        }
    }

    pub fn is_null(self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_empty(self) -> bool {
        matches!(self, Self::Empty)
    }
    pub fn is_true(self) -> bool {
        matches!(self, Self::True)
    }
    pub fn is_false(self) -> bool {
        matches!(self, Self::False)
    }
}

// Used for values that are rooted. When its inner value is got it will not be rooted
// so its lifetime will be lesser than the lifetime of the gc so
// its lifetime must be reduced so that it cannot be accessed after it is freed.
#[derive(Clone, Copy)]
pub struct RootedValue<'a>(Value<'a>);

impl<'a> RootedValue<'a> {
    unsafe fn get(&self) -> Value<'a> {
        self.0
    }
}

impl<'a> From<Value<'a>> for RootedValue<'a> {
    fn from(v: Value<'a>) -> Self {
        Self(v)
    }
}

impl PartialEq for Value<'_> {
    fn eq(&self, other: &Self) -> bool {
        if let Some(i1) = self.as_i32() {
            if let Some(i2) = other.as_i32() {
                i1 == i2
            } else if let Some(f2) = other.as_f64() {
                i1 as f64 == f2
            } else {
                false
            }
        } else if let Some(f1) = self.as_f64() {
            if let Some(f2) = other.as_f64() {
                f1 == f2
            } else if let Some(i2) = other.as_i32() {
                f1 == i2 as f64
            } else {
                false
            }
        } else if self.is_null() {
            other.is_null()
        } else if let Some(o1) = self.as_object() {
            if let Some(o2) = other.as_object() {
                if let Some(s1) = o1.cast::<NString>() {
                    if let Some(s2) = o2.cast::<NString>() {
                        s1 == s2
                    } else {
                        false
                    }
                } else {
                    o1.ptr_eq(o2)
                }
            } else {
                false
            }
        } else if let Some(b1) = self.as_bool() {
            if let Some(b2) = other.as_bool() {
                b1 == b2
            } else {
                false
            }
        } else {
            unreachable()
        }
    }
}

impl Eq for Value<'_> {}

impl<'a> std::fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match () {
            _ if self.is_bool() => write!(f, "{}", self.as_bool().unwrap()),
            _ if self.is_i32() => write!(f, "{}", self.as_i32().unwrap()),
            _ if self.is_f64() => write!(f, "{}", &self.as_f64().unwrap()),
            _ if self.is_object() => write!(f, "object"),
            _ => todo!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::gc::GC;

    use super::*;
    #[test]
    fn test_value() {
        assert_eq!(Value::from_i32(42).as_i32(), Some(42));
        assert_eq!(Value::from_f64(4.2).as_f64(), Some(4.2));
        assert!(Value::from_i32(42).is_number());
        assert!(Value::from_f64(4.2).is_number());
        assert_eq!(Value::from_bool(true).as_bool(), Some(true));
        assert_eq!(Value::from_bool(false).as_bool(), Some(false));
        assert_eq!(Value::new_true().as_bool(), Some(true));
        assert_eq!(Value::new_false().as_bool(), Some(false));
        assert!(Value::empty().is_empty());
        assert!(Value::null().is_null());
        assert!(Value::new_false().is_false());
        assert!(Value::new_true().is_true());
        assert!(Value::from_bool(true).is_true());
        assert!(Value::from_bool(false).is_false());
        let gc = GC::new();
        let s = gc.alloc_constant(NString::from("abc"));
        assert!(Value::from_object(s.into()).as_object().unwrap().ptr_eq(s.into()));
    }
}


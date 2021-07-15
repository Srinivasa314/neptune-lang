use std::cell::Cell;

use crate::value::Value;

pub struct Stack {
    v: Vec<Value<'static>>,
    bp: Cell<*mut Value<'static>>,
    end: *mut Value<'static>,
}

pub struct BasePointer(*mut Value<'static>);
pub struct StackOverflowError;

impl Stack {
    pub fn new() -> Self {
        let mut v = vec![Value::empty(); 1024 * 128];
        let bp: *mut Value = v.as_mut_ptr();
        let end = unsafe { bp.add(v.len()) };
        Self {
            v,
            bp: Cell::new(bp),
            end,
        }
    }

    // The caller must ensure that a local at the given index exists on the stack
    pub unsafe fn getr(&self, index: u8) -> Value<'static> {
        let ptr = self.bp.get().add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.read()
    }

    // The caller must ensure that a local at the given index exists on the stack
    pub unsafe fn setr(&self, index: u8, v: Value<'static>) {
        let ptr = self.bp.get().add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.write(v)
    }

    pub fn get_bp(&self) -> BasePointer {
        BasePointer(self.bp.get())
    }

    pub unsafe fn set_bp(&self, bp: BasePointer) {
        self.bp.set(bp.0);
    }

    pub fn extend_bp(&self, by: u16, regcount: u16) -> Result<(), StackOverflowError> {
        let p = self.bp.get() as usize + by as usize;
        if p + regcount as usize > self.end as usize {
            Err(StackOverflowError)
        } else {
            self.bp.set(p as *mut Value<'static>);
            Ok(())
        }
    }
}

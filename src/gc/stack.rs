use std::cell::Cell;

use crate::value::Value;

pub struct Stack {
    v: Vec<Value<'static>>,
    pub bp: Cell<*mut Value<'static>>,
    //pub sp: Cell<*mut Value<'static>>,
    pub end: *mut Value<'static>,
}

impl Stack {
    pub fn new() -> Self {
        let mut v = vec![Value::empty(); 1024 * 128];
        let bp: *mut Value = v.as_mut_ptr();
        let end = unsafe { bp.add(v.len()) };
        Self {
            v,
            bp: Cell::new(bp),
            //sp: Cell::new(sp),
            end,
        }
    }

    /* // The caller must ensure that a value exists on the stack
        pub unsafe fn pop(&self) -> Value<'static> {
            debug_assert!(self.sp.get() as *const _ > self.v.as_ptr());
            self.sp.set(self.sp.get().sub(1));
            self.sp.get().read()
        }

        //The caller must ensure that amount elements exist on the stack
        pub unsafe fn pop_many(&self, amount: usize) {
            self.sp.set(self.sp.get().sub(amount))
        }


        // The caller must supply the correct arity and make sure that a function exists
        pub unsafe fn get_fun(&self, arity: u8) -> Value<'static> {
            let ptr = self.sp.get().sub(arity as usize + 1);
            debug_assert!(ptr as *const _ >= self.bp.get());
            ptr.read()
        }

        // The caller must ensure that the correct number of arguments exist on the stack
        pub unsafe fn setup_fun_call(&self, arity: u8) {
            self.bp.set(self.sp.get().sub(arity as usize));
        }
    */
    // The caller must ensure that a local at the given index exists on the stack
    pub unsafe fn get_local(&self, index: u8) -> Value<'static> {
        let ptr = self.bp.get().add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.read()
    }

    // The caller must ensure that a local at the given index exists on the stack
    pub unsafe fn set_local(&self, index: u8, v: Value<'static>) {
        let ptr = self.bp.get().add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.write(v)
    }
    /*
    pub fn push(&self, v: Value<'static>) {
        unsafe {
            if self.sp.get() == self.end {
                todo!("Throw an exception when exceptions are implemented")
            } else {
                self.sp.get().write(v);
                self.sp.set(self.sp.get().add(1))
            }
        }
    }
    */
}

/*
impl std::fmt::Debug for Stack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            write!(
                f,
                "{:?}",
                &self.v[0..((self.sp.get().offset_from(self.v.as_ptr())) as usize)]
            )
        }
    }
}
*/

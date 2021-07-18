use crate::value::Value;

pub struct Stack {
    v: Vec<Value<'static>>,
    bp: *mut Value<'static>,
    end: *mut Value<'static>,
}

pub struct BasePointer(*mut Value<'static>);
pub struct StackOverflowError;

impl Stack {
    pub fn new() -> Self {
        let mut v = vec![Value::empty(); 1024 * 128];
        let bp: *mut Value = v.as_mut_ptr();
        let end = unsafe { bp.add(v.len()) };
        Self { v, bp, end }
    }

    // The caller must ensure that a local at the given index exists on the stack
    pub unsafe fn getr(&self, index: u8) -> Value<'static> {
        let ptr = self.bp.add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.read()
    }

    // The caller must ensure that a local at the given index exists on the stack
    pub unsafe fn setr(&self, index: u8, v: Value<'static>) {
        let ptr = self.bp.add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.write(v)
    }

    pub fn get_bp(&self) -> BasePointer {
        BasePointer(self.bp)
    }

    pub unsafe fn set_bp(&mut self, bp: BasePointer) {
        self.bp = bp.0;
    }

    pub fn extend_bp(&mut self, by: u16, regcount: u16) -> Result<(), StackOverflowError> {
        let p = self.bp.wrapping_add(by as usize);
        if p.wrapping_add(regcount as usize) > self.end {
            Err(StackOverflowError)
        } else {
            self.bp = p;
            Ok(())
        }
    }
}

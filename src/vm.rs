use crate::{
    bytecode::{BytecodeReader, Op},
    gc::{self},
    value::Value,
};

struct Stack {
    v: Vec<Value<'static>>,
    bp: *mut Value<'static>,
    end: *mut Value<'static>,
}

struct BasePointer(*mut Value<'static>);
struct StackOverflowError;

impl Stack {
    fn new() -> Self {
        let mut v = vec![Value::empty(); 1024 * 128];
        let bp: *mut Value = v.as_mut_ptr();
        let end = unsafe { bp.add(v.len()) };
        Self { v, bp, end }
    }

    // The caller must ensure that a local at the given index exists on the stack
    unsafe fn getr(&self, index: u8) -> Value<'static> {
        let ptr = self.bp.add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.read()
    }

    // The caller must ensure that a local at the given index exists on the stack
    unsafe fn setr(&self, index: u8, v: Value<'static>) {
        let ptr = self.bp.add(index as usize);
        debug_assert!(ptr < self.end);
        ptr.write(v)
    }

    fn get_bp(&self) -> BasePointer {
        BasePointer(self.bp)
    }

    unsafe fn set_bp(&mut self, bp: BasePointer) {
        self.bp = bp.0;
    }

    fn extend_bp(&mut self, by: u16, regcount: u16) -> Result<(), StackOverflowError> {
        let p = self.bp.wrapping_add(by as usize);
        if p.wrapping_add(regcount as usize) > self.end {
            Err(StackOverflowError)
        } else {
            self.bp = p;
            Ok(())
        }
    }
}

struct Frame {
    br: BytecodeReader<'static>,
    bp: BasePointer,
}

//TODO: Return uncaught exception in future
fn run(gc: &mut gc::GC, mut bc: BytecodeReader) -> Result<(), String> {
    let mut frames: Vec<Frame> = Vec::with_capacity(1024);
    let mut curr_frame = frames.as_mut_ptr();
    let frames_end = unsafe { curr_frame.add(1024) };
    unsafe {
        loop {
            match bc.read_op() {
                Op::Wide => todo!(),
                Op::ExtraWide => todo!(),
                Op::LoadInt => todo!(),
                Op::LoadRegister => todo!(),
                Op::StoreR0 => todo!(),
                Op::StoreR1 => todo!(),
                Op::StoreR2 => todo!(),
                Op::StoreR3 => todo!(),
                Op::StoreR4 => todo!(),
                Op::Move => todo!(),
                Op::Increment => todo!(),
                Op::Negate => todo!(),
                Op::AddRegister => todo!(),
                Op::AddInt => todo!(),
                Op::SubtractRegister => todo!(),
                Op::SubtractInt => todo!(),
                Op::MultiplyRegister => todo!(),
                Op::MultiplyInt => todo!(),
                Op::DivideRegister => todo!(),
                Op::DivideInt => todo!(),
                Op::Less => todo!(),
                Op::LoadConstant => todo!(),
                Op::Return => todo!(),
                Op::Jump => todo!(),
                Op::JumpBack => todo!(),
                Op::JumpIfFalse => todo!(),
                Op::Call1Argument => todo!(),
                Op::GetGlobal => todo!(),
                Op::Exit => todo!(),
                Op::StoreRegister => todo!(),
                Op::Call => todo!(),
                Op::Call0Argument => todo!(),
                Op::Call2Argument => todo!(),
                Op::StoreR5 => todo!(),
                Op::StoreR6 => todo!(),
                Op::StoreR7 => todo!(),
                Op::StoreR8 => todo!(),
                Op::StoreR9 => todo!(),
                Op::StoreR10 => todo!(),
                Op::StoreR11 => todo!(),
                Op::StoreR12 => todo!(),
                Op::StoreR13 => todo!(),
                Op::StoreR14 => todo!(),
                Op::StoreR15 => todo!(),
            }
        }
    }
}

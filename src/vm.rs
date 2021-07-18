use crate::{
    bytecode::{BytecodeReader, Op},
    gc::{self, BasePointer, GCAllocator},
    value::Value,
};

struct Frame {
    br: BytecodeReader<'static>,
    bp: BasePointer,
}

//TODO: Return uncaught exception in future
pub fn run(gc: &mut gc::GCAllocator, mut bc: BytecodeReader) -> Result<(), String> {
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
                Op::ModInt => todo!(),
                Op::Less => todo!(),
                Op::LoadConstant => todo!(),
                Op::Print => todo!(),
                Op::Return => todo!(),
                Op::Jump => todo!(),
                Op::JumpBack => todo!(),
                Op::JumpIfFalse => todo!(),
                Op::Call1Argument => todo!(),
                Op::GetGlobal => todo!(),
                Op::Exit => todo!(),
            }
        }
    }
}

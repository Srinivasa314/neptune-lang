use crate::gc::BasePointer;
use crate::{
    bytecode::{BytecodeReader, Op},
    gc::{self, GCAllocator},
    value::Value,
    Function,
};

struct VM {
    gc: GCAllocator,
}

impl VM {
    pub fn run(&mut self) {
        todo!()
    }
}

pub unsafe fn run(gc: gc::GCAllocator, function: Function) -> Result<(), String> {
    let mut accumulator = Value::empty();
    let mut frames = Vec::with_capacity(1024);
    let frames: *mut (BytecodeReader, BasePointer) = frames.as_mut_ptr();
    let mut curr_frame = frames;
    let frames_end = frames.add(1024);
    let mut no_registers = function.no_registers;
    let mut bytecode = BytecodeReader::new(&function.bytecode);
    loop {
        match bytecode.read_op() {
            Op::AddRegister => {
                let lhs = accumulator;
                let rhs = gc.getr(bytecode.read_u8());
                accumulator = match (lhs.as_i32(), rhs.as_i32()) {
                    (Some(lhs), Some(rhs)) => Value::from_i32(match lhs.checked_add(rhs) {
                        Some(i) => i,
                        None => return Err("add error".to_string()),
                    }),
                    _ => match (lhs.as_f64(), rhs.as_f64()) {
                        (Some(lhs), Some(rhs)) => Value::from_f64(lhs + rhs),
                        _ => return Err("add error".to_string()),
                    },
                };
            }
            Op::AddInt => {
                let lhs = accumulator;
                let rhs = bytecode.read_i8();
                accumulator = match lhs.as_i32() {
                    Some(lhs) => Value::from_i32(match lhs.checked_add(rhs as i32) {
                        Some(i) => i,
                        None => return Err("add error".to_string()),
                    }),
                    _ => return Err("add error".to_string()),
                };
            }
            Op::SubtractInt => {
                let lhs = accumulator;
                let rhs = bytecode.read_i8();
                accumulator = match lhs.as_i32() {
                    Some(lhs) => Value::from_i32(match lhs.checked_sub(rhs as i32) {
                        Some(i) => i,
                        None => return Err("add error".to_string()),
                    }),
                    _ => return Err("add error".to_string()),
                };
            }
            Op::ModInt => {
                let lhs = accumulator;
                let rhs = bytecode.read_u8();
                accumulator = match lhs.as_i32() {
                    Some(lhs) => Value::from_i32(lhs % (rhs as i32)),
                    _ => return Err("add error".to_string()),
                };
            }
            Op::LoadInt => {
                accumulator = Value::from_i32(bytecode.read_i8() as i32);
            }
            Op::Print => match accumulator.as_i32() {
                Some(i) => println!("{}", i),
                None => match accumulator.as_f64() {
                    Some(f) => println!("{}", f),
                    None => return Err("print error".to_string()),
                },
            },
            Op::LoadConstant => accumulator = bytecode.read_value(),
            Op::Exit => return Ok(()),
            Op::Return => {
                curr_frame = curr_frame.sub(1);
                gc.set_bp(curr_frame.read().1);
                bytecode = curr_frame.read().0;
            }
            Op::Less => {
                let lhs = gc.getr(bytecode.read_u8());
                let rhs = accumulator;
                accumulator = match (lhs.as_i32(), rhs.as_i32()) {
                    (Some(lhs), Some(rhs)) => Value::from_bool(lhs < rhs),
                    _ => match (lhs.as_f64(), rhs.as_f64()) {
                        (Some(lhs), Some(rhs)) => Value::from_bool(lhs < rhs),
                        _ => return Err("add error".to_string()),
                    },
                };
            }
            Op::Jump => {
                let offset = bytecode.read_u16();
                bytecode.jump(offset);
            }
            Op::JumpBack => {
                let offset = bytecode.read_u16();
                bytecode.back_jump(offset);
            }
            Op::JumpIfFalse => {
                let offset = bytecode.read_u16();

                match accumulator.as_bool() {
                    Some(b) => {
                        if !b {
                            bytecode.jump(offset);
                        }
                    }
                    None => return Err("jmpiffalse error".to_string()),
                }
            }
            Op::Call1Argument => {
                let src = bytecode.read_u8();
                let arg0 = gc.getr(bytecode.read_u8());
                // We are sure that functions are always in the constant table
                // DONT DO THIS FOR CLOSURES,etc.
                let fun = gc.getr(src);
                match fun.as_object() {
                    Some(fun) => match fun.cast::<Function>() {
                        Some(fun) => {
                            if 1 == fun.arity {
                                curr_frame.write((bytecode, gc.get_bp()));
                                no_registers = fun.no_registers;
                                curr_frame = curr_frame.add(1);
                                if gc.extend_bp(1, no_registers as u16).is_err()
                                    || curr_frame > frames_end
                                {
                                    return Err("overflow".to_string());
                                }
                                gc.setr(0, arg0);
                                bytecode = BytecodeReader::new(&fun.bytecode);
                            } else {
                                return Err("call error arity wrong".to_string());
                            }
                        }
                        None => return Err("call error not a function".to_string()),
                    },
                    None => return Err("call error not an object".to_string()),
                }
            }
            Op::GetGlobal => {
                let index = bytecode.read_u8();
                match gc.get_global(index.into()) {
                    Some(v) => {
                        accumulator = v;
                    }
                    None => return Err("getglobal error".to_string()),
                }
            }
            Op::LoadRegister => accumulator = gc.getr(bytecode.read_u8()),
            Op::StoreR0 => {
                gc.setr(0, accumulator);
            }
            Op::StoreR1 => {
                gc.setr(1, accumulator);
            }
            Op::StoreR2 => {
                gc.setr(2, accumulator);
            }
            Op::StoreR3 => {
                gc.setr(3, accumulator);
            }
            Op::StoreR4 => {
                gc.setr(4, accumulator);
            }
            Op::Move => {
                let dest = bytecode.read_u8();
                let src = bytecode.read_u8();
                gc.setr(dest, gc.getr(src));
            }
            Op::Increment => {
                let lhs = accumulator;
                accumulator = match lhs.as_i32() {
                    Some(lhs) => Value::from_i32(match lhs.checked_add(1) {
                        Some(i) => i,
                        None => return Err("add error".to_string()),
                    }),
                    _ => return Err("add error".to_string()),
                };
            }
            Op::SubtractRegister => todo!(),
            Op::Wide => todo!(),
            Op::ExtraWide => todo!(),
            Op::MultiplyRegister => todo!(),
            Op::MultiplyInt => todo!(),
            Op::DivideRegister => todo!(),
            Op::DivideInt => todo!(),
            Op::Negate => todo!(),
        }
    }
}

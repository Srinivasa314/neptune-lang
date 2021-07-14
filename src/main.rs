pub mod bytecode;
pub mod bytecode_compiler;
pub mod gc;
pub mod parser;
pub mod scanner;
pub mod value;
pub mod vm;

use bytecode::{Bytecode, BytecodeWriter, Op};
use gc::ObjectTrait;

pub struct Function {
    bytecode: Bytecode,
    arity: u8,
    no_registers: u8,
}

unsafe impl ObjectTrait for Function {}

fn main() {
    /*
    let mut gc = gc::GCAllocator::new();
    let mut writer = BytecodeWriter::new();
    /* writer.write_op(Op::LoadI8);
    writer.write_u8(0);
    writer.write_op(Op::StoreR0);
    let start = writer.get_op_index().unwrap();
    writer.write_op(Op::LoadRegister);
    writer.write_u8(0);
    writer.write_op(Op::LoadI32);
    writer.write_i32(100);
    writer.write_op(Op::Less);
    writer.write_u8(0);
    writer.write_op(Op::JumpIfFalse);
    writer.write_u16(0);
    let jmp = writer.get_jmp_index().unwrap();
    writer.write_op(Op::LoadI32);
    writer.write_i32(123);
    writer.write_op(Op::AddI8);
    writer.write_u8(1);
    writer.write_op(Op::SubtractI8);
    writer.write_u8(1);
    writer.write_op(Op::AddI8);
    writer.write_u8(1);
    writer.write_op(Op::SubtractI8);
    writer.write_u8(1);
    writer.write_op(Op::AddI8);
    writer.write_u8(1);
    writer.write_op(Op::SubtractI8);
    writer.write_u8(1);
    writer.write_op(Op::LoadRegister);
    writer.write_u8(0);
    writer.write_op(Op::Increment);
    writer.write_op(Op::StoreR0);
    writer.write_op(Op::JumpBack);
    writer.write_u16((writer.get_jmp_index().unwrap() + 2) - start);
    writer.write_op(Op::Exit);
    writer.patch_jump(jmp, writer.get_op_index().unwrap() - (jmp + 2));*/
    writer.write_op(Op::LoadInt);
    writer.write_i8(10);
    writer.write_op(Op::StoreR0);
    writer.write_op(Op::Print);
    writer.write_op(Op::Exit);

    let bytecode = writer.bytecode();
    dbg!(&bytecode);
    unsafe {
        dbg!(run(
            gc,
            Function {
                bytecode,
                arity: 0,
                no_registers: 2,
            },
        ));
    }
    */
    let mut gc = gc::GCAllocator::new();
    let s = "(1.2+2.2+-2)*3.4";
    let tokens = scanner::Scanner::new(s).scan_tokens();
    println!("{:?}", tokens);
    let ast = parser::Parser::new(tokens.into_iter()).parse();
    println!("{:#?}", ast);
    let mut writer = BytecodeWriter::new();
    dbg!(writer.evaluate(&ast.unwrap()));
    writer.write_op(Op::Exit);
    let bytecode = writer.bytecode();
    println!("{:?}", bytecode);
    unsafe {
        dbg!(vm::run(
            gc,
            Function {
                bytecode,
                arity: 0,
                no_registers: 2,
            },
        ));
    }
}

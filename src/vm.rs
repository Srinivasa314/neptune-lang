use cxx::{type_id, ExternType};
use std::{
    ffi::c_void,
    fmt::{Debug, Display},
    marker::PhantomData,
};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct StringSlice<'a> {
    data: *const u8,
    len: usize,
    _marker: PhantomData<&'a [u8]>,
}

impl<'a> StringSlice<'a> {
    fn as_str(self) -> &'a str {
        unsafe {
            let s = std::slice::from_raw_parts(self.data, self.len);
            std::str::from_utf8_unchecked(s)
        }
    }
}

impl<'a> From<&'a str> for StringSlice<'a> {
    fn from(s: &'a str) -> Self {
        Self {
            data: s.as_ptr(),
            len: s.len(),
            _marker: PhantomData,
        }
    }
}

impl<'a> Display for StringSlice<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl Debug for FunctionInfoWriter<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_cxx_string())
    }
}

unsafe impl<'a> ExternType for StringSlice<'a> {
    type Id = type_id!("neptune_vm::StringSlice");
    type Kind = cxx::kind::Trivial;
}

#[repr(C)]
pub struct FunctionInfoWriter<'vm> {
    handle: *const c_void,
    vm: *const c_void,
    _marker: PhantomData<&'vm ()>,
}

unsafe impl<'vm> ExternType for FunctionInfoWriter<'vm> {
    type Id = type_id!("neptune_vm::FunctionInfoWriter");
    type Kind = cxx::kind::Trivial;
}

impl<'vm> Drop for FunctionInfoWriter<'vm> {
    fn drop(&mut self) {
        unsafe { self.release() }
    }
}

#[cxx::bridge(namespace = neptune_vm)]
mod ffi {
    #[repr(u8)]
    enum Op {
        Wide,
        ExtraWide,
        LoadRegister,
        LoadR0,
        LoadR1,
        LoadR2,
        LoadR3,
        LoadR4,
        LoadR5,
        LoadR6,
        LoadR7,
        LoadR8,
        LoadR9,
        LoadR10,
        LoadR11,
        LoadR12,
        LoadR13,
        LoadR14,
        LoadR15,
        LoadSmallInt,
        LoadNull,
        LoadTrue,
        LoadFalse,
        LoadConstant,
        StoreRegister,
        StoreR0,
        StoreR1,
        StoreR2,
        StoreR3,
        StoreR4,
        StoreR5,
        StoreR6,
        StoreR7,
        StoreR8,
        StoreR9,
        StoreR10,
        StoreR11,
        StoreR12,
        StoreR13,
        StoreR14,
        StoreR15,
        Move,
        LoadGlobal,
        StoreGlobal,
        LoadSubscript,
        StoreArrayUnchecked,
        StoreSubscript,
        AddRegister,
        SubtractRegister,
        MultiplyRegister,
        DivideRegister,
        ConcatRegister,
        AddInt,
        SubtractInt,
        MultiplyInt,
        DivideInt,
        Negate,
        Equal,
        NotEqual,
        StrictEqual,
        StrictNotEqual,
        GreaterThan,
        LesserThan,
        GreaterThanOrEqual,
        LesserThanOrEqual,
        Call,
        Call0Argument,
        Call1Argument,
        Call2Argument,
        ToString,
        NewArray,
        NewMap,
        EmptyArray,
        EmptyMap,
        ForLoop,
        Jump,
        JumpIfFalseOrNull,
        BeginForLoop,
        JumpBack,
        JumpConstant,
        JumpIfFalseOrNullConstant,
        BeginForLoopConstant,
        Return,
        Exit,
    }

    #[repr(u8)]
    enum VMStatus {
        Success,
        Error,
    }

    unsafe extern "C++" {
        include!("neptune-lang/neptune-vm/neptune-vm.h");
        type StringSlice<'a> = super::StringSlice<'a>;
        type Op;
        type VMResult;
        type VMStatus;
        type VM;
        type FunctionInfoWriter<'a> = super::FunctionInfoWriter<'a>;
        fn write_op(self: &mut FunctionInfoWriter, op: Op, line: u32) -> usize;
        // The bytecode should be valid
        unsafe fn run(self: &mut FunctionInfoWriter) -> UniquePtr<VMResult>;
        fn to_cxx_string(self: &FunctionInfoWriter) -> UniquePtr<CxxString>;
        fn write_u8(self: &mut FunctionInfoWriter, u: u8);
        fn write_u16(self: &mut FunctionInfoWriter, u: u16);
        fn write_u32(self: &mut FunctionInfoWriter, u: u32);
        fn float_constant(self: &mut FunctionInfoWriter, f: f64) -> Result<u16>;
        fn string_constant<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            s: StringSlice<'s>,
        ) -> Result<u16>;
        fn symbol_constant<'vm, 's>(
            self: &mut FunctionInfoWriter<'vm>,
            s: StringSlice<'s>,
        ) -> Result<u16>;
        fn int_constant(self: &mut FunctionInfoWriter, i: i32) -> Result<u16>;
        fn shrink(self: &mut FunctionInfoWriter);
        fn pop_last_op(self: &mut FunctionInfoWriter, last_op_pos: usize);
        fn set_max_registers(self: &mut FunctionInfoWriter, max_registers: u16);
        fn add_global<'vm, 's>(self: &'vm VM, name: StringSlice<'s>);
        fn new_function_info<'vm>(self: &'vm VM, name: StringSlice) -> FunctionInfoWriter<'vm>;
        fn new_vm() -> UniquePtr<VM>;
        // This must only be called by drop
        unsafe fn release(self: &mut FunctionInfoWriter);
        fn get_result<'a>(self: &'a VMResult) -> StringSlice<'a>;
        fn get_status(self: &VMResult) -> VMStatus;
        fn patch_jump(self: &mut FunctionInfoWriter, op_position: usize, jump_offset: u32);
        fn size(self: &FunctionInfoWriter) -> usize;
    }
}

pub use ffi::{new_vm, Op, VMStatus, VM};

#[cfg(test)]
mod tests {
    use crate::{InterpretError, Neptune};

    #[test]
    fn test() {
        let mut n = Neptune::new();
        assert_eq!(n.eval("null").unwrap().unwrap(), "null");
        assert_eq!(n.eval("true").unwrap().unwrap(), "true");
        assert_eq!(n.eval("false").unwrap().unwrap(), "false");
        assert_eq!(n.eval("1.0").unwrap().unwrap(), "1.0");
        assert_eq!(n.eval("1.2").unwrap().unwrap(), "1.2");
        assert!(matches!(
            n.eval("0.0/0.0").unwrap().unwrap().as_str(),
            "-nan" | "nan"
        ));
        assert_eq!(n.eval("1.0/0.0").unwrap().unwrap(), "inf");
        assert_eq!(n.eval("-1.0/0.0").unwrap().unwrap(), "-inf");
        assert_eq!(n.eval("5").unwrap().unwrap(), "5");
        assert_eq!(n.eval("-5").unwrap().unwrap(), "-5");
        assert_eq!(n.eval("1000").unwrap().unwrap(), "1000");
        assert_eq!(n.eval("-1000").unwrap().unwrap(), "-1000");
        assert_eq!(n.eval("1000000").unwrap().unwrap(), "1000000");
        assert_eq!(n.eval("-1000000").unwrap().unwrap(), "-1000000");
        assert_eq!(n.eval("'hi'").unwrap().unwrap(), "'hi'");
        assert_eq!(n.eval("'hi\n'").unwrap().unwrap(), "'hi\\n'");
        assert_eq!(n.eval("'\"'").unwrap().unwrap(), "'\"'");
        assert_eq!(n.eval("'\\''").unwrap().unwrap(), "'\\''");
        assert_eq!(n.eval("@abc").unwrap().unwrap(), "@abc");
        assert_eq!(n.eval("global=1"), Ok(None));
        assert_eq!(
            n.eval("glibal").unwrap_err(),
            InterpretError::RuntimePanic("Cannot access uninitialized variable glibal".into())
        );
        assert_eq!(n.eval("global").unwrap().unwrap(), "1");
        assert_eq!(n.eval("global=2"), Ok(None));
        n.exec("global=global+1000").unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "1002");
        assert_eq!(
            n.eval("global+2147483000").unwrap_err(),
            InterpretError::RuntimePanic(
                "Cannot add 1002 and 2147483000 as the result does not fit in an int".into()
            )
        );
        assert_eq!(n.eval("1.2+3").unwrap().unwrap(), "4.2");
        assert_eq!(
            n.eval("null+1"),
            Err(InterpretError::RuntimePanic(
                "Cannot add types null and int".into()
            ))
        );
        n.exec("{let a=1;let b=a;global=b}").unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "1");
        assert_eq!(
            n.eval("-(-2147483647-global)"),
            Err(InterpretError::RuntimePanic(
                "Cannot negate -2147483648 as the result cannot be stored in an int".into()
            ))
        );
        assert_eq!(
            n.eval("-{}"),
            Err(InterpretError::RuntimePanic(
                "Cannot negate type map".into()
            ))
        );
        assert_eq!(n.eval("-(2.3)").unwrap().unwrap(), "-2.3");
        assert_eq!(n.eval("'\\(432)'").unwrap().unwrap(), "'432'");
        assert_eq!(n.eval("'\\(-1)'").unwrap().unwrap(), "'-1'");
        assert!(matches!(
            n.eval("'\\(0.0/0.0)'").unwrap().unwrap().as_str(),
            "'-nan'" | "'nan"
        ));
        assert_eq!(n.eval("'\\(-2.0)'").unwrap().unwrap(), "'-2.0'");
        assert_eq!(n.eval("'\\(null)'").unwrap().unwrap(), "'null'");
        assert_eq!(n.eval("'\\('hello')'").unwrap().unwrap(), "'hello'");
        assert_eq!(n.eval("'\\(@bye)'").unwrap().unwrap(), "'bye'");
        assert_eq!(n.eval("@a==@a").unwrap().unwrap(), "true");
        n.exec("arr=[]").unwrap();
        assert_eq!(
            n.eval("arr[0]"),
            Err(InterpretError::RuntimePanic(
                "Array index out of range".into()
            ))
        );
        assert_eq!(
            n.eval("arr[-1]"),
            Err(InterpretError::RuntimePanic(
                "Array index out of range".into()
            ))
        );
        assert_eq!(
            n.eval("arr[0.0]"),
            Err(InterpretError::RuntimePanic(
                "Array indices must be int not float".into()
            ))
        );
        assert_eq!(n.eval("[null,true][1]").unwrap().unwrap(), "true");
        assert_eq!(
            n.exec("{let a=1000000000;let b=2000000000;let c=a+b}"),
            Err(InterpretError::RuntimePanic(
                "Cannot add 1000000000 and 2000000000 as the result does not fit in an int".into()
            ))
        );
        assert_eq!(n.eval("2.3-global").unwrap().unwrap(), "1.3");
        assert_eq!(
            n.eval("'a'-[]"),
            Err(InterpretError::RuntimePanic(
                "Cannot subtract types string and array".into()
            ))
        );
        assert_eq!(n.eval("2.3>=1").unwrap().unwrap(), "true");
        assert_eq!(n.eval("'a'~'b'").unwrap().unwrap(), "'ab'");
        assert_eq!(
            n.eval("'a'~1.2"),
            Err(InterpretError::RuntimePanic(
                "Cannot concat types string and float".into()
            ))
        );
        assert_eq!(n.eval("1==global").unwrap().unwrap(), "true");
        assert_eq!(n.eval("1==3").unwrap().unwrap(), "false");
        assert_eq!(n.eval("1==1.0").unwrap().unwrap(), "true");
        assert_eq!(n.eval("1==1.1").unwrap().unwrap(), "false");
        assert_eq!(n.eval("1.1==1.1").unwrap().unwrap(), "true");
        assert_eq!(n.eval("[]==[]").unwrap().unwrap(), "false");
        assert_eq!(n.eval("null==null").unwrap().unwrap(), "true");
        assert_eq!(n.eval("null==false").unwrap().unwrap(), "false");
        n.exec("global='a'").unwrap();
        assert_eq!(n.eval("'a'==global").unwrap().unwrap(), "true");
        assert_eq!(n.eval("0.0/0.0==0.0/0.0").unwrap().unwrap(), "false");
        assert_eq!(n.eval("-0.0==0.0").unwrap().unwrap(), "true");
        assert_eq!(n.eval("0.0/0.0===0.0/0.0").unwrap().unwrap(), "true");
        assert_eq!(n.eval("-0.0===0.0").unwrap().unwrap(), "false");
        assert_eq!(n.eval("1===1.0").unwrap().unwrap(), "false");
        n.exec(
            r#"global={};
        global[1]=2
        global[1.0]=3.0
        global[true]=true
        global['a']='a'
        global['a']='b'
        global[@a]=4
        global[@a]=6
        global[[]]=1
        global[global]='global'
        "#,
        )
        .unwrap();
        assert_eq!(n.eval("global[1]").unwrap().unwrap(), "2");
        assert_eq!(n.eval("global[1.0]").unwrap().unwrap(), "3.0");
        assert_eq!(n.eval("global[true]").unwrap().unwrap(), "true");
        assert_eq!(n.eval("global['a']").unwrap().unwrap(), "'b'");
        assert_eq!(n.eval("global[@a]").unwrap().unwrap(), "6");
        // todo:enable this test when u can properly print array
        //assert!(n.eval("global[[]]").is_err());
        assert_eq!(n.eval("global[global]").unwrap().unwrap(), "'global'");
        assert_eq!(
            n.eval("1[2]"),
            Err(InterpretError::RuntimePanic("Cannot index type int".into()))
        );
        assert_eq!(
            n.eval("'a'[2]"),
            Err(InterpretError::RuntimePanic(
                "Cannot index type string".into()
            ))
        );
        n.exec("global=[null]").unwrap();
        assert_eq!(n.eval("global[0]=5"), Ok(None));
        assert_eq!(n.eval("global[0]").unwrap().unwrap(), "5");
        assert_eq!(
            n.exec("8[0]=1"),
            Err(InterpretError::RuntimePanic("Cannot index type int".into()))
        );
        n.exec("if true{global=3}").unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "3");
        n.exec("if global==3{global=5}else{global=7}").unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "5");
        n.exec("if global==0{global=10}else if global==5{global=11}else{global=12}")
            .unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "11");
        n.exec("global=0\nfor i in 1 to 10{global+=i}").unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "45");
        n.exec("global=0\nfor i in 1 to 10{for j in 1 to 10{global+=1}}")
            .unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "81");
        n.exec("global=0\nfor i in 1 to 1{global+=1}").unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "0");
        n.exec("global=0\nfor i in 1 to -1{global+=1}").unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "0");
        n.exec(
            r#"
                global=0
                for i in 1 to 10{
                    if i==7{
                        break
                    }
                global+=i
                }
       "#,
        )
        .unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "21");
        n.exec(
            r#"
                global=0
                for i in 1 to 10{
                    if i==7{
                        continue
                    }
                global+=i
                }
       "#,
        )
        .unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "38");
        n.exec(
            r#"
                let i=0
                global=0
                while i<10{
                    i+=1
                    if i==7{
                        break
                    }
                global+=i
                }
       "#,
        )
        .unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "21");
        n.exec(
            r#"
                let i=0
                global=0
                while i<10{
                    i+=1
                    if i==7{
                        continue
                    }
                global+=i
                }
       "#,
        )
        .unwrap();
        assert_eq!(n.eval("global").unwrap().unwrap(), "48");
    }
}

use crate::parser::ClosureBody;
use crate::parser::Function;
use crate::parser::Literal;
use crate::parser::Statement;
use crate::parser::Substring;
use crate::vm::FunctionInfoWriter;
use crate::vm::ModuleVariable;
use crate::vm::Op;
use crate::vm::VM;
use crate::CompileError;
use crate::CompileResult;
use crate::{parser::Expr, scanner::TokenType};
use std::collections::HashMap;
use std::convert::TryFrom;

pub struct Compiler<'vm> {
    module_name: String,
    errors: Vec<CompileError>,
    vm: &'vm VM,
}

impl<'vm> Compiler<'vm> {
    pub fn new(vm: &'vm VM, module_name: String) -> Self {
        Self {
            vm,
            module_name,
            errors: vec![],
        }
    }

    pub fn exec(
        mut self,
        ast: Vec<Statement>,
    ) -> Result<FunctionInfoWriter<'vm>, Vec<CompileError>> {
        self.register_module_variables(&ast);
        let mut b = BytecodeCompiler::new(&mut self, "<main>", BytecodeType::Script, 0);
        b.compile_statments(&ast);
        b.bc_writer.write_u8(Op::Return.repr);
        let bytecode = b.bc_writer;
        if self.errors.is_empty() {
            Ok(bytecode)
        } else {
            Err(self.errors)
        }
    }

    pub fn can_eval(ast: &[Statement]) -> Option<&Expr> {
        match ast {
            [Statement::Expr(e)] => match e {
                Expr::Binary { op, .. } => match op {
                    TokenType::Equal
                    | TokenType::PlusEqual
                    | TokenType::MinusEqual
                    | TokenType::StarEqual
                    | TokenType::SlashEqual
                    | TokenType::TildeEqual
                    | TokenType::ModEqual => None,
                    _ => Some(e),
                },
                _ => Some(e),
            },
            _ => None,
        }
    }

    pub fn eval(mut self, ast: &Expr) -> Result<FunctionInfoWriter<'vm>, Vec<CompileError>> {
        let mut b = BytecodeCompiler::new(&mut self, "<main>", BytecodeType::Script, 0);
        match b.evaluate_expr(ast) {
            Ok(er) => {
                b.store_in_accumulator(er, 1);
            }
            Err(e) => b.error(e),
        }
        b.bc_writer.shrink();
        b.bc_writer.set_max_registers(b.max_registers);
        b.bc_writer.write_u8(Op::Return.repr);
        let bytecode = b.bc_writer;
        if self.errors.is_empty() {
            Ok(bytecode)
        } else {
            Err(self.errors)
        }
    }

    fn register_module_variable(&mut self, name: &str, mutable: bool, exported: bool, line: u32) {
        if name.starts_with('_') {
            self.errors.push(CompileError {
                message: format!("Exported variable {} cannot start with _", name),
                line,
            })
        }
        if !self.vm.add_module_variable(
            self.module_name.as_str().into(),
            name.into(),
            mutable,
            exported,
        ) {
            self.errors.push(CompileError {
                message: format!("Cannot redeclare module variable {}", name),
                line,
            })
        }
    }

    fn register_module_variables(&mut self, ast: &[Statement]) {
        for statement in ast {
            match statement {
                Statement::DestructuringVarDeclaration {
                    names,
                    mutable,
                    line,
                    exported,
                    ..
                } => {
                    for name in names {
                        self.register_module_variable(name, *mutable, *exported, *line)
                    }
                }
                Statement::VarDeclaration {
                    name,
                    mutable,
                    line,
                    exported,
                    ..
                } => self.register_module_variable(name, *mutable, *exported, *line),
                Statement::Function {
                    body: Function { name, line, .. },
                    exported,
                    ..
                } => self.register_module_variable(name, false, *exported, *line),
                Statement::Class {
                    name,
                    line,
                    exported,
                    ..
                } => self.register_module_variable(name, false, *exported, *line),

                _ => {}
            }
        }
    }
}

struct BytecodeCompiler<'c, 'vm> {
    compiler: Option<&'c mut Compiler<'vm>>,
    parent: Option<Box<BytecodeCompiler<'c, 'vm>>>,
    bc_writer: FunctionInfoWriter<'vm>,
    locals: Vec<HashMap<String, Local>>,
    regcount: u32,
    max_registers: u32,
    op_positions: Vec<usize>,
    loops: Vec<Loop>,
    bctype: BytecodeType,
    upvalues: Vec<UpValue>,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum BytecodeType {
    Script,
    Function,
    Method,
    Constructor,
}

enum Loop {
    While {
        start_reg: u32,
        loop_start: usize,
        breaks: Vec<usize>,
    },
    For {
        start_reg: u32,
        continues: Vec<usize>,
        breaks: Vec<usize>,
    },
}

#[derive(Clone)]
struct Local {
    reg: u32,
    mutable: bool,
    is_captured: bool,
}

struct UpValue {
    index: u32,
    is_local: bool,
    mutable: bool,
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn new(c: &'c mut Compiler<'vm>, name: &str, bctype: BytecodeType, arity: u8) -> Self {
        Self {
            bc_writer: c
                .vm
                .new_function_info(c.module_name.as_str().into(), name.into(), arity),
            locals: if bctype == BytecodeType::Script {
                vec![]
            } else {
                vec![HashMap::default()]
            },
            regcount: 0,
            compiler: Some(c),
            max_registers: 0,
            op_positions: vec![],
            loops: vec![],
            bctype,
            parent: None,
            upvalues: vec![],
        }
    }

    fn push_register(&mut self) -> u32 {
        self.regcount += 1;
        if self.regcount > self.max_registers {
            self.max_registers = self.regcount;
        }
        self.regcount - 1
    }

    fn pop_register(&mut self) {
        self.regcount -= 1;
    }

    fn get_global(&self, name: &str) -> Option<ModuleVariable> {
        (self.compiler.as_ref().unwrap() as &Compiler)
            .vm
            .get_module_variable(
                (self.compiler.as_ref().unwrap() as &Compiler)
                    .module_name
                    .as_str()
                    .into(),
                name.into(),
            )
            .ok()
    }

    fn error(&mut self, e: CompileError) {
        self.compiler.as_mut().unwrap().errors.push(e)
    }

    fn write0(&mut self, op: Op, line: u32) {
        let pos = self.bc_writer.write_op(op, line);
        self.op_positions.push(pos);
    }

    fn write1(&mut self, op: Op, u: u32, line: u32) {
        if let Ok(u) = u8::try_from(u) {
            self.write0(op, line);
            self.bc_writer.write_u8(u);
        } else if let Ok(u) = u16::try_from(u) {
            self.write0(Op::Wide, line);
            self.bc_writer.write_u8(op.repr);
            self.bc_writer.write_u16(u);
        } else {
            self.write0(Op::ExtraWide, line);
            self.bc_writer.write_u8(op.repr);
            self.bc_writer.write_u32(u);
        }
    }

    fn write1_signed(&mut self, op: Op, i: i32, line: u32) {
        if let Ok(i) = i8::try_from(i) {
            self.write0(op, line);
            self.bc_writer.write_u8(i as u8);
        } else if let Ok(i) = i16::try_from(i) {
            self.write0(Op::Wide, line);
            self.bc_writer.write_u8(op.repr);
            self.bc_writer.write_u16(i as u16);
        } else {
            self.write0(Op::ExtraWide, line);
            self.bc_writer.write_u8(op.repr);
            self.bc_writer.write_u32(i as u32);
        }
    }

    fn write2(&mut self, op: Op, u1: u32, u2: u32, line: u32) {
        match (u8::try_from(u1), u8::try_from(u2)) {
            (Ok(u1), Ok(u2)) => {
                self.write0(op, line);
                self.bc_writer.write_u8(u1);
                self.bc_writer.write_u8(u2)
            }
            _ => match (u16::try_from(u1), u16::try_from(u2)) {
                (Ok(u1), Ok(u2)) => {
                    self.write0(Op::Wide, line);
                    self.bc_writer.write_u8(op.repr);
                    self.bc_writer.write_u16(u1);
                    self.bc_writer.write_u16(u2)
                }
                _ => {
                    self.write0(Op::ExtraWide, line);
                    self.bc_writer.write_u8(op.repr);
                    self.bc_writer.write_u32(u1);
                    self.bc_writer.write_u32(u2)
                }
            },
        }
    }

    fn write3(&mut self, op: Op, u1: u32, u2: u32, u3: u32, line: u32) {
        match (u8::try_from(u1), u8::try_from(u2), u8::try_from(u3)) {
            (Ok(u1), Ok(u2), Ok(u3)) => {
                self.write0(op, line);
                self.bc_writer.write_u8(u1);
                self.bc_writer.write_u8(u2);
                self.bc_writer.write_u8(u3)
            }
            _ => match (u16::try_from(u1), u16::try_from(u2), u16::try_from(u3)) {
                (Ok(u1), Ok(u2), Ok(u3)) => {
                    self.write0(Op::Wide, line);
                    self.bc_writer.write_u8(op.repr);
                    self.bc_writer.write_u16(u1);
                    self.bc_writer.write_u16(u2);
                    self.bc_writer.write_u16(u3)
                }
                _ => {
                    self.write0(Op::ExtraWide, line);
                    self.bc_writer.write_u8(op.repr);
                    self.bc_writer.write_u32(u1);
                    self.bc_writer.write_u32(u2);
                    self.bc_writer.write_u32(u3)
                }
            },
        }
    }

    fn pop_last_op(&mut self) {
        let pos = self.op_positions.pop().unwrap();
        self.bc_writer.pop_last_op(pos);
    }
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn write_op_store_register(&mut self, reg: u32, line: u32) {
        match reg {
            0..=15 => {
                self.write0(
                    Op {
                        repr: match reg {
                            0 => Op::StoreR0.repr,
                            1 => Op::StoreR1.repr,
                            2 => Op::StoreR2.repr,
                            3 => Op::StoreR3.repr,
                            4 => Op::StoreR4.repr,
                            5 => Op::StoreR5.repr,
                            6 => Op::StoreR6.repr,
                            7 => Op::StoreR7.repr,
                            8 => Op::StoreR8.repr,
                            9 => Op::StoreR9.repr,
                            10 => Op::StoreR10.repr,
                            11 => Op::StoreR11.repr,
                            12 => Op::StoreR12.repr,
                            13 => Op::StoreR13.repr,
                            14 => Op::StoreR14.repr,
                            15 => Op::StoreR15.repr,
                            _ => unreachable!(),
                        },
                    },
                    line,
                );
            }
            _ => {
                self.write1(Op::StoreRegister, reg, line);
            }
        }
    }

    fn write_op_load_register(&mut self, reg: u32, line: u32) {
        match reg {
            0..=15 => {
                self.write0(
                    Op {
                        repr: match reg {
                            0 => Op::LoadR0.repr,
                            1 => Op::LoadR1.repr,
                            2 => Op::LoadR2.repr,
                            3 => Op::LoadR3.repr,
                            4 => Op::LoadR4.repr,
                            5 => Op::LoadR5.repr,
                            6 => Op::LoadR6.repr,
                            7 => Op::LoadR7.repr,
                            8 => Op::LoadR8.repr,
                            9 => Op::LoadR9.repr,
                            10 => Op::LoadR10.repr,
                            11 => Op::LoadR11.repr,
                            12 => Op::LoadR12.repr,
                            13 => Op::LoadR13.repr,
                            14 => Op::LoadR14.repr,
                            15 => Op::LoadR15.repr,
                            _ => unreachable!(),
                        },
                    },
                    line,
                );
            }
            _ => {
                self.write1(Op::LoadRegister, reg, line);
            }
        }
    }

    fn load_int(&mut self, i: i32, line: u32) {
        match i8::try_from(i) {
            Ok(i) => self.write1(Op::LoadSmallInt, i as u8 as u32, line),
            _ => {
                let c = self.bc_writer.int_constant(i);
                self.write1(Op::LoadConstant, c, line);
            }
        }
    }

    fn new_local(&mut self, name: String, mutable: bool) -> u32 {
        let reg = self.push_register();
        self.locals.last_mut().unwrap().insert(
            name,
            Local {
                mutable,
                reg,
                is_captured: false,
            },
        );
        reg
    }
}

#[derive(Debug, Clone, Copy)]
enum ExprResult {
    Register(u32),
    Accumulator,
    Int(i32),
}

macro_rules! binary_op {
    ($op:ident,$register_inst:ident,$int_inst:ident,$op_fn:ident,$op_checked_fn:ident,$op_name:tt) => {
        fn $op(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
            let left = self.evaluate_expr(left)?;
            let mut reg = 0;
            if !matches!(left, ExprResult::Register(_)) {
                reg = self.push_register();
                self.store_in_accumulator(left, line);
                self.write_op_store_register(reg, line);
            }
            let right = self.evaluate_expr(right)?;
            if !matches!(left, ExprResult::Register(_)) {
                self.pop_register();
            }
            match (left, right) {
                (ExprResult::Int(i1), ExprResult::Int(i2)) => {
                    self.undo_save_to_register(ExprResult::Int(i1));
                    Ok(ExprResult::Int(i1.$op_checked_fn(i2).ok_or(
                        CompileError {
                            message: format!(
                                "Cannot {} {} and {} as the result cannot be stored in an Int",
                                $op_name, i1, i2
                            ),
                            line,
                        },
                    )?))
                }
                (left, ExprResult::Int(i)) => {
                    self.undo_save_to_register(left);
                    self.store_in_accumulator(left, line);
                    self.write1_signed(Op::$int_inst, i, line);
                    Ok(ExprResult::Accumulator)
                }
                (ExprResult::Register(r), right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$register_inst, r as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (_, right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$register_inst, reg as u32, line);
                    Ok(ExprResult::Accumulator)
                }
            }
        }
    };
}

macro_rules! comparing_binary_op {
    ($op:ident,$inst:ident,$op_symbol:tt) => {
        fn $op(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
            let left = self.evaluate_expr(left)?;
            let mut reg = 0;
            if !matches!(left, ExprResult::Register(_)) {
                reg = self.push_register();
                self.store_in_accumulator(left, line);
                self.write_op_store_register(reg, line);
            }
            let right = self.evaluate_expr(right)?;
            if !matches!(left, ExprResult::Register(_)) {
                self.pop_register();
            }
            match (left, right) {
                (ExprResult::Int(i1), ExprResult::Int(i2)) => {
                    self.undo_save_to_register(ExprResult::Int(i1));
                    self.write0(if i1 $op_symbol i2{Op::LoadTrue} else {Op::LoadFalse},line);
                    Ok(ExprResult::Accumulator)
                }
                (ExprResult::Register(r), right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$inst, r as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (_, right) => {
                    self.store_in_accumulator(right, line);
                    self.write1(Op::$inst, reg as u32, line);
                    Ok(ExprResult::Accumulator)
                }
            }
        }
    };
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn undo_save_to_register(&mut self, result: ExprResult) {
        match result {
            ExprResult::Register(_) => {}
            ExprResult::Accumulator => {
                self.pop_last_op();
            }
            ExprResult::Int(_) => {
                self.pop_last_op();
                self.pop_last_op();
            }
        }
    }

    fn store_in_accumulator(&mut self, result: ExprResult, line: u32) {
        match result {
            ExprResult::Register(reg) => {
                self.write_op_load_register(reg, line);
            }
            ExprResult::Accumulator => {}
            ExprResult::Int(i) => {
                self.load_int(i, line);
            }
        }
    }

    fn store_in_register(&mut self, result: ExprResult, line: u32) -> u32 {
        if let ExprResult::Register(r) = result {
            r
        } else {
            self.store_in_accumulator(result, line);
            let reg = self.push_register();
            self.write_op_store_register(reg, line);
            reg
        }
    }

    fn store_in_specific_register(&mut self, result: ExprResult, reg: u32, line: u32) {
        match result {
            ExprResult::Register(r) => {
                if r != reg {
                    self.write2(Op::Move, r, reg, line)
                }
            }
            ExprResult::Accumulator => self.write_op_store_register(reg, line),
            ExprResult::Int(i) => {
                self.load_int(i, line);
                self.write_op_store_register(reg, line)
            }
        }
    }

    fn resolve_local(&mut self, name: &str) -> Option<&mut Local> {
        for locals in self.locals.iter_mut().rev() {
            if let Some(local) = locals.get_mut(name) {
                return Some(local);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, name: &str, line: u32) -> Option<u32> {
        match self.parent.as_mut() {
            Some(parent) => match parent.resolve_local(name) {
                Some(l) => {
                    l.is_captured = true;
                    let l = l.clone();
                    let u = self.add_upvalue(l.reg, true, l.mutable);
                    Some(u)
                }
                None => match parent.resolve_upvalue(name, line) {
                    Some(u) => {
                        let mutable = parent.upvalues[u as usize].mutable;
                        Some(self.add_upvalue(u, false, mutable))
                    }
                    None => None,
                },
            },
            None => None,
        }
    }

    fn add_upvalue(&mut self, index: u32, is_local: bool, mutable: bool) -> u32 {
        for (i, upvalue) in self.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i as u32;
            }
        }
        self.upvalues.push(UpValue {
            index,
            is_local,
            mutable,
        });
        self.bc_writer.add_upvalue(index, is_local);
        (self.upvalues.len() - 1) as u32
    }

    fn compile_statments(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.compile_statement(statement)
        }
        self.bc_writer.shrink();
        self.bc_writer.set_max_registers(self.max_registers);
    }

    fn var_declaration(
        &mut self,
        name: &str,
        expr: &Expr,
        line: u32,
        mutable: bool,
    ) -> CompileResult<()> {
        if self.bctype == BytecodeType::Script && self.locals.is_empty() {
            let g = self.get_global(name).unwrap();
            let res = self.evaluate_expr(expr)?;
            self.store_in_accumulator(res, line);
            self.write1(Op::StoreModuleVariable, g.position, line);
        } else {
            if self.locals.last().unwrap().contains_key(name) {
                return Err(CompileError {
                    message: format!("Cannot redeclare variable {} in the same scope", name),
                    line,
                });
            }
            let reg = self.push_register();
            let res = self.evaluate_expr_with_dest(expr, Some(reg))?;
            self.pop_register();
            let reg = self.new_local(name.into(), mutable);
            self.store_in_specific_register(res, reg, line);
        }
        Ok(())
    }

    fn create_variable_and_store_accumulator(
        &mut self,
        name: &str,
        mutable: bool,
        line: u32,
    ) -> CompileResult<()> {
        if self.bctype == BytecodeType::Script && self.locals.is_empty() {
            let g = self.get_global(name).unwrap();
            self.write1(Op::StoreModuleVariable, g.position, line);
        } else {
            if self.locals.last().unwrap().contains_key(name) {
                return Err(CompileError {
                    message: format!("Cannot redeclare variable {} in the same scope", name),
                    line,
                });
            }
            let reg = self.new_local(name.into(), mutable);
            self.write_op_store_register(reg, line);
        }
        Ok(())
    }

    fn compile_statement(&mut self, statement: &Statement) {
        if let Err(e) = (|| -> CompileResult<()> {
            match statement {
                Statement::Expr(expr) => match expr {
                    Expr::Binary {
                        left,
                        op,
                        right,
                        line,
                    } => {
                        if *op == TokenType::Equal {
                            self.equal(left, right, *line)?;
                        } else if *op == TokenType::PlusEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Plus,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::MinusEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Minus,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::StarEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Star,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::SlashEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Slash,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::ModEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Mod,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else if *op == TokenType::TildeEqual {
                            self.equal(
                                left,
                                &Expr::Binary {
                                    left: left.clone(),
                                    op: TokenType::Tilde,
                                    right: right.clone(),
                                    line: *line,
                                },
                                *line,
                            )?;
                        } else {
                            self.evaluate_expr(expr)?;
                        }
                    }
                    _ => {
                        self.evaluate_expr(expr)?;
                    }
                },
                Statement::DestructuringVarDeclaration {
                    names,
                    expr,
                    mutable,
                    exported,
                    line,
                } => {
                    if *exported && (self.bctype != BytecodeType::Script || !self.locals.is_empty())
                    {
                        self.error(CompileError {
                            message: "Cannot export non module variable".to_string(),
                            line: *line,
                        });
                    }
                    let object_res = self.evaluate_expr(expr)?;
                    let reg = if !(matches!(object_res, ExprResult::Register(_))
                        || (self.bctype == BytecodeType::Script && self.locals.is_empty()))
                    {
                        let target = names.len() + (self.regcount as usize);
                        if (target as u32) > self.max_registers {
                            self.max_registers = target as u32;
                        }
                        self.store_in_specific_register(object_res, target as u32, *line);
                        target as u32
                    } else {
                        self.store_in_register(object_res, *line)
                    };
                    for name in names {
                        let property = self.bc_writer.symbol_constant(name.as_str().into());
                        self.write2(Op::LoadProperty, reg, property, *line);
                        self.create_variable_and_store_accumulator(name, *mutable, *line)?;
                    }
                    if !matches!(object_res, ExprResult::Register(_))
                        && (self.bctype == BytecodeType::Script && self.locals.is_empty())
                    {
                        self.pop_register();
                    }
                }
                Statement::VarDeclaration {
                    name,
                    expr,
                    mutable,
                    exported,
                    line,
                } => {
                    if *exported && (self.bctype != BytecodeType::Script || !self.locals.is_empty())
                    {
                        self.error(CompileError {
                            message: "Cannot export non module variable".to_string(),
                            line: *line,
                        });
                    }
                    self.var_declaration(name, expr, *line, *mutable)?;
                }
                Statement::Block { block, end_line } => {
                    self.block(block, *end_line);
                }
                Statement::If {
                    condition,
                    block,
                    else_stmt,
                    if_end,
                } => {
                    let res = self.evaluate_expr(condition);
                    let line = condition.line();
                    match res {
                        Err(e) => self.error(e),
                        Ok(res) => {
                            self.store_in_accumulator(res, line);
                        }
                    }
                    let c = self.bc_writer.reserve_constant();
                    let cond_check = self.bc_writer.size();
                    self.write1(Op::JumpIfFalseOrNullConstant, c, line);
                    self.block(block, *if_end);
                    let if_end_pos = self.bc_writer.size();
                    if let Some(else_stmt) = else_stmt {
                        let c = self.bc_writer.reserve_constant();
                        self.write1(Op::JumpConstant, c, *if_end);
                        let jump_end = self.bc_writer.size();
                        self.bc_writer
                            .patch_jump(cond_check, (jump_end - cond_check) as u32);
                        self.compile_statement(else_stmt);
                        let else_end = self.bc_writer.size();
                        self.bc_writer
                            .patch_jump(if_end_pos, (else_end - if_end_pos) as u32);
                    } else {
                        self.bc_writer
                            .patch_jump(cond_check, (if_end_pos - cond_check) as u32);
                    }
                }
                Statement::While {
                    condition,
                    block,
                    end_line,
                } => {
                    let loop_start = self.bc_writer.size();
                    self.loops.push(Loop::While {
                        start_reg: self.regcount,
                        loop_start,
                        breaks: vec![],
                    });
                    match self.evaluate_expr(condition) {
                        Err(e) => self.error(e),
                        Ok(res) => {
                            self.store_in_accumulator(res, condition.line());
                        }
                    }

                    let c = self.bc_writer.reserve_constant();
                    let loop_cond_check = self.bc_writer.size();
                    self.write1(Op::JumpIfFalseOrNullConstant, c as u32, condition.line());
                    self.block(block, *end_line);
                    let almost_loop_end = self.bc_writer.size();
                    self.write1(
                        Op::JumpBack,
                        (almost_loop_end - loop_start) as u32,
                        *end_line,
                    );
                    let loop_end = self.bc_writer.size();
                    self.bc_writer
                        .patch_jump(loop_cond_check, (loop_end - loop_cond_check) as u32);
                    match self.loops.last().unwrap() {
                        Loop::While { breaks, .. } => {
                            for b in breaks.iter() {
                                self.bc_writer.patch_jump(*b, (loop_end - b) as u32);
                            }
                        }
                        _ => unreachable!(),
                    }
                    self.loops.pop();
                }
                Statement::For {
                    iter,
                    expr,
                    block,
                    end_line,
                } => {
                    if let Expr::Binary {
                        left: start,
                        right: end,
                        op: TokenType::DotDot,
                        ..
                    } = expr
                    {
                        let start = self.evaluate_expr(start);
                        if let Err(ref e) = start {
                            self.error(e.clone());
                        }
                        self.locals.push(HashMap::default());
                        let iter_reg = self.new_local(iter.clone(), false);
                        if let Ok(start) = start {
                            self.store_in_specific_register(start, iter_reg, expr.line());
                        }
                        let end = self.evaluate_expr(end);
                        if let Err(ref e) = end {
                            self.error(e.clone());
                        }
                        let end_reg = self.new_local("$end".into(), false);
                        if let Ok(end) = end {
                            self.store_in_specific_register(end, end_reg, expr.line());
                        }
                        let c = self.bc_writer.reserve_constant();
                        let before_loop_prep = self.bc_writer.size();
                        self.write2(Op::BeginForLoopConstant, c as u32, iter_reg, expr.line());
                        let loop_start = self.bc_writer.size();
                        self.loops.push(Loop::For {
                            start_reg: iter_reg,
                            breaks: vec![],
                            continues: vec![],
                        });
                        for stmt in block {
                            self.compile_statement(stmt);
                        }
                        let last_block = self.locals.last().unwrap();
                        if last_block.values().any(|l| l.is_captured) {
                            self.write1(Op::Close, (iter_reg) as u32, *end_line);
                        }
                        let loop_almost_end = self.bc_writer.size();
                        self.write2(
                            Op::ForLoop,
                            (loop_almost_end - loop_start) as u32,
                            iter_reg,
                            *end_line,
                        );
                        let loop_end = self.bc_writer.size();
                        self.bc_writer
                            .patch_jump(before_loop_prep, (loop_end - before_loop_prep) as u32);
                        match self.loops.last().unwrap() {
                            Loop::For {
                                continues, breaks, ..
                            } => {
                                for b in breaks.iter() {
                                    self.bc_writer.patch_jump(*b, (loop_end - b) as u32);
                                }
                                for c in continues.iter() {
                                    self.bc_writer.patch_jump(*c, (loop_almost_end - c) as u32);
                                }
                            }
                            _ => unreachable!(),
                        }
                        self.loops.pop();
                        let last_block = self.locals.pop().unwrap();
                        self.regcount -= last_block.len() as u32;
                    } else {
                        let res = self.evaluate_expr(expr);
                        if let Ok(res) = res {
                            self.store_in_accumulator(res, expr.line());
                        } else if let Err(e) = res {
                            self.error(e);
                        }
                        self.locals.push(HashMap::default());
                        let iterator = self.new_local("$iter".into(), false);
                        self.store_in_specific_register(
                            ExprResult::Accumulator,
                            iterator,
                            expr.line(),
                        );

                        let loop_start = self.bc_writer.size();
                        let hasnext_property = self.bc_writer.symbol_constant("hasNext".into());
                        let start = self.regcount;
                        self.push_register();
                        self.write3(
                            Op::CallMethod,
                            iterator,
                            hasnext_property,
                            start,
                            expr.line(),
                        );
                        self.bc_writer.write_u8(0);
                        self.pop_register();

                        let c = self.bc_writer.reserve_constant();
                        let loop_cond_check = self.bc_writer.size();
                        self.write1(Op::JumpIfFalseOrNullConstant, c as u32, expr.line());

                        let iter_reg = self.new_local(iter.into(), false);
                        let next_property = self.bc_writer.symbol_constant("next".into());
                        let start = self.regcount;
                        self.push_register();
                        self.write3(Op::CallMethod, iterator, next_property, start, expr.line());
                        self.bc_writer.write_u8(0);
                        self.pop_register();
                        self.store_in_specific_register(
                            ExprResult::Accumulator,
                            iter_reg,
                            expr.line(),
                        );
                        self.loops.push(Loop::While {
                            start_reg: iter_reg,
                            loop_start,
                            breaks: vec![],
                        });
                        for stmt in block {
                            self.compile_statement(stmt);
                        }
                        let last_block = self.locals.last().unwrap();
                        if last_block.values().any(|l| l.is_captured) {
                            self.write1(Op::Close, (iter_reg) as u32, *end_line);
                        }
                        let almost_loop_end = self.bc_writer.size();
                        self.write1(
                            Op::JumpBack,
                            (almost_loop_end - loop_start) as u32,
                            *end_line,
                        );
                        let loop_end = self.bc_writer.size();
                        self.bc_writer
                            .patch_jump(loop_cond_check, (loop_end - loop_cond_check) as u32);
                        match self.loops.last().unwrap() {
                            Loop::While { breaks, .. } => {
                                for b in breaks.iter() {
                                    self.bc_writer.patch_jump(*b, (loop_end - b) as u32);
                                }
                            }
                            _ => unreachable!(),
                        }
                        self.loops.pop();
                        let last_block = self.locals.pop().unwrap();
                        self.regcount -= last_block.len() as u32;
                    }
                }
                Statement::Break { line } => self.break_stmt(*line)?,
                Statement::Continue { line } => self.continue_stmt(*line)?,
                Statement::Function {
                    body:
                        Function {
                            name,
                            line,
                            arguments,
                            body,
                            last_line,
                            ..
                        },
                    exported,
                    ..
                } => {
                    if *exported && (self.bctype != BytecodeType::Script || !self.locals.is_empty())
                    {
                        self.error(CompileError {
                            message: "Cannot export non module variable".to_string(),
                            line: *line,
                        });
                    }
                    let bytecode = self.closure(
                        name,
                        *line,
                        arguments,
                        &ClosureBody::Block(body.clone()),
                        BytecodeType::Function,
                        *last_line,
                    )?;
                    let c = self.bc_writer.fun_constant(bytecode);
                    self.write1(Op::MakeFunction, c, *last_line);
                    self.create_variable_and_store_accumulator(name, false, *line)?;
                }
                Statement::Return { line, expr } => {
                    if self.bctype == BytecodeType::Script {
                        return Err(CompileError {
                            message: "Cannot use return outside a function or method".into(),
                            line: *line,
                        });
                    }
                    if let Some(expr) = expr {
                        if self.bctype == BytecodeType::Constructor {
                            return Err(CompileError {
                                message: "Cannot return expression from a constructor".to_string(),
                                line: *line,
                            });
                        }
                        let expr_res = self.evaluate_expr(expr)?;
                        self.store_in_accumulator(expr_res, *line);
                    } else if self.bctype == BytecodeType::Constructor {
                        self.write0(Op::LoadR0, *line);
                    } else {
                        self.write0(Op::LoadNull, *line);
                    }
                    self.write0(Op::Return, *line);
                }
                Statement::Throw(e) => {
                    let expr_res = self.evaluate_expr(e)?;
                    self.store_in_accumulator(expr_res, e.line());
                    self.write0(Op::Throw, e.line());
                }
                Statement::TryCatch {
                    try_block,
                    try_end,
                    error_var,
                    catch_block,
                    catch_end,
                } => {
                    let try_start_pos = self.bc_writer.size();
                    self.block(try_block, *try_end);
                    let try_end_pos = self.bc_writer.size();
                    let c = self.bc_writer.reserve_constant();
                    let jump_pos = self.bc_writer.size();
                    self.write1(Op::JumpConstant, c, *try_end);
                    self.locals.push(HashMap::default());
                    let error_reg = self.push_register();
                    self.locals.last_mut().unwrap().insert(
                        error_var.clone(),
                        Local {
                            mutable: false,
                            reg: error_reg,
                            is_captured: false,
                        },
                    );
                    let catch_start_pos = self.bc_writer.size();
                    for stmt in catch_block {
                        self.compile_statement(stmt);
                    }
                    let last_block = self.locals.pop().unwrap();
                    self.regcount -= last_block.len() as u32;
                    if last_block.values().any(|l| l.is_captured) {
                        self.write1(Op::Close, self.regcount as u32, *catch_end);
                    }
                    let catch_end_pos = self.bc_writer.size();
                    self.bc_writer
                        .patch_jump(jump_pos, (catch_end_pos - jump_pos) as u32);
                    self.bc_writer.add_exception_handler(
                        try_start_pos as u32,
                        try_end_pos as u32,
                        error_reg,
                        catch_start_pos as u32,
                    );
                }
                Statement::Class {
                    line,
                    name,
                    parent,
                    methods,
                    exported,
                } => {
                    if *exported && (self.bctype != BytecodeType::Script || !self.locals.is_empty())
                    {
                        self.error(CompileError {
                            message: "Cannot export non module variable".to_string(),
                            line: *line,
                        });
                    }
                    let class = self.bc_writer.class_constant(name.as_str().into());
                    if let Some(parent) = parent {
                        let res = self.evaluate_expr(parent);
                        if let Ok(res) = res {
                            self.store_in_accumulator(res, *line);
                        } else if let Err(e) = res {
                            self.error(e);
                        }
                    } else {
                        self.write1(
                            Op::LoadModuleVariable,
                            self.get_global("Object").unwrap().position,
                            *line,
                        );
                    }
                    for method in methods {
                        let bytecode = self.closure(
                            &method.name,
                            method.line,
                            &method.arguments,
                            &ClosureBody::Block(method.body.clone()),
                            if method.name == "construct" {
                                BytecodeType::Constructor
                            } else {
                                BytecodeType::Method
                            },
                            method.last_line,
                        );

                        match bytecode {
                            Ok(bytecode) => self.bc_writer.add_method(
                                class,
                                method.name.as_str().into(),
                                bytecode,
                            ),
                            Err(e) => self.error(e),
                        }
                    }
                    self.write1(Op::MakeClass, class as u32, *line);
                    self.create_variable_and_store_accumulator(name, false, *line)?;
                }
                Statement::Switch { line, expr, cases } => {
                    let jump_table = self.bc_writer.jump_table();
                    match self.evaluate_expr(expr) {
                        Ok(res) => self.store_in_accumulator(res, *line),
                        Err(e) => self.error(e),
                    }
                    self.write1(Op::Switch, jump_table, *line);
                    let switch_start = self.bc_writer.size();
                    let c = self.bc_writer.reserve_constant();
                    self.write1(Op::JumpConstant, c, *line);
                    let mut case_positions = vec![];
                    let mut default_statement = None;
                    for case in cases.iter() {
                        let case_start = self.bc_writer.size();
                        self.compile_statement(&case.1);
                        let c = self.bc_writer.reserve_constant();
                        case_positions.push(self.bc_writer.size());
                        self.write1(Op::JumpConstant, c, case.2);
                        if case.0.contains(&Literal::Default) {
                            default_statement = Some(case_start);
                        } else {
                            for literal in &case.0 {
                                self.bc_writer.reuse_constants = false;
                                match literal {
                                    Literal::Int(i) => self.bc_writer.int_constant(*i),
                                    Literal::Float(f) => self.bc_writer.float_constant(*f),
                                    Literal::String(s) => {
                                        self.bc_writer.string_constant(s.as_str().into())
                                    }
                                    Literal::Symbol(sym) => {
                                        self.bc_writer.symbol_constant(sym.as_str().into())
                                    }
                                    Literal::Null => self.bc_writer.null_constant(),
                                    Literal::True => self.bc_writer.bool_constant(true),
                                    Literal::False => self.bc_writer.bool_constant(false),
                                    Literal::Default => unreachable!(),
                                };
                                self.bc_writer.reuse_constants = true;
                                if !self.bc_writer.insert_in_jump_table(
                                    jump_table,
                                    (case_start - switch_start) as u32,
                                ) {
                                    self.error(CompileError {
                                        message: "Cannot repeat cases in switch statement".into(),
                                        line: case.2,
                                    });
                                }
                            }
                        }
                    }
                    let end_pos = self.bc_writer.size();
                    if let Some(pos) = default_statement {
                        self.bc_writer
                            .patch_jump(switch_start, (pos - switch_start) as u32);
                    } else {
                        self.bc_writer
                            .patch_jump(switch_start, (end_pos - switch_start) as u32);
                    }
                    for pos in case_positions {
                        self.bc_writer.patch_jump(pos, (end_pos - pos) as u32);
                    }
                }
            };
            Ok(())
        })() {
            self.error(e)
        }
    }

    fn block(&mut self, stmts: &[Statement], end_line: u32) {
        self.locals.push(HashMap::default());
        for stmt in stmts {
            self.compile_statement(stmt);
        }
        let last_block = self.locals.pop().unwrap();
        self.regcount -= last_block.len() as u32;
        if last_block.values().any(|l| l.is_captured) {
            self.write1(Op::Close, self.regcount as u32, end_line);
        }
    }

    fn break_stmt(&mut self, line: u32) -> CompileResult<()> {
        if self.loops.is_empty() {
            return Err(CompileError {
                message: "Cannot use break outside a loop".into(),
                line,
            });
        } else {
            let break_pos = self.bc_writer.size();
            let start;
            match self.loops.last_mut().unwrap() {
                Loop::While {
                    breaks, start_reg, ..
                } => {
                    start = *start_reg;
                    if self.locals.last().unwrap().values().any(|l| l.is_captured) {
                        if u8::try_from(start).is_ok() {
                            breaks.push(break_pos + 2)
                        } else {
                            breaks.push(break_pos + 3)
                        }
                    } else {
                        breaks.push(break_pos)
                    }
                }
                Loop::For {
                    breaks, start_reg, ..
                } => {
                    start = *start_reg;
                    if self.locals.last().unwrap().values().any(|l| l.is_captured) {
                        if u8::try_from(start).is_ok() {
                            breaks.push(break_pos + 2)
                        } else {
                            breaks.push(break_pos + 3)
                        }
                    } else {
                        breaks.push(break_pos)
                    }
                }
            }
            if self.locals.last().unwrap().values().any(|l| l.is_captured) {
                self.write1(Op::Close, start as u32, line);
            }
            let c = self.bc_writer.reserve_constant();
            self.write1(Op::JumpConstant, c, line);
        }
        Ok(())
    }

    fn continue_stmt(&mut self, line: u32) -> CompileResult<()> {
        if self.loops.is_empty() {
            return Err(CompileError {
                message: "Cannot use continue outside a loop".into(),
                line,
            });
        } else {
            match self.loops.last_mut().unwrap() {
                Loop::While {
                    loop_start,
                    start_reg,
                    ..
                } => {
                    let start_reg = *start_reg;
                    let loop_start = *loop_start;
                    if self.locals.last().unwrap().values().any(|l| l.is_captured) {
                        self.write1(Op::Close, start_reg as u32, line);
                    }
                    let continue_pos = self.bc_writer.size();
                    self.write1(Op::JumpBack, (continue_pos - loop_start) as u32, line);
                }
                Loop::For {
                    continues,
                    start_reg,
                    ..
                } => {
                    let start_reg = *start_reg;
                    let continue_pos = self.bc_writer.size();
                    if self.locals.last().unwrap().values().any(|l| l.is_captured) {
                        if u8::try_from(start_reg).is_ok() {
                            continues.push(continue_pos + 2);
                        } else {
                            continues.push(continue_pos + 3);
                        }
                        self.write1(Op::Close, start_reg as u32, line);
                    } else {
                        continues.push(continue_pos);
                    }
                    let c = self.bc_writer.reserve_constant();
                    self.write1(Op::JumpConstant, c, line);
                }
            }
        }
        Ok(())
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> CompileResult<ExprResult> {
        self.evaluate_expr_with_dest(expr, None)
    }

    fn evaluate_expr_with_dest(
        &mut self,
        expr: &Expr,
        dest: Option<u32>,
    ) -> CompileResult<ExprResult> {
        match expr {
            Expr::Literal { inner, line } => match inner {
                TokenType::IntLiteral(i) => {
                    if *i != -1 {
                        Ok(ExprResult::Int(*i))
                    } else {
                        Err(CompileError {
                            message: "Cannot parse integer 2147483648".to_string(),
                            line: *line,
                        })
                    }
                }
                TokenType::FloatLiteral(f) => Ok({
                    let c = self.bc_writer.float_constant(*f);
                    self.write1(Op::LoadConstant, c, *line);
                    ExprResult::Accumulator
                }),
                TokenType::Null => {
                    self.write0(Op::LoadNull, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::True => {
                    self.write0(Op::LoadTrue, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::False => {
                    self.write0(Op::LoadFalse, *line);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::Symbol(sym) => {
                    let sym = self.bc_writer.symbol_constant(sym.as_str().into());
                    self.write1(Op::LoadConstant, sym, *line);
                    Ok(ExprResult::Accumulator)
                }
                _ => unreachable!(),
            },
            Expr::Binary {
                left,
                op,
                right,
                line,
            } => match op {
                TokenType::Plus => self.add(left, right, *line),
                TokenType::Minus => self.subtract(left, right, *line),
                TokenType::Star => self.multiply(left, right, *line),
                TokenType::Slash => self.divide(left, right, *line),
                TokenType::Mod => self.modulus(left, right, *line),
                TokenType::EqualEqual => self.equal_equal(left, right, *line),
                TokenType::EqualEqualEqual => self.equal_equal_equal(left, right, *line),
                TokenType::BangEqualEqual => self.not_equal_equal(left, right, *line),
                TokenType::BangEqual => self.not_equal(left, right, *line),
                TokenType::Greater => self.greater_than(left, right, *line),
                TokenType::Less => self.lesser_than(left, right, *line),
                TokenType::GreaterEqual => self.greater_than_or_equal(left, right, *line),
                TokenType::LessEqual => self.lesser_than_or_equal(left, right, *line),
                TokenType::Equal => Err(CompileError {
                    message: "= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::PlusEqual => Err(CompileError {
                    message: "+= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::MinusEqual => Err(CompileError {
                    message: "-= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::StarEqual => Err(CompileError {
                    message: "*= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::SlashEqual => Err(CompileError {
                    message: "/= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::ModEqual => Err(CompileError {
                    message: "%= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::Tilde => self.concat(left, right, *line),
                TokenType::TildeEqual => Err(CompileError {
                    message: "~= is not an expression".to_string(),
                    line: *line,
                }),
                TokenType::And => {
                    let left = self.evaluate_expr(left)?;
                    self.store_in_accumulator(left, *line);
                    let jump_pos = self.bc_writer.size();
                    let c = self.bc_writer.reserve_constant();
                    self.write1(Op::JumpIfFalseOrNullConstant, c as u32, *line);
                    let right = self.evaluate_expr(right)?;
                    self.store_in_accumulator(right, *line);
                    let end = self.bc_writer.size();
                    self.bc_writer.patch_jump(jump_pos, (end - jump_pos) as u32);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::Or => {
                    let left = self.evaluate_expr(left)?;
                    self.store_in_accumulator(left, *line);
                    let jump_pos = self.bc_writer.size();
                    let c = self.bc_writer.reserve_constant();
                    self.write1(Op::JumpIfNotFalseOrNullConstant, c as u32, *line);
                    let right = self.evaluate_expr(right)?;
                    self.store_in_accumulator(right, *line);
                    let end = self.bc_writer.size();
                    self.bc_writer.patch_jump(jump_pos, (end - jump_pos) as u32);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::DotDot => {
                    let left = self.evaluate_expr(left)?;
                    let left_reg = self.store_in_register(left, *line);
                    let right = self.evaluate_expr(right)?;
                    self.store_in_accumulator(right, *line);
                    self.write1(Op::Range, left_reg as u32, *line);
                    if !(matches!(left, ExprResult::Register(_))) {
                        self.pop_register();
                    }
                    Ok(ExprResult::Accumulator)
                }
                _ => unreachable!(),
            },
            Expr::Unary { op, right, line } => match op {
                TokenType::Minus => self.negate(right, *line),
                TokenType::Bang => self.not(right, *line),
                _ => unreachable!(),
            },
            Expr::Variable { name, line } => match self.resolve_local(name) {
                Some(local) => Ok(ExprResult::Register(local.reg)),
                None => match self.resolve_upvalue(name, *line) {
                    Some(upval) => {
                        self.write1(Op::LoadUpvalue, upval as u32, *line);
                        Ok(ExprResult::Accumulator)
                    }
                    None => {
                        let global = self.get_global(name).ok_or_else(|| CompileError {
                            message: format!("{} is not defined", name),
                            line: *line,
                        })?;
                        self.write1(Op::LoadModuleVariable, global.position, *line);
                        Ok(ExprResult::Accumulator)
                    }
                },
            },
            Expr::String { inner, line } => {
                if inner.is_empty() {
                    let str = self.bc_writer.string_constant("".into());
                    self.write1(Op::LoadConstant, str, *line);
                } else {
                    match &inner[0] {
                        Substring::String(s) => {
                            let str = self.bc_writer.string_constant(s.as_str().into());
                            self.write1(Op::LoadConstant, str, *line);
                        }
                        Substring::Expr(e) => {
                            let expr_res = self.evaluate_expr(e)?;
                            self.expr_to_string(expr_res, *line);
                        }
                    }
                    if inner.len() > 1 {
                        let reg = self.push_register();
                        for i in &inner[1..] {
                            self.write_op_store_register(reg, *line);
                            match i {
                                Substring::String(s) => {
                                    if !s.is_empty() {
                                        let str = self.bc_writer.string_constant(s.as_str().into());
                                        self.write1(Op::LoadConstant, str, *line);
                                    } else {
                                        continue;
                                    }
                                }
                                Substring::Expr(expr) => {
                                    let expr = self.evaluate_expr(expr)?;
                                    self.expr_to_string(expr, *line);
                                }
                            }
                            self.write1(Op::ConcatRegister, reg as u32, *line);
                        }
                        self.pop_register();
                    }
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::Array { inner, line } => {
                let array_reg = match dest {
                    Some(r) => r,
                    None => self.push_register(),
                };
                self.write2(Op::NewArray, inner.len() as u32, array_reg, *line);
                for (index, expr) in inner.iter().enumerate() {
                    let expr_res = self.evaluate_expr(expr)?;
                    self.store_in_accumulator(expr_res, expr.line());
                    self.write2(
                        Op::StoreArrayUnchecked,
                        array_reg,
                        index as u32,
                        expr.line(),
                    );
                }
                if dest.is_none() {
                    self.write_op_load_register(array_reg, *line);
                    self.pop_register();
                    Ok(ExprResult::Accumulator)
                } else {
                    Ok(ExprResult::Register(array_reg))
                }
            }
            Expr::Subscript {
                object,
                subscript,
                line,
            } => {
                let res = self.evaluate_expr(object)?;
                let reg = self.store_in_register(res, *line);
                let subscript = self.evaluate_expr(subscript)?;
                self.store_in_accumulator(subscript, *line);
                self.write1(Op::LoadSubscript, reg as u32, *line);
                if !matches!(res, ExprResult::Register(_)) {
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::Map { inner, line } => {
                let map_reg = match dest {
                    Some(r) => r,
                    None => self.push_register(),
                };
                self.write2(Op::NewMap, inner.len() as u32, map_reg, *line);
                for (key, val) in inner.iter() {
                    let key_res = self.evaluate_expr(key)?;
                    let key_reg = self.store_in_register(key_res, key.line());
                    let val_res = self.evaluate_expr(val)?;
                    self.store_in_accumulator(val_res, val.line());
                    self.write2(Op::StoreSubscript, map_reg, key_reg, val.line());
                    if !(matches!(key_res, ExprResult::Register(_))) {
                        self.pop_register();
                    }
                }
                if dest.is_none() {
                    self.write_op_load_register(map_reg, *line);
                    self.pop_register();
                    Ok(ExprResult::Accumulator)
                } else {
                    Ok(ExprResult::Register(map_reg))
                }
            }
            Expr::Call {
                line,
                function,
                arguments,
            } => {
                let start = self.regcount;
                if arguments.len() >= 25 {
                    return Err(CompileError {
                        message: "Cannot have more than 25 arguments".to_string(),
                        line: *line,
                    });
                }
                for arg in arguments {
                    let reg = self.push_register();
                    let expr = self.evaluate_expr_with_dest(arg, Some(reg))?;
                    self.store_in_specific_register(expr, reg, *line);
                }
                let expr = self.evaluate_expr(function)?;
                self.store_in_accumulator(expr, *line);
                self.write1(Op::Call, start as u32, *line);
                self.bc_writer.write_u8(arguments.len() as u8);
                for _ in 0..arguments.len() {
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::Closure {
                line,
                args,
                body,
                last_line,
            } => {
                let bytecode = self.closure(
                    "<closure>",
                    *line,
                    args,
                    body,
                    BytecodeType::Function,
                    *last_line,
                )?;
                let c = self.bc_writer.fun_constant(bytecode);
                self.write1(Op::MakeFunction, c, *last_line);
                Ok(ExprResult::Accumulator)
            }
            Expr::Member { object, property } => {
                let line = object.line();
                let object_res = self.evaluate_expr(object)?;
                let reg = self.store_in_register(object_res, line);
                let property = self.bc_writer.symbol_constant(property.as_str().into());
                self.write2(Op::LoadProperty, reg, property, line);
                if !matches!(object_res, ExprResult::Register(_)) {
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::ObjectLiteral { line, inner } => {
                let obj_reg = match dest {
                    Some(r) => r,
                    None => self.push_register(),
                };
                self.write2(Op::NewObject, inner.len() as u32, obj_reg, *line);
                for (key, val) in inner.iter() {
                    let sym = self.bc_writer.symbol_constant(key.as_str().into());
                    let val_res = if let Some(val) = val {
                        self.evaluate_expr(val)?
                    } else {
                        self.evaluate_expr(&Expr::Variable {
                            name: key.clone(),
                            line: *line,
                        })?
                    };
                    let line = if let Some(val) = val {
                        val.line()
                    } else {
                        *line
                    };
                    self.store_in_accumulator(val_res, line);
                    self.write2(Op::StoreProperty, obj_reg, sym, line);
                }
                if dest.is_none() {
                    self.write_op_load_register(obj_reg, *line);
                    self.pop_register();
                    Ok(ExprResult::Accumulator)
                } else {
                    Ok(ExprResult::Register(obj_reg))
                }
            }
            Expr::New {
                line,
                class,
                arguments,
            } => {
                let start = self.regcount;
                if arguments.len() >= 25 {
                    return Err(CompileError {
                        message: "Cannot have more than 25 arguments".to_string(),
                        line: *line,
                    });
                }
                self.push_register();
                for arg in arguments {
                    let reg = self.push_register();
                    let expr = self.evaluate_expr_with_dest(arg, Some(reg))?;
                    self.store_in_specific_register(expr, reg, *line);
                }
                let expr = self.evaluate_expr(class)?;
                self.store_in_accumulator(expr, *line);
                self.write1(Op::Construct, start as u32, *line);
                self.bc_writer.write_u8(arguments.len() as u8);
                for _ in 0..arguments.len() {
                    self.pop_register();
                }
                self.pop_register();
                Ok(ExprResult::Accumulator)
            }
            Expr::This { line } => {
                if self.bctype == BytecodeType::Method || self.bctype == BytecodeType::Constructor {
                    Ok(ExprResult::Register(0))
                } else {
                    Err(CompileError {
                        message: "Cannot use this outside method".to_string(),
                        line: *line,
                    })
                }
            }
            Expr::MethodCall {
                object,
                property,
                arguments,
            } => {
                let line = object.line();
                let object_res = self.evaluate_expr(object)?;
                let reg = self.store_in_register(object_res, line);
                let property = self.bc_writer.symbol_constant(property.as_str().into());
                let start = self.regcount;
                if arguments.len() >= 25 {
                    return Err(CompileError {
                        message: "Cannot have more than 25 arguments".to_string(),
                        line,
                    });
                }
                self.push_register();
                for arg in arguments {
                    let reg = self.push_register();
                    let expr = self.evaluate_expr_with_dest(arg, Some(reg))?;
                    self.store_in_specific_register(expr, reg, line);
                }
                self.write3(Op::CallMethod, reg, property, start, line);
                self.bc_writer.write_u8(arguments.len() as u8);
                for _ in 0..arguments.len() {
                    self.pop_register();
                }
                self.pop_register();
                if !matches!(object_res, ExprResult::Register(_)) {
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::SuperCall {
                line,
                method,
                arguments,
            } => {
                if !(self.bctype == BytecodeType::Method
                    || self.bctype == BytecodeType::Constructor)
                {
                    return Err(CompileError {
                        message: "Super calls can be done only in methods".to_string(),
                        line: *line,
                    });
                }
                let property = self.bc_writer.symbol_constant(method.as_str().into());
                let start = self.regcount;
                if arguments.len() >= 25 {
                    return Err(CompileError {
                        message: "Cannot have more than 25 arguments".to_string(),
                        line: *line,
                    });
                }
                self.push_register();
                for arg in arguments {
                    let reg = self.push_register();
                    let expr = self.evaluate_expr_with_dest(arg, Some(reg))?;
                    self.store_in_specific_register(expr, reg, *line);
                }
                self.write2(Op::SuperCall, property, start, *line);
                self.bc_writer.write_u8(arguments.len() as u8);
                for _ in 0..arguments.len() {
                    self.pop_register();
                }
                self.pop_register();
                Ok(ExprResult::Accumulator)
            }
        }
    }

    fn closure(
        &mut self,
        name: &str,
        line: u32,
        args: &[String],
        body: &ClosureBody,
        bctype: BytecodeType,
        last_line: u32,
    ) -> CompileResult<FunctionInfoWriter<'vm>> {
        if args.len() >= 25 {
            return Err(CompileError {
                message: "Cannot have more than 25 arguments".to_string(),
                line,
            });
        }
        let bc = BytecodeCompiler::new(
            self.compiler.take().unwrap(),
            name,
            bctype,
            args.len() as u8,
        );
        let parent = std::mem::replace(self, bc);
        self.parent = Some(Box::new(parent));
        if bctype == BytecodeType::Method || bctype == BytecodeType::Constructor {
            self.new_local("this".to_string(), false);
        }
        for arg in args {
            self.new_local(arg.clone(), true);
        }
        match body {
            ClosureBody::Block(body) => {
                self.compile_statments(body);
                if !matches!(body.last(), Some(Statement::Return { .. })) {
                    if bctype == BytecodeType::Constructor {
                        self.write0(Op::LoadR0, last_line);
                    } else {
                        self.write0(Op::LoadNull, last_line);
                    }
                    self.write0(Op::Return, last_line);
                }
            }
            ClosureBody::Expr(body) => {
                match self.evaluate_expr(body) {
                    Ok(res) => {
                        self.store_in_accumulator(res, last_line);
                    }
                    Err(e) => self.error(e),
                }
                self.bc_writer.shrink();
                self.bc_writer.set_max_registers(self.max_registers);
                self.write0(Op::Return, last_line);
            }
        }
        let parent = *self.parent.take().unwrap();
        let mut bc = std::mem::replace(self, parent);
        self.compiler = bc.compiler.take();
        Ok(bc.bc_writer)
    }

    fn negate(&mut self, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        if let Expr::Literal {
            inner: TokenType::FloatLiteral(f),
            line,
        } = right
        {
            let c = self.bc_writer.float_constant(-f);
            self.write1(Op::LoadConstant, c, *line);
            return Ok(ExprResult::Accumulator);
        }
        match self.evaluate_expr(right)? {
            ExprResult::Register(r) => {
                self.write_op_load_register(r, line);
                self.write0(Op::Negate, line);
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Accumulator => {
                self.write0(Op::Negate, line);
                Ok(ExprResult::Accumulator)
            }
            ExprResult::Int(i) => Ok(ExprResult::Int(i.checked_neg().ok_or_else(|| {
                CompileError {
                    message: format!(
                        "Cannot negate {} as the result cannot be stored in an Int",
                        i
                    ),
                    line,
                }
            })?)),
        }
    }

    fn expr_to_string(&mut self, expr_res: ExprResult, line: u32) {
        let reg = self.store_in_register(expr_res, line);
        let property = self.bc_writer.symbol_constant("toString".into());
        let start = self.regcount;
        self.push_register();
        self.write3(Op::CallMethod, reg, property, start, line);
        self.bc_writer.write_u8(0);
        self.pop_register();
        if !matches!(expr_res, ExprResult::Register(_)) {
            self.pop_register();
        }
    }

    fn not(&mut self, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        let result = self.evaluate_expr(right)?;
        self.store_in_accumulator(result, line);
        self.write0(Op::Not, line);
        Ok(ExprResult::Accumulator)
    }

    fn concat(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        let left = self.evaluate_expr(left)?;
        let reg;
        match left {
            ExprResult::Register(r) => reg = r,
            ExprResult::Accumulator => {
                reg = self.push_register();
                self.write_op_store_register(reg, line)
            }
            ExprResult::Int(_) => {
                return Err(CompileError {
                    message:
                        "Can only perform concat operation on strings. Consider using interpolation"
                            .into(),
                    line,
                })
            }
        }
        let right = self.evaluate_expr(right)?;
        if matches!(left, ExprResult::Accumulator) {
            self.pop_register();
        }
        match right {
            ExprResult::Int(_) => Err(CompileError {
                message:
                    "Can only perform concat operation on strings. Consider using interpolation"
                        .into(),
                line,
            }),
            right => {
                self.store_in_accumulator(right, line);
                self.write1(Op::ConcatRegister, reg as u32, line);
                Ok(ExprResult::Accumulator)
            }
        }
    }

    fn equal(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<()> {
        match left {
            Expr::Subscript {
                object,
                subscript,
                line,
            } => {
                let object = self.evaluate_expr(object)?;
                let object_reg = self.store_in_register(object, *line);
                let subscript = self.evaluate_expr(subscript)?;
                let subscript_reg = self.store_in_register(subscript, *line);
                let right = self.evaluate_expr(right)?;
                self.store_in_accumulator(right, *line);
                self.write2(Op::StoreSubscript, object_reg, subscript_reg, *line);
                if !matches!(subscript, ExprResult::Register(_)) {
                    self.pop_register();
                }
                if !matches!(object, ExprResult::Register(_)) {
                    self.pop_register();
                }
            }
            Expr::Variable { name, line } => {
                if let Some(local) = self.resolve_local(name).cloned() {
                    let expr_res = self.evaluate_expr_with_dest(right, Some(local.reg))?;
                    if !local.mutable {
                        return Err(CompileError {
                            message: format!("Cannot modify constant {}", name),
                            line: *line,
                        });
                    }
                    self.store_in_specific_register(expr_res, local.reg, *line);
                } else if let Some(upval) = self.resolve_upvalue(name, *line) {
                    let res = self.evaluate_expr(right)?;
                    if !self.upvalues[upval as usize].mutable {
                        return Err(CompileError {
                            message: format!("Cannot modify constant {}", name),
                            line: *line,
                        });
                    }
                    self.store_in_accumulator(res, *line);
                    self.write1(Op::StoreUpvalue, upval as u32, *line);
                } else {
                    let global = self.get_global(name).ok_or_else(|| CompileError {
                        message: format!("{} is not defined", name),
                        line: *line,
                    })?;
                    let res = self.evaluate_expr(right)?;
                    self.store_in_accumulator(res, *line);
                    if !global.mutable {
                        return Err(CompileError {
                            message: format!("Cannot modify constant {}", name),
                            line: *line,
                        });
                    } else {
                        self.write1(Op::StoreModuleVariable, global.position, *line);
                    }
                }
            }
            Expr::Member { object, property } => {
                let res = self.evaluate_expr(object)?;
                let object = self.store_in_register(res, line);
                let sym = self.bc_writer.symbol_constant(property.as_str().into());
                let right = self.evaluate_expr(right)?;
                self.store_in_accumulator(right, line);
                self.write2(Op::StoreProperty, object, sym, line);
                if !matches!(res, ExprResult::Register(_)) {
                    self.pop_register();
                }
            }
            _ => {
                return Err(CompileError {
                    message: "Invalid target for assignment".into(),
                    line,
                })
            }
        }
        Ok(())
    }

    binary_op!(add, AddRegister, AddInt, add, checked_add, "add");
    binary_op!(
        subtract,
        SubtractRegister,
        SubtractInt,
        sub,
        checked_sub,
        "subtract"
    );
    binary_op!(
        multiply,
        MultiplyRegister,
        MultiplyInt,
        mul,
        checked_mul,
        "multiply"
    );
    binary_op!(
        divide,
        DivideRegister,
        DivideInt,
        div,
        checked_div,
        "divide"
    );
    binary_op!(modulus, ModRegister, ModInt, rem, checked_mod, "mod");

    comparing_binary_op!(equal_equal,Equal,==);
    comparing_binary_op!(equal_equal_equal,StrictEqual,==);
    comparing_binary_op!(not_equal,NotEqual,!=);
    comparing_binary_op!(not_equal_equal,StrictNotEqual,!=);
    comparing_binary_op!(greater_than,GreaterThan,>);
    comparing_binary_op!(lesser_than,LesserThan,<);
    comparing_binary_op!(greater_than_or_equal,GreaterThanOrEqual,>=);
    comparing_binary_op!(lesser_than_or_equal,LesserThanOrEqual,<=);
}

trait CheckedMod {
    fn checked_mod(self, rhs: Self) -> Option<Self>
    where
        Self: Sized;
}
impl CheckedMod for i32 {
    fn checked_mod(self, rhs: i32) -> Option<i32> {
        if self == i32::MIN && rhs == -1 {
            Some(0)
        } else {
            self.checked_rem(rhs)
        }
    }
}

use cxx::Exception;

use crate::parser::ClosureBody;
use crate::parser::Statement;
use crate::parser::Substring;
use crate::vm::FunctionInfoWriter;
use crate::vm::Op;
use crate::vm::VM;
use crate::CompileError;
use crate::CompileResult;
use crate::{parser::Expr, scanner::TokenType};
use std::collections::HashMap;
use std::convert::TryFrom;

pub struct Compiler<'vm> {
    globals: &'vm mut HashMap<String, u32>,
    errors: Vec<CompileError>,
    vm: &'vm VM,
}

impl<'vm> Compiler<'vm> {
    pub fn new(vm: &'vm VM, globals: &'vm mut HashMap<String, u32>) -> Self {
        Self {
            vm,
            globals,
            errors: vec![],
        }
    }

    pub fn exec(
        mut self,
        ast: Vec<Statement>,
    ) -> Result<FunctionInfoWriter<'vm>, Vec<CompileError>> {
        let mut b = BytecodeCompiler::new(&mut self, "<script>", BytecodeType::Script, 0);
        b.evaluate_statments(&ast);
        b.bytecode.write_u8(Op::Exit.repr);
        let bytecode = b.bytecode;
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
        let mut b = BytecodeCompiler::new(&mut self, "<script>", BytecodeType::Script, 0);
        match b.evaluate_expr(ast) {
            Ok(er) => {
                if let Err(e) = b.store_in_accumulator(er, 1) {
                    b.error(e)
                }
            }
            Err(e) => b.error(e),
        }
        b.bytecode.shrink();
        b.bytecode.set_max_registers(b.max_registers);
        b.bytecode.write_u8(Op::Exit.repr);
        let bytecode = b.bytecode;
        if self.errors.is_empty() {
            Ok(bytecode)
        } else {
            Err(self.errors)
        }
    }
}

struct BytecodeCompiler<'c, 'vm> {
    compiler: Option<&'c mut Compiler<'vm>>,
    parent: Option<Box<BytecodeCompiler<'c, 'vm>>>,
    bytecode: FunctionInfoWriter<'vm>,
    locals: Vec<HashMap<String, Local>>,
    regcount: u16,
    max_registers: u16,
    op_positions: Vec<usize>,
    loops: Vec<Loop>,
    bctype: BytecodeType,
    upvalues: Vec<UpValue>,
}

#[derive(PartialEq, Eq)]
enum BytecodeType {
    Script,
    Function,
    Closure,
}

enum Loop {
    While {
        loop_start: usize,
        breaks: Vec<usize>,
    },
    For {
        continues: Vec<usize>,
        breaks: Vec<usize>,
    },
}

#[derive(Clone)]
struct Local {
    reg: u16,
    mutable: bool,
    is_captured: bool,
}

struct UpValue {
    index: u16,
    is_local: bool,
    mutable: bool,
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn new(c: &'c mut Compiler<'vm>, name: &str, bctype: BytecodeType, arity: u8) -> Self {
        Self {
            bytecode: c.vm.new_function_info(name.into(), arity),
            locals: vec![HashMap::default()],
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

    fn push_register(&mut self, line: u32) -> CompileResult<u16> {
        if self.regcount == u16::MAX {
            Err(CompileError {
                message: "Cannot have more than 65535 locals and temporaries".into(),
                line,
            })
        } else {
            self.regcount += 1;
            if self.regcount > self.max_registers {
                self.max_registers = self.regcount;
            }
            Ok(self.regcount - 1)
        }
    }

    fn pop_register(&mut self) {
        self.regcount -= 1;
    }

    fn get_global(&self, name: &str) -> Option<u32> {
        self.compiler.as_ref().unwrap().globals.get(name).cloned()
    }

    fn new_global(&mut self, name: &str) -> u32 {
        let comp = &mut self.compiler.as_mut().unwrap();
        let len = comp.globals.len() as u32;
        comp.globals.insert(name.to_string(), len);
        comp.vm.add_global(name.into());
        len
    }

    fn error(&mut self, e: CompileError) {
        self.compiler.as_mut().unwrap().errors.push(e)
    }

    fn write0(&mut self, op: Op, line: u32) {
        let pos = self.bytecode.write_op(op, line);
        self.op_positions.push(pos);
    }

    fn write1(&mut self, op: Op, u: u32, line: u32) {
        if let Ok(u) = u8::try_from(u) {
            self.write0(op, line);
            self.bytecode.write_u8(u);
        } else if let Ok(u) = u16::try_from(u) {
            self.write0(Op::Wide, line);
            self.bytecode.write_u8(op.repr);
            self.bytecode.write_u16(u);
        } else {
            self.write0(Op::ExtraWide, line);
            self.bytecode.write_u8(op.repr);
            self.bytecode.write_u32(u);
        }
    }

    fn write2(&mut self, op: Op, u1: u16, u2: u16, line: u32) {
        match (u8::try_from(u1), u8::try_from(u2)) {
            (Ok(u1), Ok(u2)) => {
                self.write0(op, line);
                self.bytecode.write_u8(u1);
                self.bytecode.write_u8(u2)
            }
            _ => {
                self.write0(Op::Wide, line);
                self.bytecode.write_u8(op.repr);
                self.bytecode.write_u16(u1);
                self.bytecode.write_u16(u2)
            }
        }
    }

    fn write2_u32(&mut self, op: Op, u1: u32, u2: u16, line: u32) {
        match u16::try_from(u1) {
            Ok(u1) => self.write2(op, u1, u2, line),
            Err(_) => {
                self.write0(Op::ExtraWide, line);
                self.bytecode.write_u8(op.repr);
                self.bytecode.write_u32(u1);
                self.bytecode.write_u32(u2 as u32);
            }
        }
    }

    fn pop_last_op(&mut self) {
        let pos = self.op_positions.pop().unwrap();
        self.bytecode.pop_last_op(pos);
    }
}

impl<'c, 'vm> BytecodeCompiler<'c, 'vm> {
    fn write_op_store_register(&mut self, reg: u16, line: u32) {
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
                self.write1(Op::StoreRegister, reg as u32, line);
            }
        }
    }

    fn write_op_load_register(&mut self, reg: u16, line: u32) {
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
                self.write1(Op::LoadRegister, reg as u32, line);
            }
        }
    }

    fn load_const(&mut self, constant: Result<u16, Exception>, line: u32) -> CompileResult<()> {
        self.write1(
            Op::LoadConstant,
            constant.map_err(|_| CompileError {
                line,
                message: "Cannot have more than 65535 constants per function".into(),
            })? as u32,
            line,
        );
        Ok(())
    }

    fn reserve_int(&mut self, line: u32) -> CompileResult<u16> {
        self.bytecode.reserve_constant().map_err(|_| CompileError {
            line,
            message: "Cannot have more than 65535 constants per function".into(),
        })
    }

    fn load_int(&mut self, i: i32, line: u32) -> CompileResult<()> {
        match i {
            0..=255 => self.write1(Op::LoadSmallInt, i as u32, line),
            -256..=-1 => self.write1(Op::LoadSmallInt, i as i8 as u8 as u32, line),
            _ => {
                let c = self.bytecode.int_constant(i);
                self.load_const(c, line)?;
            }
        }
        Ok(())
    }

    fn new_local(&mut self, line: u32, name: String, mutable: bool) -> CompileResult<u16> {
        let reg = self.push_register(line)?;
        self.locals.last_mut().unwrap().insert(
            name,
            Local {
                mutable,
                reg,
                is_captured: false,
            },
        );
        Ok(reg)
    }
}

#[derive(Debug, Clone, Copy)]
enum ExprResult {
    Register(u16),
    Accumulator,
    Int(i32),
}

macro_rules! binary_op {
    ($op:ident,$register_inst:ident,$int_inst:ident,$op_fn:ident,$op_checked_fn:ident,$op_name:tt) => {
        fn $op(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
            let left = self.evaluate_expr(left)?;
            let mut reg = 0;
            if !matches!(left, ExprResult::Register(_)) {
                reg = self.push_register(line)?;
                self.store_in_accumulator(left, line)?;
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
                                "Cannot {} {} and {} as the result cannot be stored in an int",
                                $op_name, i1, i2
                            ),
                            line,
                        },
                    )?))
                }
                (left, ExprResult::Int(i)) => {
                    self.undo_save_to_register(left);
                    self.store_in_accumulator(left, line)?;
                    self.write1(Op::$int_inst, signed_to_unsigned(i), line);
                    Ok(ExprResult::Accumulator)
                }
                (ExprResult::Register(r), right) => {
                    self.store_in_accumulator(right, line)?;
                    self.write1(Op::$register_inst, r as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (_, right) => {
                    self.store_in_accumulator(right, line)?;
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
                reg = self.push_register(line)?;
                self.store_in_accumulator(left, line)?;
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
                    self.store_in_accumulator(right, line)?;
                    self.write1(Op::$inst, r as u32, line);
                    Ok(ExprResult::Accumulator)
                }
                (_, right) => {
                    self.store_in_accumulator(right, line)?;
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

    fn store_in_accumulator(&mut self, result: ExprResult, line: u32) -> CompileResult<()> {
        match result {
            ExprResult::Register(reg) => {
                self.write_op_load_register(reg, line);
            }
            ExprResult::Accumulator => {}
            ExprResult::Int(i) => {
                self.load_int(i, line)?;
            }
        }
        Ok(())
    }

    fn store_in_register(&mut self, result: ExprResult, line: u32) -> CompileResult<u16> {
        if let ExprResult::Register(r) = result {
            Ok(r)
        } else {
            self.store_in_accumulator(result, line)?;
            let reg = self.push_register(line)?;
            self.write_op_store_register(reg, line);
            Ok(reg)
        }
    }

    fn store_in_specific_register(
        &mut self,
        result: ExprResult,
        reg: u16,
        line: u32,
    ) -> CompileResult<()> {
        match result {
            ExprResult::Register(r) => {
                if r != reg {
                    self.write2(Op::Move, r, reg, line)
                }
            }
            ExprResult::Accumulator => self.write_op_store_register(reg, line),
            ExprResult::Int(i) => {
                self.load_int(i, line)?;
                self.write_op_store_register(reg, line)
            }
        }
        Ok(())
    }

    fn resolve_local(&mut self, name: &str) -> Option<&mut Local> {
        for locals in self.locals.iter_mut().rev() {
            if let Some(local) = locals.get_mut(name) {
                return Some(local);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, name: &str, line: u32) -> CompileResult<Option<u16>> {
        match self.parent.as_mut() {
            Some(parent) => match parent.resolve_local(name) {
                Some(l) => {
                    l.is_captured = true;
                    let l = l.clone();
                    let u = self.add_upvalue(l.reg, true, l.mutable, line)?;
                    Ok(Some(u))
                }
                None => match parent.resolve_upvalue(name, line) {
                    Ok(Some(u)) => {
                        let mutable = parent.upvalues[u as usize].mutable;
                        Ok(Some(self.add_upvalue(u, false, mutable, line)?))
                    }
                    Ok(None) => Ok(None),
                    Err(e) => Err(e),
                },
            },
            None => Ok(None),
        }
    }

    fn add_upvalue(
        &mut self,
        index: u16,
        is_local: bool,
        mutable: bool,
        line: u32,
    ) -> CompileResult<u16> {
        for (i, upvalue) in self.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return Ok(i as u16);
            }
        }
        if self.upvalues.len() == 65536 {
            Err(CompileError {
                message: "Cannot have more than 65536 upvalues".into(),
                line,
            })
        } else {
            self.upvalues.push(UpValue {
                index,
                is_local,
                mutable,
            });
            self.bytecode.add_upvalue(index, is_local);
            Ok((self.upvalues.len() - 1) as u16)
        }
    }

    fn evaluate_statments(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.evaluate_statement(statement)
        }
        self.bytecode.shrink();
        self.bytecode.set_max_registers(self.max_registers);
    }

    fn var_declaration(
        &mut self,
        name: &str,
        expr: &Expr,
        line: u32,
        mutable: bool,
    ) -> CompileResult<()> {
        if self.locals.last().unwrap().contains_key(name) {
            return Err(CompileError {
                message: format!("Cannot redeclare variable {} in the same scope", name),
                line,
            });
        }
        let reg = self.new_local(line, name.into(), mutable)?;
        let res = self.evaluate_expr_with_dest(expr, Some(reg))?;
        self.store_in_specific_register(res, reg, line)?;
        Ok(())
    }

    fn evaluate_statement(&mut self, statement: &Statement) {
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
                Statement::VarDeclaration {
                    name,
                    expr,
                    mutable,
                    line,
                } => {
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
                        Err(e) => self.error(e.clone()),
                        Ok(res) => {
                            if let Err(e) = self.store_in_accumulator(res, line) {
                                self.error(e);
                            }
                        }
                    }
                    let c = self.reserve_int(line)?;
                    let cond_check = self.bytecode.size();
                    self.write1(Op::JumpIfFalseOrNullConstant, c.into(), line);
                    self.block(block, *if_end);
                    let if_end_pos = self.bytecode.size();
                    if let Some(else_stmt) = else_stmt {
                        let c = self.reserve_int(*if_end)?;
                        self.write1(Op::JumpConstant, c.into(), *if_end);
                        let jump_end = self.bytecode.size();
                        self.bytecode
                            .patch_jump(cond_check, (jump_end - cond_check) as u32);
                        self.evaluate_statement(else_stmt);
                        let else_end = self.bytecode.size();
                        self.bytecode
                            .patch_jump(if_end_pos, (else_end - if_end_pos) as u32);
                    } else {
                        self.bytecode
                            .patch_jump(cond_check, (if_end_pos - cond_check) as u32);
                    }
                }
                Statement::While {
                    condition,
                    block,
                    end_line,
                } => {
                    let loop_start = self.bytecode.size();
                    self.loops.push(Loop::While {
                        loop_start,
                        breaks: vec![],
                    });
                    match self.evaluate_expr(condition) {
                        Err(e) => self.error(e.clone()),
                        Ok(res) => {
                            if let Err(e) = self.store_in_accumulator(res, condition.line()) {
                                self.error(e);
                            }
                        }
                    }

                    let c = self.reserve_int(condition.line())?;
                    let loop_cond_check = self.bytecode.size();
                    self.write1(Op::JumpIfFalseOrNullConstant, c as u32, condition.line());
                    self.block(block, *end_line);
                    let almost_loop_end = self.bytecode.size();
                    self.write1(
                        Op::JumpBack,
                        (almost_loop_end - loop_start) as u32,
                        *end_line,
                    );
                    let loop_end = self.bytecode.size();
                    self.bytecode
                        .patch_jump(loop_cond_check, (loop_end - loop_cond_check) as u32);
                    match self.loops.last().unwrap() {
                        Loop::While { breaks, .. } => {
                            for b in breaks.iter() {
                                self.bytecode.patch_jump(*b, (loop_end - b) as u32);
                            }
                        }
                        _ => unreachable!(),
                    }
                    self.loops.pop();
                }
                Statement::For {
                    iter,
                    start,
                    end,
                    block,
                    begin_line,
                    end_line,
                } => {
                    let start = self.evaluate_expr(start);
                    if let Err(ref e) = start {
                        self.error(e.clone());
                    }
                    self.locals.push(HashMap::default());
                    let iter_reg = self.new_local(*begin_line, iter.clone(), false)?;
                    if let Ok(start) = start {
                        if let Err(e) =
                            self.store_in_specific_register(start, iter_reg, *begin_line)
                        {
                            self.error(e);
                        }
                    }
                    let end = self.evaluate_expr(end);
                    if let Err(ref e) = end {
                        self.error(e.clone());
                    }
                    let end_reg = self.new_local(*begin_line, "$end".into(), false)?;
                    if let Ok(end) = end {
                        if let Err(e) = self.store_in_specific_register(end, end_reg, *begin_line) {
                            self.error(e);
                        }
                    }
                    let c = self.reserve_int(*begin_line)?;
                    let before_loop_prep = self.bytecode.size();
                    self.write2_u32(Op::BeginForLoopConstant, c as u32, iter_reg, *begin_line);
                    let loop_start = self.bytecode.size();
                    self.loops.push(Loop::For {
                        breaks: vec![],
                        continues: vec![],
                    });
                    for stmt in block {
                        self.evaluate_statement(stmt);
                    }
                    let last_block = self.locals.last().unwrap();
                    if last_block.values().any(|l| l.is_captured) {
                        self.write1(Op::Close, (self.regcount - 2) as u32, *end_line);
                    }
                    let loop_almost_end = self.bytecode.size();
                    self.write2_u32(
                        Op::ForLoop,
                        (loop_almost_end - loop_start) as u32,
                        iter_reg,
                        *end_line,
                    );
                    let loop_end = self.bytecode.size();
                    self.bytecode
                        .patch_jump(before_loop_prep, (loop_end - before_loop_prep) as u32);
                    match self.loops.last().unwrap() {
                        Loop::For { continues, breaks } => {
                            for b in breaks.iter() {
                                self.bytecode.patch_jump(*b, (loop_end - b) as u32);
                            }
                            for c in continues.iter() {
                                self.bytecode.patch_jump(*c, (loop_almost_end - c) as u32);
                            }
                        }
                        _ => unreachable!(),
                    }
                    self.loops.pop();
                    let last_block = self.locals.pop().unwrap();
                    self.regcount -= last_block.len() as u16;
                }
                Statement::Break { line } => self.break_stmt(*line)?,
                Statement::Continue { line } => self.continue_stmt(*line)?,
                Statement::Function {
                    line,
                    name,
                    arguments,
                    body,
                    last_line,
                } => {
                    if self.bctype != BytecodeType::Script || self.locals.len() != 1 {
                        return Err(CompileError {
                            message: "Global functions must be declared at the topmost scope"
                                .into(),
                            line: *line,
                        });
                    }
                    if arguments.len() >= 25 {
                        return Err(CompileError {
                            message: "Cannot have more than 25 arguments".to_string(),
                            line: *line,
                        });
                    }
                    let mut bc = BytecodeCompiler::new(
                        self.compiler.take().unwrap(),
                        name.as_str(),
                        BytecodeType::Function,
                        arguments.len() as u8,
                    );
                    for arg in arguments {
                        bc.new_local(*line, arg.clone(), true)?;
                    }
                    bc.evaluate_statments(body);
                    if !matches!(body.last(), Some(Statement::Return { .. })) {
                        bc.write0(Op::LoadNull, *last_line);
                        bc.write0(Op::Return, *last_line);
                    }
                    self.compiler = Some(bc.compiler.take().unwrap());
                    let c = self.bytecode.fun_constant(bc.bytecode);
                    self.write1(
                        Op::MakeFunction,
                        c.map_err(|_| CompileError {
                            line: *last_line,
                            message: "Cannot have more than 65535 constants per function".into(),
                        })? as u32,
                        *last_line,
                    );
                    let g = self
                        .get_global(name.as_str())
                        .unwrap_or_else(|| self.new_global(name.as_str()));
                    self.write1(Op::StoreGlobal, g, *line);
                }
                Statement::Return { line, expr } => {
                    if self.bctype == BytecodeType::Script {
                        return Err(CompileError {
                            message: "Cannot use return outside a function or method".into(),
                            line: *line,
                        });
                    }
                    if let Some(expr) = expr {
                        let expr_res = self.evaluate_expr(expr)?;
                        self.store_in_accumulator(expr_res, *line)?;
                    } else {
                        self.write0(Op::LoadNull, *line);
                    }
                    self.write0(Op::Return, *line);
                }
                Statement::Print(e) => {
                    let expr_res = self.evaluate_expr(e)?;
                    self.store_in_accumulator(expr_res, e.line())?;
                    self.write0(Op::Print, e.line());
                }
                Statement::Panic(e) => {
                    let expr_res = self.evaluate_expr(e)?;
                    self.store_in_accumulator(expr_res, e.line())?;
                    self.write0(Op::Panic, e.line());
                }
                Statement::TryCatch {
                    try_block,
                    try_end,
                    error_var,
                    catch_block,
                    catch_end,
                } => {
                    self.block(try_block, *try_end);
                    let c = self.reserve_int(*try_end)?;
                    self.write1(Op::JumpConstant, c.into(), *try_end);
                    let jump_pos = self.bytecode.size();
                    self.locals.push(HashMap::default());
                    let error_reg = self.push_register(*try_end)?;
                    self.locals.last_mut().unwrap().insert(
                        error_var.clone(),
                        Local {
                            mutable: false,
                            reg: error_reg,
                            is_captured: false,
                        },
                    );
                    for stmt in catch_block {
                        self.evaluate_statement(stmt);
                    }
                    let last_block = self.locals.pop().unwrap();
                    self.regcount -= last_block.len() as u16;
                    if last_block.values().any(|l| l.is_captured) {
                        self.write1(Op::Close, self.regcount as u32, *catch_end);
                    }
                    let catch_end_pos = self.bytecode.size();
                    self.bytecode
                        .patch_jump(jump_pos, (catch_end_pos - jump_pos) as u32);
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
            self.evaluate_statement(stmt);
        }
        let last_block = self.locals.pop().unwrap();
        self.regcount -= last_block.len() as u16;
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
            let break_pos = self.bytecode.size();
            match self.loops.last_mut().unwrap() {
                Loop::While { breaks, .. } => breaks.push(break_pos),
                Loop::For { breaks, .. } => breaks.push(break_pos),
            }
            let c = self.reserve_int(line)?;
            self.write1(Op::JumpConstant, c.into(), line);
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
                Loop::While { loop_start, .. } => {
                    let continue_pos = self.bytecode.size();
                    let loop_start = *loop_start;
                    self.write1(Op::JumpBack, (continue_pos - loop_start) as u32, line);
                }
                Loop::For { continues, .. } => {
                    let continue_pos = self.bytecode.size();
                    continues.push(continue_pos);
                    let c = self.reserve_int(line)?;
                    self.write1(Op::JumpConstant, c.into(), line);
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
        dest: Option<u16>,
    ) -> CompileResult<ExprResult> {
        match expr {
            Expr::Literal { inner, line } => match inner {
                TokenType::IntLiteral(i) => Ok(ExprResult::Int(*i)),
                TokenType::FloatLiteral(f) => Ok({
                    let c = self.bytecode.float_constant(*f);
                    self.load_const(c, *line)?;
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
                    let sym = self.bytecode.symbol_constant(sym.as_str().into());
                    self.load_const(sym, *line)?;
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
                    self.store_in_accumulator(left, *line)?;
                    let jump_pos = self.bytecode.size();
                    let c = self.reserve_int(*line)?;
                    self.write1(Op::JumpIfFalseOrNullConstant, c as u32, *line);
                    let right = self.evaluate_expr(right)?;
                    self.store_in_accumulator(right, *line)?;
                    let end = self.bytecode.size();
                    self.bytecode.patch_jump(jump_pos, (end - jump_pos) as u32);
                    Ok(ExprResult::Accumulator)
                }
                TokenType::Or => {
                    let left = self.evaluate_expr(left)?;
                    self.store_in_accumulator(left, *line)?;
                    let jump_pos = self.bytecode.size();
                    let c = self.reserve_int(*line)?;
                    self.write1(Op::JumpIfNotFalseOrNullConstant, c as u32, *line);
                    let right = self.evaluate_expr(right)?;
                    self.store_in_accumulator(right, *line)?;
                    let end = self.bytecode.size();
                    self.bytecode.patch_jump(jump_pos, (end - jump_pos) as u32);
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
                None => match self.resolve_upvalue(name, *line)? {
                    Some(upval) => {
                        self.write1(Op::LoadUpvalue, upval as u32, *line);
                        Ok(ExprResult::Accumulator)
                    }
                    None => {
                        let global = self
                            .get_global(name)
                            .unwrap_or_else(|| self.new_global(name));
                        self.write1(Op::LoadGlobal, global, *line);
                        Ok(ExprResult::Accumulator)
                    }
                },
            },
            Expr::String { inner, line } => {
                if inner.is_empty() {
                    let str = self.bytecode.string_constant("".into());
                    self.load_const(str, *line)?;
                } else {
                    match &inner[0] {
                        Substring::String(s) => {
                            let str = self.bytecode.string_constant(s.as_str().into());
                            self.load_const(str, *line)?;
                        }
                        Substring::Expr(e) => {
                            let expr_res = self.evaluate_expr(e)?;
                            self.store_in_accumulator(expr_res, *line)?;
                            self.write0(Op::ToString, *line);
                        }
                    }
                    if inner.len() > 1 {
                        let reg = self.push_register(*line)?;
                        for i in &inner[1..] {
                            self.write_op_store_register(reg, *line);
                            match i {
                                Substring::String(s) => {
                                    if !s.is_empty() {
                                        let str = self.bytecode.string_constant(s.as_str().into());
                                        self.load_const(str, *line)?;
                                    } else {
                                        continue;
                                    }
                                }
                                Substring::Expr(expr) => {
                                    let expr = self.evaluate_expr(expr)?;
                                    self.store_in_accumulator(expr, *line)?;
                                    self.write0(Op::ToString, *line);
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
                if inner.is_empty() {
                    self.write0(Op::EmptyArray, *line);
                    return Ok(ExprResult::Accumulator);
                }
                let array_reg = match dest {
                    Some(r) => r,
                    None => self.push_register(*line)?,
                };
                self.write2(
                    Op::NewArray,
                    u16::try_from(inner.len()).map_err(|_| CompileError {
                        message: "Array literal can have upto 65535 elements".to_string(),
                        line: *line,
                    })?,
                    array_reg,
                    *line,
                );
                for (index, expr) in inner.iter().enumerate() {
                    let expr_res = self.evaluate_expr(expr)?;
                    self.store_in_accumulator(expr_res, expr.line())?;
                    self.write2(
                        Op::StoreArrayUnchecked,
                        array_reg,
                        u16::try_from(index).unwrap(),
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
                let reg = self.store_in_register(res, *line)?;
                let subscript = self.evaluate_expr(subscript)?;
                self.store_in_accumulator(subscript, *line)?;
                self.write1(Op::LoadSubscript, reg as u32, *line);
                if !matches!(res, ExprResult::Register(_)) {
                    self.pop_register();
                }
                Ok(ExprResult::Accumulator)
            }
            Expr::Map { inner, line } => {
                if inner.is_empty() {
                    self.write0(Op::EmptyMap, *line);
                    return Ok(ExprResult::Accumulator);
                }
                let map_reg = match dest {
                    Some(r) => r,
                    None => self.push_register(*line)?,
                };
                self.write2(
                    Op::NewMap,
                    u16::try_from(inner.len()).map_err(|_| CompileError {
                        message: "Map literal can have up to 65535 elements".to_string(),
                        line: *line,
                    })?,
                    map_reg,
                    *line,
                );
                for (key, val) in inner.iter() {
                    let key_res = self.evaluate_expr(key)?;
                    let key_reg = self.store_in_register(key_res, key.line())?;
                    let val_res = self.evaluate_expr(val)?;
                    self.store_in_accumulator(val_res, val.line())?;
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
                    let reg = self.push_register(*line)?;
                    let expr = self.evaluate_expr_with_dest(arg, Some(reg))?;
                    self.store_in_specific_register(expr, reg, *line)?;
                }
                let expr = self.evaluate_expr(function)?;
                self.store_in_accumulator(expr, *line)?;
                let call_op = match arguments.len() {
                    0 => Op::Call0Argument,
                    1 => Op::Call1Argument,
                    2 => Op::Call2Argument,
                    3 => Op::Call3Argument,
                    _ => Op::Call,
                };
                self.write1(call_op, start as u32, *line);
                if arguments.len() > 3 {
                    self.bytecode.write_u8(arguments.len() as u8);
                }
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
                if args.len() >= 25 {
                    return Err(CompileError {
                        message: "Cannot have more than 25 arguments".to_string(),
                        line: *line,
                    });
                }
                let bc = BytecodeCompiler::new(
                    self.compiler.take().unwrap(),
                    "<closure>",
                    BytecodeType::Closure,
                    args.len() as u8,
                );
                let parent = std::mem::replace(self, bc);
                self.parent = Some(Box::new(parent));
                for arg in args {
                    self.new_local(*line, arg.clone(), true)?;
                }
                match body {
                    ClosureBody::Block(body) => {
                        self.evaluate_statments(body);
                        if !matches!(body.last(), Some(Statement::Return { .. })) {
                            self.write0(Op::LoadNull, *last_line);
                            self.write0(Op::Return, *last_line);
                        }
                    }
                    ClosureBody::Expr(body) => {
                        match self.evaluate_expr(body) {
                            Ok(res) => {
                                if let Err(e) = self.store_in_accumulator(res, *line) {
                                    self.error(e)
                                }
                            }
                            Err(e) => self.error(e),
                        }
                        self.write0(Op::Return, *line);
                    }
                }
                let parent = *self.parent.take().unwrap();
                let mut bc = std::mem::replace(self, parent);
                self.compiler = bc.compiler.take();
                let c = self.bytecode.fun_constant(bc.bytecode);
                self.write1(
                    Op::MakeFunction,
                    c.map_err(|_| CompileError {
                        line: *last_line,
                        message: "Cannot have more than 65535 constants per function".into(),
                    })? as u32,
                    *last_line,
                );
                Ok(ExprResult::Accumulator)
            }
        }
    }
    fn negate(&mut self, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        if let Expr::Literal {
            inner: TokenType::FloatLiteral(f),
            line,
        } = right
        {
            let c = self.bytecode.float_constant(-f);
            self.load_const(c, *line)?;
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
                        "Cannot negate {} as the result cannot be stored in an int",
                        i
                    ),
                    line,
                }
            })?)),
        }
    }

    fn not(&mut self, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        let result = self.evaluate_expr(right)?;
        self.store_in_accumulator(result, line)?;
        self.write0(Op::Not, line);
        Ok(ExprResult::Accumulator)
    }

    fn concat(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<ExprResult> {
        let left = self.evaluate_expr(left)?;
        let reg;
        match left {
            ExprResult::Register(r) => reg = r,
            ExprResult::Accumulator => {
                reg = self.push_register(line)?;
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
                self.store_in_accumulator(right, line)?;
                self.write1(Op::ConcatRegister, reg as u32, line);
                Ok(ExprResult::Accumulator)
            }
        }
    }

    fn equal(&mut self, left: &Expr, right: &Expr, line: u32) -> CompileResult<()> {
        if let Expr::Subscript {
            object,
            subscript,
            line,
        } = left
        {
            let object = self.evaluate_expr(object)?;
            let object_reg = self.store_in_register(object, *line)?;
            let subscript = self.evaluate_expr(subscript)?;
            let subscript_reg = self.store_in_register(subscript, *line)?;
            let right = self.evaluate_expr(right)?;
            self.store_in_accumulator(right, *line)?;
            self.write2(Op::StoreSubscript, object_reg, subscript_reg, *line);
            if !matches!(subscript, ExprResult::Register(_)) {
                self.pop_register();
            }
            if !matches!(object, ExprResult::Register(_)) {
                self.pop_register();
            }
            return Ok(());
        }
        if let Expr::Variable { name, line } = left {
            if let Some(local) = self.resolve_local(name).cloned() {
                let expr_res = self.evaluate_expr_with_dest(right, Some(local.reg))?;
                if !local.mutable {
                    return Err(CompileError {
                        message: format!("Cannot modify constant {}", name),
                        line: *line,
                    });
                }
                self.store_in_specific_register(expr_res, local.reg, *line)
            } else if let Some(upval) = self.resolve_upvalue(name, *line)? {
                let res = self.evaluate_expr(right)?;
                if !self.upvalues[upval as usize].mutable {
                    return Err(CompileError {
                        message: format!("Cannot modify constant {}", name),
                        line: *line,
                    });
                }
                self.store_in_accumulator(res, *line)?;
                self.write1(Op::StoreUpvalue, upval as u32, *line);
                Ok(())
            } else {
                let global = self
                    .get_global(name)
                    .unwrap_or_else(|| self.new_global(name));
                let res = self.evaluate_expr(right)?;
                self.store_in_accumulator(res, *line)?;
                self.write1(Op::StoreGlobal, global, *line);
                Ok(())
            }
        } else {
            Err(CompileError {
                message: "Invalid target for assignment".to_string(),
                line,
            })
        }
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
    binary_op!(modulus, ModRegister, ModInt, rem, checked_rem, "mod");

    comparing_binary_op!(equal_equal,Equal,==);
    comparing_binary_op!(equal_equal_equal,StrictEqual,==);
    comparing_binary_op!(not_equal,NotEqual,!=);
    comparing_binary_op!(not_equal_equal,StrictNotEqual,!=);
    comparing_binary_op!(greater_than,GreaterThan,>);
    comparing_binary_op!(lesser_than,LesserThan,<);
    comparing_binary_op!(greater_than_or_equal,GreaterThanOrEqual,>=);
    comparing_binary_op!(lesser_than_or_equal,LesserThanOrEqual,<=);
}

fn signed_to_unsigned(i: i32) -> u32 {
    match i8::try_from(i) {
        Ok(i) => i as u8 as u32,
        Err(_) => match i16::try_from(i) {
            Ok(i) => i as u16 as u32,
            Err(_) => i as u32,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::{parser::Parser, scanner::Scanner, vm::new_vm};

    use super::*;
    #[test]
    fn test() {
        let tests: Vec<String> =
            serde_json::from_str(include_str!("../tests/compiler_tests/tests.json")).unwrap();
        for test in tests {
            let s = std::fs::read_to_string(format!("tests/compiler_tests/{}.np", test)).unwrap();
            let s = Scanner::new(&s);
            let tokens = s.scan_tokens();
            let parser = Parser::new(tokens.into_iter());
            let (stmts, errors) = parser.parse(false);
            assert!(errors.is_empty(), "{:?}", errors);
            let vm = new_vm();
            let mut globals = HashMap::default();
            let compiler = Compiler::new(&vm, &mut globals);
            let fw = compiler.exec(stmts).unwrap();
            if std::env::var("GENERATE_TESTS").is_ok() {
                std::fs::write(
                    format!("tests/compiler_tests/{}.bc", test),
                    fw.to_cxx_string().as_bytes(),
                )
                .unwrap();
            } else {
                let expected =
                    std::fs::read_to_string(format!("tests/compiler_tests/{}.bc", test)).unwrap();
                assert_eq!(expected, fw.to_cxx_string().to_str().unwrap());
            }
        }
    }

    #[test]
    fn error() {
        let tests: Vec<String> =
            serde_json::from_str(include_str!("../tests/compiler_tests/errors.json")).unwrap();
        for test in tests {
            let s = std::fs::read_to_string(format!("tests/compiler_tests/{}.np", test)).unwrap();
            let s = Scanner::new(&s);
            let tokens = s.scan_tokens();
            let parser = Parser::new(tokens.into_iter());
            let (stmts, errors) = parser.parse(false);
            assert!(errors.is_empty());
            let vm = new_vm();
            let mut globals = HashMap::default();
            let compiler = Compiler::new(&vm, &mut globals);
            let errors = compiler.exec(stmts).unwrap_err();
            if std::env::var("GENERATE_TESTS").is_ok() {
                std::fs::write(
                    format!("tests/compiler_tests/{}.json", test),
                    serde_json::to_string_pretty(&errors).unwrap(),
                )
                .unwrap();
            } else {
                let expected =
                    std::fs::read_to_string(format!("tests/compiler_tests/{}.json", test)).unwrap();
                assert_eq!(expected, serde_json::to_string_pretty(&errors).unwrap());
            }
        }
    }
}

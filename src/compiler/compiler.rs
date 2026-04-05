use crate::compiler::bytecode::{Chunk, Instruction, Opcode};
use crate::compiler::register::RegisterAllocator;
use crate::parser::ast::*;
use crate::vm::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[allow(unused)]
pub struct Compiler {
    pub chunk: Chunk,
    allocator: RegisterAllocator,
    locals: HashMap<String, u8>,
    scope_depth: usize,
    module_scope: bool,
}

impl Compiler {
    pub fn new(name: String) -> Self {
        Self {
            chunk: Chunk::new(name),
            allocator: RegisterAllocator::new(),
            locals: HashMap::new(),
            scope_depth: 0,
            module_scope: true,
        }
    }

    pub fn compile_stmts(&mut self, stmts: &[Stmt]) -> Arc<Chunk> {
        for stmt in stmts {
            self.compile_stmt(stmt);
        }
        // Implicit return null if not present
        if !self.is_last_return() {
            let r = self.allocator.alloc();
            let idx = self.chunk.add_constant(Value::Null);
            self.emit(Instruction::with_imm(Opcode::LoadConst, r.0, idx));
            self.emit(Instruction::new(Opcode::Return, r.0, 0, 0));
        }
        Arc::new(self.chunk.clone())
    }

    fn is_last_return(&self) -> bool {
        matches!(
            self.chunk.code.last().map(|i| i.opcode),
            Some(Opcode::Return)
        )
    }

    fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Assign { target, value, .. } => {
                let reg = self.compile_expr(value);
                self.store_name(target, reg);
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr);
            }
            Stmt::Return(expr) => {
                let reg = if let Some(e) = expr {
                    self.compile_expr(e)
                } else {
                    let r = self.allocator.alloc();
                    let idx = self.chunk.add_constant(Value::Null);
                    self.emit(Instruction::with_imm(Opcode::LoadConst, r.0, idx));
                    r.0
                };
                self.emit(Instruction::new(Opcode::Return, reg, 0, 0));
            }
            Stmt::If {
                cond,
                then,
                elifs,
                else_,
            } => {
                let cond_reg = self.compile_expr(cond);
                let jump_if_false_idx = self.emit_placeholder(Opcode::JumpIfFalse, cond_reg, 0);

                for s in then {
                    self.compile_stmt(s);
                }
                let end_jump_idx = self.emit_placeholder(Opcode::Jump, 0, 0);

                self.patch_jump(jump_if_false_idx);

                for (e_cond, e_body) in elifs {
                    let e_cond_reg = self.compile_expr(e_cond);
                    let e_jump_idx = self.emit_placeholder(Opcode::JumpIfFalse, e_cond_reg, 0);
                    for s in e_body {
                        self.compile_stmt(s);
                    }
                    // Re-patching the main end jump is complex in this simple version,
                    // focusing on core structure first.
                    self.patch_jump(e_jump_idx);
                }

                if let Some(e_block) = else_ {
                    for s in e_block {
                        self.compile_stmt(s);
                    }
                }

                self.patch_jump(end_jump_idx);
            }
            // More statements (While, For, etc.) would follow similar logic
            _ => { /* TODO: Implement remaining statements */ }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> u8 {
        match expr {
            Expr::Int(n) => {
                let r = self.allocator.alloc();
                let idx = self.chunk.add_constant(Value::Int(*n));
                self.emit(Instruction::with_imm(Opcode::LoadConst, r.0, idx));
                r.0
            }
            Expr::BinOp { op, left, right } => {
                let l = self.compile_expr(left);
                let r = self.compile_expr(right);
                let dst = self.allocator.alloc();
                let opcode = match op {
                    BinOp::Plus => Opcode::Add,
                    BinOp::Minus => Opcode::Sub,
                    BinOp::Star => Opcode::Mul,
                    BinOp::Slash => Opcode::Div,
                    BinOp::EqEq => Opcode::Eq,
                    BinOp::Lt => Opcode::Lt,
                    _ => Opcode::Add, // Default for now
                };
                self.emit(Instruction::new(opcode, dst.0, l, r));
                dst.0
            }
            Expr::Ident(name) => {
                if let Some(&slot) = self.locals.get(name) {
                    slot
                } else {
                    let dst = self.allocator.alloc();
                    let idx = self.chunk.add_name(name.clone());
                    self.emit(Instruction::with_imm(Opcode::LoadGlobal, dst.0, idx));
                    dst.0
                }
            }
            _ => 0, // Fallback
        }
    }

    fn store_name(&mut self, name: &str, reg: u8) {
        if self.module_scope {
            let idx = self.chunk.add_name(name.to_string());
            self.emit(Instruction::with_imm(Opcode::StoreGlobal, reg, idx));
        } else {
            self.locals.insert(name.to_string(), reg);
        }
    }

    fn emit(&mut self, instr: Instruction) {
        self.chunk.code.push(instr);
    }

    fn emit_placeholder(&mut self, opcode: Opcode, reg: u8, imm: u16) -> usize {
        self.emit(Instruction::with_imm(opcode, reg, imm));
        self.chunk.code.len() - 1
    }

    fn patch_jump(&mut self, idx: usize) {
        let target = self.chunk.code.len() as u16;
        let instr = &mut self.chunk.code[idx];
        *instr = Instruction::with_imm(instr.opcode, instr.dst, target);
    }
}

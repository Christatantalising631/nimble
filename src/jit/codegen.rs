//! JIT code-generation stub backed by Cranelift.
//!
//! Translates a `FunctionChunk`'s instruction stream into Cranelift IR.
//! Only arithmetic ops are wired up; all others are left as no-ops pending
//! full JIT implementation.

use crate::compiler::bytecode::{FunctionChunk, Instr};
use cranelift_codegen::ir::{types, InstBuilder};
pub use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, Variable};

pub struct CodeGenerator<'a> {
    builder: &'a mut FunctionBuilder<'a>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(builder: &'a mut FunctionBuilder<'a>) -> Self {
        Self { builder }
    }

    pub fn translate(&mut self, chunk: &FunctionChunk) {
        let entry_block = self.builder.create_block();
        self.builder.append_block_params_for_function_params(entry_block);
        self.builder.switch_to_block(entry_block);
        self.builder.seal_block(entry_block);

        // Declare one Cranelift variable per VM register.
        let mut regs: Vec<Variable> = Vec::new();
        for i in 0..(chunk.num_registers as usize).max(256) {
            let var = Variable::from_u32(i as u32);
            self.builder.declare_var(var, types::I64);
            regs.push(var);
        }

        for instr in &chunk.instrs {
            match instr {
                Instr::AddInt { dst, a, b } => {
                    let va  = self.builder.use_var(regs[a.0 as usize]);
                    let vb  = self.builder.use_var(regs[b.0 as usize]);
                    let res = self.builder.ins().iadd(va, vb);
                    self.builder.def_var(regs[dst.0 as usize], res);
                }
                Instr::SubInt { dst, a, b } => {
                    let va  = self.builder.use_var(regs[a.0 as usize]);
                    let vb  = self.builder.use_var(regs[b.0 as usize]);
                    let res = self.builder.ins().isub(va, vb);
                    self.builder.def_var(regs[dst.0 as usize], res);
                }
                Instr::MulInt { dst, a, b } => {
                    let va  = self.builder.use_var(regs[a.0 as usize]);
                    let vb  = self.builder.use_var(regs[b.0 as usize]);
                    let res = self.builder.ins().imul(va, vb);
                    self.builder.def_var(regs[dst.0 as usize], res);
                }
                Instr::Return { src } => {
                    let val = src
                        .map(|r| self.builder.use_var(regs[r.0 as usize]))
                        .unwrap_or_else(|| self.builder.ins().iconst(types::I64, 0));
                    self.builder.ins().return_(&[val]);
                }
                _ => { /* remaining opcodes emitted in future passes */ }
            }
        }
    }
}

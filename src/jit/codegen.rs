use crate::compiler::bytecode::{Chunk, Opcode};
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

    pub fn translate(&mut self, chunk: &Chunk) {
        let entry_block = self.builder.create_block();
        self.builder
            .append_block_params_for_function_params(entry_block);
        self.builder.switch_to_block(entry_block);
        self.builder.seal_block(entry_block);

        // Map VM registers to Cranelift variables
        let mut regs = Vec::new();
        for i in 0..256 {
            let var = Variable::from_u32(i as u32);
            self.builder.declare_var(var, types::I64);
            regs.push(var);
        }

        for &instr in &chunk.code {
            match instr.opcode {
                Opcode::Add => {
                    let va = self.builder.use_var(regs[instr.src1 as usize]);
                    let vb = self.builder.use_var(regs[instr.src2 as usize]);
                    let res = self.builder.ins().iadd(va, vb);
                    self.builder.def_var(regs[instr.dst as usize], res);
                }
                Opcode::Sub => {
                    let va = self.builder.use_var(regs[instr.src1 as usize]);
                    let vb = self.builder.use_var(regs[instr.src2 as usize]);
                    let res = self.builder.ins().isub(va, vb);
                    self.builder.def_var(regs[instr.dst as usize], res);
                }
                Opcode::Return => {
                    let val = self.builder.use_var(regs[instr.dst as usize]);
                    self.builder.ins().return_(&[val]);
                }
                // ... remaining opcodes
                _ => {}
            }
        }
    }
}

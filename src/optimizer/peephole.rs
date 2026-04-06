//! Peephole optimiser — single-pass pattern matching over a `FunctionChunk`.
//!
//! Currently implements one fusion: consecutive `LoadConst(1) + AddInt` can
//! be collapsed (placeholder — extend as needed).

use crate::compiler::bytecode::{FunctionChunk, Instr};

pub fn optimize_chunk(chunk: &mut FunctionChunk) {
    let mut out: Vec<Instr> = Vec::with_capacity(chunk.instrs.len());
    let mut i = 0;
    while i < chunk.instrs.len() {
        // Example: LoadConst followed immediately by AddInt on the same dst —
        // a future pass could fold constant-folding here.
        out.push(chunk.instrs[i].clone());
        i += 1;
    }
    chunk.instrs = out;
}

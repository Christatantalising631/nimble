//! Intermediate representation — Static Single Assignment (SSA) form.
//!
//! Used by the optimiser and JIT pipeline. The interpreter currently
//! runs `FunctionChunk` bytecode directly; this IR is the future bridge
//! to Cranelift code-generation.

use crate::vm::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IrVal(pub u32);

#[derive(Clone, Debug)]
pub enum Inst {
    Const(Value),
    Add(IrVal, IrVal),
    Sub(IrVal, IrVal),
    Mul(IrVal, IrVal),
    Phi(Vec<IrVal>),
    /// Guard: check that `value` has `expected_type`; deopt to `deopt_id` otherwise.
    Guard(IrVal, IrType, u32),
    Return(IrVal),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IrType { Int, Float, Bool, Ptr }

pub struct BasicBlock {
    pub insts:      Vec<(IrVal, Inst)>,
    pub terminator: Terminator,
}

pub enum Terminator {
    Jump(u32),
    Branch(IrVal, u32, u32),
    Return(IrVal),
}

/// SSA-form program.
pub struct SSA {
    pub blocks:   Vec<BasicBlock>,
    pub next_val: u32,
}

impl SSA {
    pub fn new() -> Self { Self { blocks: Vec::new(), next_val: 0 } }

    pub fn fresh(&mut self) -> IrVal {
        let v = IrVal(self.next_val);
        self.next_val += 1;
        v
    }
}

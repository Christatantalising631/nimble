//! Bytecode definitions for the Nimble VM.
//!
//! All instructions are variants of `Instr`. A compiled function is represented
//! as a `FunctionChunk` — a self-contained unit holding instructions, a constant
//! pool, a name table, and metadata about registers and exports.

use crate::vm::Value;

// ── Index / address newtypes ──────────────────────────────────────────────────

/// A register index in the current call frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Reg(pub u8);

/// Index into the constant pool of a `FunctionChunk`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConstIdx(pub u16);

/// Index into the name table of a `FunctionChunk`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameIdx(pub u16);

/// Absolute instruction-pointer target used by jump instructions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Addr(pub u32);

/// Argument metadata for call-like instructions.
#[derive(Clone, Debug)]
pub struct CallArgDesc {
    pub name: Option<NameIdx>,
    pub reg: Reg,
}

// ── Instruction set ───────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub enum Instr {
    // ── Data movement ─────────────────────────────────────────────────────────
    LoadConst {
        dst: Reg,
        idx: ConstIdx,
    },
    Move {
        dst: Reg,
        src: Reg,
    },

    // ── Arithmetic (polymorphic: int / float / string coercion) ───────────────
    AddInt {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    SubInt {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    MulInt {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    DivInt {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Mod {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Negate {
        dst: Reg,
        src: Reg,
    },

    // ── Typed float ops (reserved for JIT / optimiser) ────────────────────────
    AddFloat {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    SubFloat {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    MulFloat {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    DivFloat {
        dst: Reg,
        a: Reg,
        b: Reg,
    },

    // ── Comparison ────────────────────────────────────────────────────────────
    CmpEq {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    CmpNe {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    CmpLt {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    CmpGt {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    CmpLe {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    CmpGe {
        dst: Reg,
        a: Reg,
        b: Reg,
    },

    // ── Logic ─────────────────────────────────────────────────────────────────
    And {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Or {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    Not {
        dst: Reg,
        src: Reg,
    },

    // ── Control flow ──────────────────────────────────────────────────────────
    Jump {
        target: Addr,
    },
    JumpIfFalse {
        cond: Reg,
        target: Addr,
    },
    JumpIfTrue {
        cond: Reg,
        target: Addr,
    },
    Return {
        src: Option<Reg>,
    },

    // ── Calls and concurrency ─────────────────────────────────────────────────
    Call {
        dst: Option<Reg>,
        callee: Reg,
        args: Vec<CallArgDesc>,
    },
    Spawn {
        callee: Reg,
        args: Vec<CallArgDesc>,
    },

    // ── Globals ───────────────────────────────────────────────────────────────
    LoadGlobal {
        dst: Reg,
        name: NameIdx,
    },
    StoreGlobal {
        name: NameIdx,
        src: Reg,
    },

    // ── Collection constructors ───────────────────────────────────────────────
    MakeList {
        dst: Reg,
        items: Vec<Reg>,
    },
    MakeMap {
        dst: Reg,
        pairs: Vec<(Reg, Reg)>,
    },
    MakeRange {
        dst: Reg,
        start: Reg,
        end: Reg,
    },
    MakeStruct {
        dst: Reg,
        class: NameIdx,
        fields: Vec<(NameIdx, Reg)>,
    },

    // ── Aggregate / string ops ────────────────────────────────────────────────
    Len {
        dst: Reg,
        src: Reg,
    },
    Concat {
        dst: Reg,
        parts: Vec<Reg>,
    },
    Stringify {
        dst: Reg,
        src: Reg,
    },

    // ── Error handling ────────────────────────────────────────────────────────
    MakeError {
        dst: Reg,
        msg: Reg,
    },
    Propagate {
        src: Reg,
    },
    IsError {
        dst: Reg,
        src: Reg,
    },

    // ── Field / index access ──────────────────────────────────────────────────
    LoadField {
        dst: Reg,
        obj: Reg,
        field: NameIdx,
    },
    StoreField {
        obj: Reg,
        field: NameIdx,
        src: Reg,
    },
    LoadIndex {
        dst: Reg,
        obj: Reg,
        idx: Reg,
    },
    StoreIndex {
        obj: Reg,
        idx: Reg,
        src: Reg,
    },

    // ── Iterator support (for loops) ──────────────────────────────────────────
    /// Convert a list / range / string into an iterator value.
    ForIter {
        dst: Reg,
        src: Reg,
        step: Option<Reg>,
    },
    /// Advance the iterator in `iter`; place next element into `var`.
    /// Jumps to `done` when exhausted.
    IterNext {
        var: Reg,
        iter: Reg,
        done: Addr,
    },
}

// ── FunctionChunk ─────────────────────────────────────────────────────────────

/// A compiled function: its instruction stream, constant pool, name table,
/// register count, and the list of names exported to callers (for modules).
#[derive(Clone, Debug)]
pub struct FunctionChunk {
    /// Human-readable name (function name or module path).
    pub name: String,
    /// The instruction stream.
    pub instrs: Vec<Instr>,
    /// Constant pool — values embedded in code (int literals, string literals, closures …).
    pub constants: Vec<Value>,
    /// Name table — global / field names referenced by instructions.
    pub names: Vec<String>,
    /// Number of registers required by this chunk.
    pub num_registers: u16,
    /// Parameter names, used for named-argument binding.
    pub param_names: Vec<String>,
    /// Names that should be surfaced as module exports.
    pub exports: Vec<String>,
}

impl FunctionChunk {
    pub fn new(name: String) -> Self {
        Self {
            name,
            instrs: Vec::new(),
            constants: Vec::new(),
            names: Vec::new(),
            num_registers: 0,
            param_names: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Append a constant and return its index.
    pub fn add_const(&mut self, val: Value) -> ConstIdx {
        self.constants.push(val);
        ConstIdx((self.constants.len() - 1) as u16)
    }

    /// Intern a name (deduplicates) and return its index.
    pub fn add_name(&mut self, name: String) -> NameIdx {
        for (i, n) in self.names.iter().enumerate() {
            if *n == name {
                return NameIdx(i as u16);
            }
        }
        self.names.push(name);
        NameIdx((self.names.len() - 1) as u16)
    }

    /// Append one instruction and return its position.
    pub fn emit(&mut self, instr: Instr) -> usize {
        self.instrs.push(instr);
        self.instrs.len() - 1
    }
}

/// Convenience alias — every named function is a `FunctionChunk`.
pub type Function = FunctionChunk;

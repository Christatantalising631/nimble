/// Compact, cache-friendly bytecode definitions for the register-based VM.

#[derive(Clone, Copy, Debug, Default)]
pub struct SourceLocation {
    pub line: u32,
    pub column: u32,
}

/// Opcode table for the register-based VM.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    LoadConst = 0,
    LoadLocal,
    StoreLocal,
    Move,
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Lt,
    Jump,
    JumpIfFalse,
    Call,
    Return,
    // Extensions for builtins and globals needed for parity
    LoadGlobal,
    StoreGlobal,
}

/// A single 32-bit instruction with four 8-bit fields.
/// [ opcode: 8 | dst: 8 | src1: 8 | src2: 8 ]
/// Immediates (u16) are packed into src1/src2.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub dst: u8,
    pub src1: u8,
    pub src2: u8,
}

impl Instruction {
    #[inline(always)]
    pub const fn new(opcode: Opcode, dst: u8, src1: u8, src2: u8) -> Self {
        Self {
            opcode,
            dst,
            src1,
            src2,
        }
    }

    /// Pack an immediate in the src1/src2 slots.
    #[inline(always)]
    pub const fn with_imm(opcode: Opcode, dst: u8, imm: u16) -> Self {
        Self {
            opcode,
            dst,
            src1: (imm & 0xFF) as u8,
            src2: (imm >> 8) as u8,
        }
    }

    /// Rebuild the immediate stored in src1/src2.
    #[inline(always)]
    pub const fn imm_u16(&self) -> u16 {
        (self.src1 as u16) | ((self.src2 as u16) << 8)
    }
}

use crate::vm::Value;

/// A chunk of bytecode with constants and debug metadata.
#[derive(Clone, Debug)]
pub struct Chunk {
    pub name: String,
    pub code: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub names: Vec<String>,
    pub debug_info: Vec<SourceLocation>,
}

impl Chunk {
    pub fn new(name: String) -> Self {
        Self {
            name,
            code: Vec::new(),
            constants: Vec::new(),
            names: Vec::new(),
            debug_info: Vec::new(),
        }
    }

    pub fn add_constant(&mut self, val: Value) -> u16 {
        self.constants.push(val);
        (self.constants.len() - 1) as u16
    }

    pub fn add_name(&mut self, name: String) -> u16 {
        self.names.push(name);
        (self.names.len() - 1) as u16
    }
}

pub type FunctionChunk = Chunk;
pub type Reg = u8;

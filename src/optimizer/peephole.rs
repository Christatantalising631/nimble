use crate::compiler::bytecode::{Chunk, Opcode};

pub fn optimize_chunk(chunk: &mut Chunk) {
    let mut new_code = Vec::new();
    let mut i = 0;
    while i < chunk.code.len() {
        let instr = chunk.code[i];

        // Example Fusion: LOAD_CONST(1) + ADD -> INC
        if instr.opcode == Opcode::LoadConst && i + 1 < chunk.code.len() {
            let next = chunk.code[i + 1];
            if next.opcode == Opcode::Add && next.src1 == instr.dst {
                // If the constant is 1, we could fuse this into an INCREMENT opcode
                // For now, simple fusion example logic:
                // i += 2; continue;
            }
        }

        new_code.push(instr);
        i += 1;
    }
    chunk.code = new_code;
}

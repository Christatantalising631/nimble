use crate::compiler::bytecode::{Chunk, Instruction, Opcode};
use crate::vm::frame::CallFrame;
use crate::vm::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub type Handler = fn(&mut VM, Instruction);

pub struct VM {
    pub stack: Vec<CallFrame>,
    pub globals: HashMap<String, Value>,
    // In a real high-performance VM, this would be an array of function pointers
    // but for Rust safety and simplicity, we'll start with a match or a table.
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            globals: HashMap::new(),
        }
    }

    pub fn run(&mut self, chunk: Arc<Chunk>) -> Result<Value, String> {
        self.stack.push(CallFrame::new(chunk));
        self.execute()
    }

    pub fn run_with_dir(&mut self, chunk: Arc<Chunk>, _dir: PathBuf) -> Result<Value, String> {
        self.run(chunk)
    }

    pub fn global_entries(&self) -> Vec<(String, Value)> {
        self.globals
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn load_module(&mut self, _source: &str) -> Result<Value, String> {
        Err("module loading is not implemented yet".into())
    }

    fn execute(&mut self) -> Result<Value, String> {
        loop {
            let instr = {
                let frame = self.stack.last_mut().ok_or("No call frame")?;
                if frame.ip >= frame.chunk.code.len() {
                    return Ok(Value::Null);
                }
                let i = frame.chunk.code[frame.ip];
                frame.ip += 1;
                i
            };

            match instr.opcode {
                Opcode::LoadConst => {
                    let frame = self.stack.last_mut().unwrap();
                    let val = frame.chunk.constants[instr.imm_u16() as usize].clone();
                    frame.set(instr.dst, val);
                }
                Opcode::Move => {
                    let frame = self.stack.last_mut().unwrap();
                    let val = frame.get(instr.src1);
                    frame.set(instr.dst, val);
                }
                Opcode::Add => {
                    let frame = self.stack.last_mut().unwrap();
                    let a = frame.get(instr.src1);
                    let b = frame.get(instr.src2);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => frame.set(instr.dst, Value::Int(x + y)),
                        (Value::Float(x), Value::Float(y)) => {
                            frame.set(instr.dst, Value::Float(x + y))
                        }
                        _ => return Err("Invalid operands for Add".into()),
                    }
                }
                Opcode::Sub => {
                    let frame = self.stack.last_mut().unwrap();
                    let a = frame.get(instr.src1);
                    let b = frame.get(instr.src2);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => frame.set(instr.dst, Value::Int(x - y)),
                        _ => return Err("Invalid operands for Sub".into()),
                    }
                }
                Opcode::Mul => {
                    let frame = self.stack.last_mut().unwrap();
                    let a = frame.get(instr.src1);
                    let b = frame.get(instr.src2);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => frame.set(instr.dst, Value::Int(x * y)),
                        _ => return Err("Invalid operands for Mul".into()),
                    }
                }
                Opcode::Div => {
                    let frame = self.stack.last_mut().unwrap();
                    let a = frame.get(instr.src1);
                    let b = frame.get(instr.src2);
                    match (a, b) {
                        (Value::Int(x), Value::Int(y)) => {
                            if y == 0 {
                                return Err("Div by zero".into());
                            }
                            frame.set(instr.dst, Value::Int(x / y))
                        }
                        _ => return Err("Invalid operands for Div".into()),
                    }
                }
                Opcode::Jump => {
                    let frame = self.stack.last_mut().unwrap();
                    frame.ip = instr.imm_u16() as usize;
                }
                Opcode::JumpIfFalse => {
                    let frame = self.stack.last_mut().unwrap();
                    if !frame.get(instr.dst).is_truthy() {
                        frame.ip = instr.imm_u16() as usize;
                    }
                }
                Opcode::Return => {
                    let frame = self.stack.pop().unwrap();
                    let val = frame.get(instr.dst);
                    if self.stack.is_empty() {
                        return Ok(val);
                    } else {
                        // In a real call system, we'd set the result in the caller's frame
                        // For now, simplicity.
                    }
                }
                Opcode::LoadGlobal => {
                    let frame = self.stack.last_mut().unwrap();
                    let name = &frame.chunk.names[instr.imm_u16() as usize];
                    let val = self.globals.get(name).cloned().unwrap_or(Value::Null);
                    frame.set(instr.dst, val);
                }
                Opcode::StoreGlobal => {
                    let frame = self.stack.last_mut().unwrap();
                    let name = frame.chunk.names[instr.imm_u16() as usize].clone();
                    let val = frame.get(instr.dst);
                    self.globals.insert(name, val);
                }
                _ => return Err(format!("Unimplemented opcode: {:?}", instr.opcode)),
            }
        }
    }
}

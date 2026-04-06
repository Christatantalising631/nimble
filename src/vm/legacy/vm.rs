// Legacy VM - not used by the current interpreter. Kept as reference.
// The active VM is src/vm/vm.rs

use crate::compiler::bytecode::FunctionChunk;
use crate::modules::resolver::ModuleResolver;
use crate::vm::builtins;
use crate::vm::legacy::frame::CallFrame;
use crate::vm::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct VM {
    frames:       Vec<CallFrame>,
    pub globals:  HashMap<String, Value>,
    module_cache: Arc<Mutex<HashMap<String, Value>>>,
    _resolver:    ModuleResolver,
}

impl VM {
    pub fn new() -> Self {
        let mut vm = Self {
            frames:       Vec::new(),
            globals:      HashMap::new(),
            module_cache: Arc::new(Mutex::new(HashMap::new())),
            _resolver:    ModuleResolver::new(),
        };
        vm.globals.insert("out".into(), Value::NativeFunction(builtins::out));
        vm
    }

    pub fn run(&mut self, chunk: Arc<FunctionChunk>) -> Result<Value, String> {
        let dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        self.run_with_dir(chunk, dir)
    }

    pub fn run_with_dir(&mut self, chunk: Arc<FunctionChunk>, module_dir: PathBuf) -> Result<Value, String> {
        use crate::compiler::bytecode::{Instr, Reg};
        self.frames.push(CallFrame::new(chunk, module_dir, None));
        loop {
            let mut frame = match self.frames.pop() { Some(f) => f, None => return Ok(Value::Null) };
            if frame.ip >= frame.chunk.instrs.len() {
                if self.frames.is_empty() { return Ok(Value::Null); }
                continue;
            }
            let instr = frame.chunk.instrs[frame.ip].clone();
            frame.ip += 1;
            self.frames.push(frame);
            match instr {
                Instr::LoadConst { dst, idx } => {
                    let v = self.frames.last().unwrap().chunk.constants[idx.0 as usize].clone();
                    self.frames.last_mut().unwrap().set_reg(dst, v);
                }
                Instr::Return { src } => {
                    let frame = self.frames.pop().unwrap();
                    let val = src.map(|s| frame.get_reg(s)).unwrap_or(Value::Null);
                    if let Some(caller) = self.frames.last_mut() {
                        if let Some(dst) = frame.return_reg { caller.set_reg(dst, val); }
                    } else {
                        return Ok(val);
                    }
                }
                Instr::StoreGlobal { name, src } => {
                    let n = self.frames.last().unwrap().chunk.names[name.0 as usize].clone();
                    let v = self.frames.last().unwrap().get_reg(src);
                    self.globals.insert(n, v);
                }
                _ => { /* Other opcodes not needed in legacy stub */ }
            }
        }
    }

    pub fn load_module(&mut self, _source: &str) -> Result<Value, String> {
        Err("Legacy VM does not support module loading".into())
    }

    pub fn global_entries(&self) -> Vec<(String, Value)> {
        self.globals.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }
}

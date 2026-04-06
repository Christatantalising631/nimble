//! Module registry — tracks loaded modules by name.

use crate::compiler::bytecode::FunctionChunk;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Module {
    pub name: String,
    pub chunk: Arc<FunctionChunk>,
}

pub struct ModuleRegistry {
    pub loaded: HashMap<String, Arc<Module>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, module: Arc<Module>) {
        self.loaded.insert(name, module);
    }

    pub fn get(&self, name: &str) -> Option<&Arc<Module>> {
        self.loaded.get(name)
    }
}

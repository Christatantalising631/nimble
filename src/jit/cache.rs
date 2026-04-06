//! JIT compilation cache — avoids recompiling hot functions.
//!
//! Keyed by the `FunctionChunk` name + instruction count as a cheap fingerprint.

use std::collections::HashMap;

pub struct JitCache {
    /// Maps a function fingerprint to the compiled native code pointer.
    entries: HashMap<String, *const u8>,
}

impl JitCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<*const u8> {
        self.entries.get(key).copied()
    }

    pub fn insert(&mut self, key: String, ptr: *const u8) {
        self.entries.insert(key, ptr);
    }
}

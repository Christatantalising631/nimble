pub mod builtins;
pub mod frame;
pub mod gc;
pub mod value;
pub mod vm;

pub use gc::{BumpAllocator, Heap};
pub use value::Value;
pub use vm::VM;

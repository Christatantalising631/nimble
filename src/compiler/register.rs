#[derive(Debug, Clone, Copy)]
pub struct Register(pub u8);

pub struct RegisterAllocator {
    next: u8,
}

impl RegisterAllocator {
    pub fn new() -> Self {
        Self { next: 0 }
    }

    pub fn alloc(&mut self) -> Register {
        let reg = Register(self.next);
        self.next = self.next.wrapping_add(1);
        reg
    }
}

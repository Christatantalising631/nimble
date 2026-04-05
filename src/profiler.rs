#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueType {
    Int,
    Float,
    Bool,
    Null,
    Ptr,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct TypeFeedback {
    pub seen_types: [ValueType; 4],
    pub count: u32,
}

impl TypeFeedback {
    pub fn new() -> Self {
        Self {
            seen_types: [ValueType::Unknown; 4],
            count: 0,
        }
    }

    pub fn record(&mut self, ty: ValueType) {
        if !self.seen_types.contains(&ty) {
            let idx = (self.count % 4) as usize;
            self.seen_types[idx] = ty;
        }
        self.count += 1;
    }
}

pub struct FunctionProfile {
    pub invocation_count: u32,
    pub is_hot: bool,
    pub feedback: Vec<TypeFeedback>,
}

const HOT_THRESHOLD: u32 = 1000;

impl FunctionProfile {
    pub fn new(instruction_count: usize) -> Self {
        Self {
            invocation_count: 0,
            is_hot: false,
            feedback: vec![TypeFeedback::new(); instruction_count],
        }
    }

    pub fn record_invocation(&mut self) {
        self.invocation_count += 1;
        if self.invocation_count >= HOT_THRESHOLD {
            self.is_hot = true;
        }
    }
}

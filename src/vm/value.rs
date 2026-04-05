use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// A dynamic, heap-friendly value representation for the VM.
#[derive(Clone, Debug)]
pub enum Value {
    Null,
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Arc<String>),
    List(Arc<Mutex<Vec<Value>>>),
    Map(Arc<Mutex<HashMap<String, Value>>>),
    Range { start: i64, end: i64 },
    Error(Arc<String>),
}

impl Value {
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            _ => true,
        }
    }

    pub fn stringify(&self) -> String {
        match self {
            Value::Null => "null".into(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.to_string(),
            Value::List(list) => {
                let elements = list.lock().unwrap();
                let parts: Vec<String> = elements.iter().map(|v| v.stringify()).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Map(map) => {
                let map = map.lock().unwrap();
                let parts: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.stringify()))
                    .collect();
                format!("{{{}}}", parts.join(", "))
            }
            Value::Range { start, end } => format!("range({start}, {end})"),
            Value::Error(err) => format!("error({err})"),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Bool(_) => "bool",
            Value::Str(_) => "str",
            Value::List(_) => "list",
            Value::Map(_) => "map",
            Value::Range { .. } => "range",
            Value::Error(_) => "error",
        }
    }
}

/// Object Shapes (Hidden Classes)
pub struct Shape {
    pub id: u32,
    pub properties: HashMap<String, u32>,
    pub transitions: HashMap<String, Arc<Shape>>,
}

impl Shape {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            properties: HashMap::new(),
            transitions: HashMap::new(),
        }
    }
}

pub struct Object {
    pub shape: Arc<Shape>,
    pub slots: Vec<Value>,
}

pub struct InlineCache {
    pub shape_id: u32,
    pub slot: u32,
    pub state: CacheState,
}

#[derive(PartialEq)]
pub enum CacheState {
    Uninitialized,
    Monomorphic,
    Polymorphic,
    Megamorphic,
}

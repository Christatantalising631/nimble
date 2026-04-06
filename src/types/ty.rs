#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Float,
    Str,
    Bool,
    Null,
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Struct(String),
    Fn(Vec<Type>, Box<Type>),
    Error(Box<Type>),
    Union(Vec<Type>),
    Unknown,
    Any,
}

impl Type {
    pub fn is_assignable_to(&self, other: &Type) -> bool {
        if other == &Type::Any || self == other {
            return true;
        }
        match (self, other) {
            (Type::Int, Type::Float) => true, // Auto-coercion
            (Type::Union(variants), _) => variants.iter().any(|v| v.is_assignable_to(other)),
            (_, Type::Union(variants)) => variants.iter().any(|v| self.is_assignable_to(v)),
            _ => false,
        }
    }
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Int => write!(f, "int"),
            Type::Float => write!(f, "float"),
            Type::Str => write!(f, "str"),
            Type::Bool => write!(f, "bool"),
            Type::Null => write!(f, "null"),
            Type::List(inner) => write!(f, "[{}]", inner),
            Type::Map(key, value) => write!(f, "{{{}: {}}}", key, value),
            Type::Struct(name) => write!(f, "{name}"),
            Type::Fn(params, ret) => {
                write!(f, "fn(")?;
                for (idx, param) in params.iter().enumerate() {
                    if idx > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {ret}")
            }
            Type::Error(inner) => write!(f, "error<{inner}>"),
            Type::Union(types) => {
                for (idx, ty) in types.iter().enumerate() {
                    if idx > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{ty}")?;
                }
                Ok(())
            }
            Type::Unknown => write!(f, "unknown"),
            Type::Any => write!(f, "any"),
        }
    }
}

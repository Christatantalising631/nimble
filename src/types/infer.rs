use crate::parser::ast::*;
use crate::types::ty::Type;
use std::collections::HashMap;

pub struct TypeEnv {
    vars: HashMap<String, Type>,
    parent: Option<Box<TypeEnv>>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
            parent: None,
        }
    }

    pub fn extend(parent: TypeEnv) -> Self {
        Self {
            vars: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub fn get(&self, name: &str) -> Option<Type> {
        self.vars
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }

    pub fn set(&mut self, name: String, ty: Type) {
        self.vars.insert(name, ty);
    }
}

pub struct Inferencer {
    env: TypeEnv,
}

impl Inferencer {
    pub fn new() -> Self {
        let mut env = TypeEnv::new();
        Self::install_builtins(&mut env);
        Self { env }
    }

    fn install_builtins(env: &mut TypeEnv) {
        env.set("out".into(), Type::Fn(vec![Type::Any], Box::new(Type::Null)));
        env.set("in".into(), Type::Fn(vec![Type::Any], Box::new(Type::Str)));
        env.set("input".into(), Type::Fn(vec![Type::Any], Box::new(Type::Str)));
        env.set("len".into(), Type::Fn(vec![Type::Any], Box::new(Type::Int)));
        env.set("to_int".into(), Type::Fn(vec![Type::Any], Box::new(Type::Int)));
        env.set(
            "index_of".into(),
            Type::Fn(vec![Type::Any, Type::Any], Box::new(Type::Int)),
        );
        env.set(
            "error".into(),
            Type::Fn(vec![Type::Any], Box::new(Type::Error(Box::new(Type::Any)))),
        );
    }

    fn is_truthy_type(ty: &Type) -> bool {
        match ty {
            Type::Bool | Type::Any | Type::Unknown => true,
            Type::Union(variants) => variants.iter().all(Self::is_truthy_type),
            _ => false,
        }
    }

    fn list_item_type(ty: &Type) -> Type {
        match ty {
            Type::List(item) => (**item).clone(),
            Type::Map(_, value) => (**value).clone(),
            Type::Str => Type::Str,
            _ => Type::Any,
        }
    }

    pub fn infer_stmts(&mut self, stmts: &[Stmt]) -> Result<(), String> {
        for stmt in stmts {
            self.infer_stmt(stmt)?;
        }
        Ok(())
    }

    fn infer_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Assign { target, ty, value } => {
                let inferred = self.infer_expr(value)?;
                if let Some(expected) = ty {
                    if !inferred.is_assignable_to(expected) {
                        return Err(format!(
                            "Type mismatch: cannot assign {:?} to {:?}",
                            inferred, expected
                        ));
                    }
                    self.env.set(target.clone(), expected.clone());
                } else {
                    self.env.set(target.clone(), inferred);
                }
            }
            Stmt::FnDef(f) => {
                let ret_ty = f.ret_ty.clone().unwrap_or(Type::Null);
                let param_tys: Vec<Type> = f
                    .params
                    .iter()
                    .map(|p| p.ty.clone().unwrap_or(Type::Any))
                    .collect();
                self.env
                    .set(f.name.clone(), Type::Fn(param_tys, Box::new(ret_ty)));
                // We'd normally check the body here with a new scope
            }
            Stmt::ClsDef(c) => {
                self.env.set(c.name.clone(), Type::Struct(c.name.clone()));
            }
            Stmt::If {
                cond,
                then,
                elifs,
                else_,
            } => {
                let cond_ty = self.infer_expr(cond)?;
                if !Self::is_truthy_type(&cond_ty) {
                    return Err("Condition must be bool".into());
                }
                for stmt in then {
                    self.infer_stmt(stmt)?;
                }
                for (elif_cond, body) in elifs {
                    let elif_ty = self.infer_expr(elif_cond)?;
                    if !Self::is_truthy_type(&elif_ty) {
                        return Err("Condition must be bool".into());
                    }
                    for stmt in body {
                        self.infer_stmt(stmt)?;
                    }
                }
                if let Some(body) = else_ {
                    for stmt in body {
                        self.infer_stmt(stmt)?;
                    }
                }
            }
            Stmt::For { var, iter, body, .. } => {
                let iter_ty = self.infer_expr(iter)?;
                self.env.set(var.clone(), Self::list_item_type(&iter_ty));
                for stmt in body {
                    self.infer_stmt(stmt)?;
                }
            }
            Stmt::ForKV { key, val, iter, body } => {
                let iter_ty = self.infer_expr(iter)?;
                if matches!(iter_ty, Type::Map(_, _)) {
                    self.env.set(key.clone(), Type::Str);
                    self.env.set(val.clone(), Self::list_item_type(&iter_ty));
                } else {
                    self.env.set(key.clone(), Type::Any);
                    self.env.set(val.clone(), Type::Any);
                }
                for stmt in body {
                    self.infer_stmt(stmt)?;
                }
            }
            Stmt::While { cond, body } => {
                let cond_ty = self.infer_expr(cond)?;
                if !Self::is_truthy_type(&cond_ty) {
                    return Err("Condition must be bool".into());
                }
                for stmt in body {
                    self.infer_stmt(stmt)?;
                }
            }
            Stmt::FieldAssign { obj, value, .. } => {
                self.infer_expr(obj)?;
                self.infer_expr(value)?;
            }
            Stmt::IndexAssign { obj, idx, value } => {
                self.infer_expr(obj)?;
                self.infer_expr(idx)?;
                self.infer_expr(value)?;
            }
            Stmt::Expr(expr) => {
                self.infer_expr(expr)?;
            }
            Stmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.infer_expr(expr)?;
                }
            }
            Stmt::Load { alias, .. } => {
                self.env.set(alias.clone(), Type::Any);
            }
            Stmt::Export(inner) => self.infer_stmt(inner)?,
            _ => {}
        }
        Ok(())
    }

    fn infer_expr(&mut self, expr: &Expr) -> Result<Type, String> {
        match expr {
            Expr::Int(_) => Ok(Type::Int),
            Expr::Float(_) => Ok(Type::Float),
            Expr::Str(_) => Ok(Type::Str),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Null => Ok(Type::Null),
            Expr::Ident(name) => self
                .env
                .get(name)
                .ok_or(format!("Undefined variable '{}'", name)),
            Expr::BinOp { op, left, right } => {
                let l = self.infer_expr(left)?;
                let r = self.infer_expr(right)?;
                match op {
                    BinOp::Plus | BinOp::Minus | BinOp::Star | BinOp::Slash | BinOp::Percent => {
                        if matches!(l, Type::Any | Type::Unknown) || matches!(r, Type::Any | Type::Unknown) {
                            Ok(Type::Any)
                        } else if l == Type::Int && r == Type::Int {
                            Ok(Type::Int)
                        } else if (l == Type::Int || l == Type::Float)
                            && (r == Type::Int || r == Type::Float)
                        {
                            Ok(Type::Float)
                        } else if l == Type::Str && r == Type::Str && matches!(op, BinOp::Plus) {
                            Ok(Type::Str)
                        } else {
                            Err("Invalid operands for arithmetic".into())
                        }
                    }
                    BinOp::EqEq
                    | BinOp::BangEq
                    | BinOp::Lt
                    | BinOp::Gt
                    | BinOp::LtEq
                    | BinOp::GtEq => Ok(Type::Bool),
                    BinOp::And | BinOp::Or => Ok(Type::Any),
                    BinOp::DotDot => Ok(Type::Any),
                }
            }
            Expr::UnaryOp { op, expr } => {
                let inner = self.infer_expr(expr)?;
                match op {
                    UnaryOp::Minus => match inner {
                        Type::Int | Type::Float | Type::Any | Type::Unknown => Ok(inner),
                        _ => Err("Unary '-' requires a number".into()),
                    },
                    UnaryOp::Not => Ok(Type::Bool),
                }
            }
            Expr::List(items) => {
                if items.is_empty() {
                    return Ok(Type::List(Box::new(Type::Any)));
                }
                let first = self.infer_expr(&items[0])?;
                Ok(Type::List(Box::new(first)))
            }
            Expr::Map(pairs) => {
                if pairs.is_empty() {
                    return Ok(Type::Map(Box::new(Type::Any), Box::new(Type::Any)));
                }
                let (first_key, first_value) = &pairs[0];
                let key_ty = self.infer_expr(first_key)?;
                let value_ty = self.infer_expr(first_value)?;
                Ok(Type::Map(Box::new(key_ty), Box::new(value_ty)))
            }
            Expr::Call { callee, args } => {
                let c_ty = self.infer_expr(callee)?;
                if let Type::Fn(param_tys, ret_ty) = c_ty {
                    if param_tys.len() != args.len() && !(param_tys.len() == 1 && param_tys[0] == Type::Any) {
                        return Err("Argument count mismatch".into());
                    }
                    for arg in args {
                        self.infer_expr(&arg.expr)?;
                    }
                    Ok(*ret_ty)
                } else if let Type::Struct(name) = c_ty {
                    for arg in args {
                        self.infer_expr(&arg.expr)?;
                    }
                    Ok(Type::Struct(name)) // Constructor
                } else if matches!(c_ty, Type::Any | Type::Unknown) {
                    for arg in args {
                        self.infer_expr(&arg.expr)?;
                    }
                    Ok(Type::Any)
                } else {
                    Err("Expression is not callable".into())
                }
            }
            Expr::Field { .. } => Ok(Type::Any),
            Expr::Index { obj, .. } => Ok(Self::list_item_type(&self.infer_expr(obj)?)),
            Expr::Lambda { params, ret_ty, .. } => Ok(Type::Fn(
                params
                    .iter()
                    .map(|param| param.ty.clone().unwrap_or(Type::Any))
                    .collect(),
                Box::new(ret_ty.clone().unwrap_or(Type::Any)),
            )),
            Expr::Interp(_) => Ok(Type::Str),
            Expr::Ternary { then, else_, .. } => {
                let then_ty = self.infer_expr(then)?;
                let else_ty = self.infer_expr(else_)?;
                if then_ty == else_ty {
                    Ok(then_ty)
                } else {
                    Ok(Type::Union(vec![then_ty, else_ty]))
                }
            }
            Expr::Propagate(inner) => Ok(self.infer_expr(inner)?),
            Expr::Spawn(_) => Ok(Type::Null),
        }
    }
}

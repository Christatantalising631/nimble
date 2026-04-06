use crate::error::SemanticError;
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
        env.set(
            "out".into(),
            Type::Fn(vec![Type::Any], Box::new(Type::Null)),
        );
        env.set("in".into(), Type::Fn(vec![Type::Any], Box::new(Type::Str)));
        env.set(
            "input".into(),
            Type::Fn(vec![Type::Any], Box::new(Type::Str)),
        );
        env.set("len".into(), Type::Fn(vec![Type::Any], Box::new(Type::Int)));
        env.set(
            "to_int".into(),
            Type::Fn(vec![Type::Any], Box::new(Type::Int)),
        );
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

    fn type_mismatch(
        expected: impl Into<String>,
        found: impl Into<String>,
        expected_span: crate::lexer::Span,
        found_span: crate::lexer::Span,
        help: Option<String>,
    ) -> SemanticError {
        SemanticError::TypeMismatch {
            expected: expected.into(),
            found: found.into(),
            expected_span,
            found_span,
            help,
        }
    }

    fn generic_error(
        message: impl Into<String>,
        span: crate::lexer::Span,
        label: impl Into<String>,
        help: Option<String>,
    ) -> SemanticError {
        SemanticError::Generic {
            message: message.into(),
            span,
            label: label.into(),
            help,
        }
    }

    pub fn infer_stmts(&mut self, stmts: &[Stmt]) -> Result<(), SemanticError> {
        for stmt in stmts {
            self.infer_stmt(stmt)?;
        }
        Ok(())
    }

    fn infer_stmt(&mut self, stmt: &Stmt) -> Result<(), SemanticError> {
        match &stmt.kind {
            StmtKind::Assign {
                target,
                target_span,
                ty,
                value,
                ..
            } => {
                let inferred = self.infer_expr(value)?;
                if let Some(expected) = ty {
                    if !inferred.is_assignable_to(expected) {
                        return Err(Self::type_mismatch(
                            expected.to_string(),
                            inferred.to_string(),
                            *target_span,
                            value.span,
                            Some(
                                "change the annotation or make the assigned value match it".into(),
                            ),
                        ));
                    }
                    self.env.set(target.clone(), expected.clone());
                } else {
                    self.env.set(target.clone(), inferred);
                }
            }
            StmtKind::FnDef(f) => {
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
            StmtKind::ClsDef(c) => {
                self.env.set(c.name.clone(), Type::Struct(c.name.clone()));
            }
            StmtKind::If {
                cond,
                then,
                elifs,
                else_,
            } => {
                let cond_ty = self.infer_expr(cond)?;
                if !Self::is_truthy_type(&cond_ty) {
                    return Err(Self::type_mismatch(
                        "bool",
                        cond_ty.to_string(),
                        cond.span,
                        cond.span,
                        Some("use a boolean expression in this condition".into()),
                    ));
                }
                for stmt in then {
                    self.infer_stmt(stmt)?;
                }
                for (elif_cond, body) in elifs {
                    let elif_ty = self.infer_expr(elif_cond)?;
                    if !Self::is_truthy_type(&elif_ty) {
                        return Err(Self::type_mismatch(
                            "bool",
                            elif_ty.to_string(),
                            elif_cond.span,
                            elif_cond.span,
                            Some("use a boolean expression in this condition".into()),
                        ));
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
            StmtKind::For {
                var, iter, body, ..
            } => {
                let iter_ty = self.infer_expr(iter)?;
                self.env.set(var.clone(), Self::list_item_type(&iter_ty));
                for stmt in body {
                    self.infer_stmt(stmt)?;
                }
            }
            StmtKind::ForKV {
                key,
                val,
                iter,
                body,
                ..
            } => {
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
            StmtKind::While { cond, body } => {
                let cond_ty = self.infer_expr(cond)?;
                if !Self::is_truthy_type(&cond_ty) {
                    return Err(Self::type_mismatch(
                        "bool",
                        cond_ty.to_string(),
                        cond.span,
                        cond.span,
                        Some("use a boolean expression in this condition".into()),
                    ));
                }
                for stmt in body {
                    self.infer_stmt(stmt)?;
                }
            }
            StmtKind::FieldAssign { obj, value, .. } => {
                self.infer_expr(obj)?;
                self.infer_expr(value)?;
            }
            StmtKind::IndexAssign { obj, idx, value } => {
                self.infer_expr(obj)?;
                self.infer_expr(idx)?;
                self.infer_expr(value)?;
            }
            StmtKind::Expr(expr) => {
                self.infer_expr(expr)?;
            }
            StmtKind::Return(expr) => {
                if let Some(expr) = expr {
                    self.infer_expr(expr)?;
                }
            }
            StmtKind::Load { alias, .. } => {
                self.env.set(alias.clone(), Type::Any);
            }
            StmtKind::Export(inner) => self.infer_stmt(inner)?,
            _ => {}
        }
        Ok(())
    }

    fn infer_expr(&mut self, expr: &Expr) -> Result<Type, SemanticError> {
        match &expr.kind {
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::Float(_) => Ok(Type::Float),
            ExprKind::Str(_) => Ok(Type::Str),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::Null => Ok(Type::Null),
            ExprKind::Ident(name) => {
                self.env
                    .get(name)
                    .ok_or_else(|| SemanticError::UndefinedVariable {
                        name: name.clone(),
                        span: expr.span,
                    })
            }
            ExprKind::BinOp { op, left, right } => {
                let l = self.infer_expr(left)?;
                let r = self.infer_expr(right)?;
                match op {
                    BinOp::Plus | BinOp::Minus | BinOp::Star | BinOp::Slash | BinOp::Percent => {
                        if matches!(l, Type::Any | Type::Unknown)
                            || matches!(r, Type::Any | Type::Unknown)
                        {
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
                            Err(Self::generic_error(
                                "invalid operands for arithmetic",
                                expr.span,
                                format!("left operand is `{l}` and right operand is `{r}`"),
                                Some(
                                    "use numeric operands, or `str + str` for concatenation".into(),
                                ),
                            ))
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
            ExprKind::UnaryOp { op, expr } => {
                let inner = self.infer_expr(expr)?;
                match op {
                    UnaryOp::Minus => match inner {
                        Type::Int | Type::Float | Type::Any | Type::Unknown => Ok(inner),
                        _ => Err(Self::type_mismatch(
                            "int or float",
                            inner.to_string(),
                            expr.span,
                            expr.span,
                            Some("apply unary `-` only to numeric values".into()),
                        )),
                    },
                    UnaryOp::Not => Ok(Type::Bool),
                }
            }
            ExprKind::List(items) => {
                if items.is_empty() {
                    return Ok(Type::List(Box::new(Type::Any)));
                }
                let first = self.infer_expr(&items[0])?;
                Ok(Type::List(Box::new(first)))
            }
            ExprKind::Map(pairs) => {
                if pairs.is_empty() {
                    return Ok(Type::Map(Box::new(Type::Any), Box::new(Type::Any)));
                }
                let (first_key, first_value) = &pairs[0];
                let key_ty = self.infer_expr(first_key)?;
                let value_ty = self.infer_expr(first_value)?;
                Ok(Type::Map(Box::new(key_ty), Box::new(value_ty)))
            }
            ExprKind::Call { callee, args } => {
                let c_ty = self.infer_expr(callee)?;
                if let Type::Fn(param_tys, ret_ty) = c_ty {
                    if param_tys.len() != args.len()
                        && !(param_tys.len() == 1 && param_tys[0] == Type::Any)
                    {
                        return Err(Self::generic_error(
                            format!(
                                "argument count mismatch: expected {}, found {}",
                                param_tys.len(),
                                args.len()
                            ),
                            expr.span,
                            "call arity does not match the function signature",
                            Some("pass the expected number of arguments".into()),
                        ));
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
                    Err(Self::generic_error(
                        format!("expression of type `{c_ty}` is not callable"),
                        callee.span,
                        "this expression does not evaluate to a function or constructor",
                        Some("call a function value, or remove the trailing `(...)`".into()),
                    ))
                }
            }
            ExprKind::Field { .. } => Ok(Type::Any),
            ExprKind::Index { obj, .. } => Ok(Self::list_item_type(&self.infer_expr(obj)?)),
            ExprKind::Lambda { params, ret_ty, .. } => Ok(Type::Fn(
                params
                    .iter()
                    .map(|param| param.ty.clone().unwrap_or(Type::Any))
                    .collect(),
                Box::new(ret_ty.clone().unwrap_or(Type::Any)),
            )),
            ExprKind::Interp(_) => Ok(Type::Str),
            ExprKind::Ternary { then, else_, .. } => {
                let then_ty = self.infer_expr(then)?;
                let else_ty = self.infer_expr(else_)?;
                if then_ty == else_ty {
                    Ok(then_ty)
                } else {
                    Ok(Type::Union(vec![then_ty, else_ty]))
                }
            }
            ExprKind::Propagate(inner) => Ok(self.infer_expr(inner)?),
            ExprKind::Spawn(_) => Ok(Type::Null),
        }
    }
}

use crate::compiler::bytecode::{Addr, CallArgDesc, FunctionChunk, Instr, Reg};
use crate::parser::ast::*;
use crate::vm::Value;
use std::collections::HashMap;
use std::sync::Arc;

// ── Loop context ──────────────────────────────────────────────────────────────

struct LoopCtx {
    break_patches:    Vec<usize>,
    continue_patches: Vec<usize>,
    check_pc:         usize,
    is_for:           bool,
}

// ── Compiler ──────────────────────────────────────────────────────────────────

pub struct Compiler {
    pub chunk:       FunctionChunk,
    next_reg:        u8,
    locals:          HashMap<String, Reg>,
    is_module:       bool,
    loops:           Vec<LoopCtx>,
}

impl Compiler {
    pub fn new(name: String) -> Self {
        Self { chunk: FunctionChunk::new(name), next_reg: 0, locals: HashMap::new(), is_module: true, loops: Vec::new() }
    }

    fn new_fn(name: &str) -> Self {
        Self { chunk: FunctionChunk::new(name.into()), next_reg: 0, locals: HashMap::new(), is_module: false, loops: Vec::new() }
    }

    fn alloc(&mut self) -> Reg {
        let r = Reg(self.next_reg);
        self.next_reg += 1;
        if self.next_reg as u16 > self.chunk.num_registers {
            self.chunk.num_registers = self.next_reg as u16;
        }
        r
    }

    fn emit(&mut self, instr: Instr) -> usize { self.chunk.emit(instr) }

    fn emit_jump(&mut self) -> usize {
        self.chunk.emit(Instr::Jump { target: Addr(0) })
    }
    fn emit_jump_if_false(&mut self, cond: Reg) -> usize {
        self.chunk.emit(Instr::JumpIfFalse { cond, target: Addr(0) })
    }
    fn emit_jump_if_true(&mut self, cond: Reg) -> usize {
        self.chunk.emit(Instr::JumpIfTrue { cond, target: Addr(0) })
    }
    fn patch_jump(&mut self, idx: usize) {
        let target = Addr(self.chunk.instrs.len() as u32);
        match &mut self.chunk.instrs[idx] {
            Instr::Jump { target: t }           => *t = target,
            Instr::JumpIfFalse { target: t, .. }=> *t = target,
            Instr::JumpIfTrue  { target: t, .. }=> *t = target,
            Instr::IterNext    { done: t, .. }  => *t = target,
            _ => {}
        }
    }

    fn pc(&self) -> usize { self.chunk.instrs.len() }

    fn is_last_return(&self) -> bool {
        matches!(self.chunk.instrs.last(), Some(Instr::Return { .. }))
    }

    fn compile_call_args(&mut self, args: &[CallArg]) -> Vec<CallArgDesc> {
        args.iter()
            .map(|arg| {
                let reg = self.compile_expr(&arg.expr);
                let name = arg.name.as_ref().map(|name| self.chunk.add_name(name.clone()));
                CallArgDesc { name, reg }
            })
            .collect()
    }

    // ── Public compile entry ──────────────────────────────────────────────────

    pub fn compile_stmts(&mut self, stmts: &[Stmt]) -> Arc<FunctionChunk> {
        for s in stmts { self.compile_stmt(s); }
        if !self.is_last_return() {
            self.emit(Instr::Return { src: None });
        }
        Arc::new(self.chunk.clone())
    }

    // ── Statements ────────────────────────────────────────────────────────────

    fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Export(inner) => {
                let export_name = match inner.as_ref() {
                    Stmt::FnDef(f)          => Some(f.name.clone()),
                    Stmt::Assign{target,..} => Some(target.clone()),
                    Stmt::ClsDef(c)         => Some(c.name.clone()),
                    _ => None,
                };
                self.compile_stmt(inner);
                if let Some(n) = export_name { self.chunk.exports.push(n); }
            }
            Stmt::FnDef(fndef) => self.compile_fn_def(fndef),
            Stmt::ClsDef(cls)  => self.compile_cls_def(cls),

            Stmt::Assign { target, value, .. } => {
                let r = self.compile_expr(value);
                self.store_name(target, r);
            }
            Stmt::FieldAssign { obj, field, value } => {
                let r_obj = self.compile_expr(obj);
                let r_val = self.compile_expr(value);
                let fi = self.chunk.add_name(field.clone());
                self.emit(Instr::StoreField { obj: r_obj, field: fi, src: r_val });
            }
            Stmt::IndexAssign { obj, idx, value } => {
                let r_obj = self.compile_expr(obj);
                let r_idx = self.compile_expr(idx);
                let r_val = self.compile_expr(value);
                self.emit(Instr::StoreIndex { obj: r_obj, idx: r_idx, src: r_val });
            }
            Stmt::Return(opt_expr) => {
                let r = if let Some(e) = opt_expr {
                    let r = self.compile_expr(e);
                    Some(r)
                } else { None };
                self.emit(Instr::Return { src: r });
            }
            Stmt::Expr(e) => { self.compile_expr(e); }

            Stmt::If { cond, then, elifs, else_ } => {
                let r_cond = self.compile_expr(cond);
                let jf = self.emit_jump_if_false(r_cond);
                for s in then { self.compile_stmt(s); }
                let mut end_jumps = Vec::new();
                end_jumps.push(self.emit_jump());
                self.patch_jump(jf);

                for (ec, eb) in elifs {
                    let r_ec = self.compile_expr(ec);
                    let ejf = self.emit_jump_if_false(r_ec);
                    for s in eb { self.compile_stmt(s); }
                    end_jumps.push(self.emit_jump());
                    self.patch_jump(ejf);
                }

                if let Some(e_block) = else_ {
                    for s in e_block { self.compile_stmt(s); }
                }
                for j in end_jumps { self.patch_jump(j); }
            }

            Stmt::While { cond, body } => {
                let check_pc = self.pc();
                let r_cond = self.compile_expr(cond);
                let jf = self.emit_jump_if_false(r_cond);
                self.loops.push(LoopCtx { break_patches: Vec::new(), continue_patches: Vec::new(), check_pc, is_for: false });
                for s in body { self.compile_stmt(s); }
                let ctx = self.loops.pop().unwrap();
                // patch continues -> check_pc
                for p in &ctx.continue_patches { self.patch_to(p, check_pc); }
                self.emit(Instr::Jump { target: Addr(check_pc as u32) });
                self.patch_jump(jf);
                let loop_end = self.pc();
                for p in ctx.break_patches { self.patch_to(&p, loop_end); }
            }

            Stmt::For { var, iter, step, body } => {
                let r_iter_val = self.compile_expr(iter);
                let r_step     = step.as_ref().map(|expr| self.compile_expr(expr));
                let r_iter     = self.alloc();
                let r_var      = self.alloc();
                self.emit(Instr::ForIter { dst: r_iter, src: r_iter_val, step: r_step });
                let check_pc = self.pc();
                let done_patch = self.chunk.emit(Instr::IterNext { var: r_var, iter: r_iter, done: Addr(0) });

                // bind loop variable
                let prev = self.locals.remove(var);
                self.locals.insert(var.clone(), r_var);

                self.loops.push(LoopCtx { break_patches: Vec::new(), continue_patches: Vec::new(), check_pc, is_for: true });
                for s in body { self.compile_stmt(s); }
                let ctx = self.loops.pop().unwrap();
                let increment_pc = self.pc();
                for p in &ctx.continue_patches { self.patch_to(p, increment_pc); }
                self.emit(Instr::Jump { target: Addr(check_pc as u32) });
                // patch done
                let loop_end = self.pc();
                match &mut self.chunk.instrs[done_patch] {
                    Instr::IterNext { done, .. } => *done = Addr(loop_end as u32),
                    _ => {}
                }
                for p in ctx.break_patches { self.patch_to(&p, loop_end); }

                // restore previous local binding (if any)
                self.locals.remove(var);
                if let Some(prev_reg) = prev { self.locals.insert(var.clone(), prev_reg); }
            }

            Stmt::ForKV { key, val, iter, body } => {
                let r_map  = self.compile_expr(iter);
                // get keys list via builtin
                let r_loader = self.alloc();
                let nk = self.chunk.add_name("__map_keys".into());
                self.emit(Instr::LoadGlobal { dst: r_loader, name: nk });
                let r_keys = self.alloc();
                self.emit(Instr::Call {
                    dst: Some(r_keys),
                    callee: r_loader,
                    args: vec![CallArgDesc { name: None, reg: r_map }],
                });
                // iterate keys
                let r_iter = self.alloc();
                let r_key_var = self.alloc();
                let r_val_var = self.alloc();
                self.emit(Instr::ForIter { dst: r_iter, src: r_keys, step: None });
                let check_pc = self.pc();
                let done_patch = self.chunk.emit(Instr::IterNext { var: r_key_var, iter: r_iter, done: Addr(0) });
                // load value from map
                self.emit(Instr::LoadIndex { dst: r_val_var, obj: r_map, idx: r_key_var });

                let prev_k = self.locals.remove(key);
                let prev_v = self.locals.remove(val);
                self.locals.insert(key.clone(), r_key_var);
                self.locals.insert(val.clone(), r_val_var);

                self.loops.push(LoopCtx { break_patches: Vec::new(), continue_patches: Vec::new(), check_pc, is_for: true });
                for s in body { self.compile_stmt(s); }
                let ctx = self.loops.pop().unwrap();
                let inc_pc = self.pc();
                for p in &ctx.continue_patches { self.patch_to(p, inc_pc); }
                self.emit(Instr::Jump { target: Addr(check_pc as u32) });
                let loop_end = self.pc();
                match &mut self.chunk.instrs[done_patch] {
                    Instr::IterNext { done, .. } => *done = Addr(loop_end as u32),
                    _ => {}
                }
                for p in ctx.break_patches { self.patch_to(&p, loop_end); }
                self.locals.remove(key);
                self.locals.remove(val);
                if let Some(r) = prev_k { self.locals.insert(key.clone(), r); }
                if let Some(r) = prev_v { self.locals.insert(val.clone(), r); }
            }

            Stmt::Break => {
                let p = self.emit_jump();
                if let Some(ctx) = self.loops.last_mut() { ctx.break_patches.push(p); }
            }
            Stmt::Continue => {
                if let Some((is_for, check_pc)) = self.loops.last().map(|ctx| (ctx.is_for, ctx.check_pc)) {
                    if is_for {
                        let p = self.emit_jump();
                        if let Some(ctx) = self.loops.last_mut() {
                            ctx.continue_patches.push(p);
                        }
                    } else {
                        let target = Addr(check_pc as u32);
                        self.emit(Instr::Jump { target });
                    }
                }
            }

            Stmt::Load { alias, source } => {
                // __load_module("source") -> StoreGlobal(alias)
                let r_fn  = self.alloc();
                let ni    = self.chunk.add_name("__load_module".into());
                self.emit(Instr::LoadGlobal { dst: r_fn, name: ni });
                let r_src = self.alloc();
                let ci    = self.chunk.add_const(Value::Str(Arc::new(source.clone())));
                self.emit(Instr::LoadConst { dst: r_src, idx: ci });
                let r_mod = self.alloc();
                self.emit(Instr::Call {
                    dst: Some(r_mod),
                    callee: r_fn,
                    args: vec![CallArgDesc { name: None, reg: r_src }],
                });
                self.store_name(alias, r_mod);
            }
        }
    }

    // ── Function & class compilation ──────────────────────────────────────────

    fn compile_fn_def(&mut self, fndef: &FnDef) {
        let mut sub = Compiler::new_fn(&fndef.name);
        sub.chunk.param_names = fndef.params.iter().map(|p| p.name.clone()).collect();
        for (i, p) in fndef.params.iter().enumerate() {
            sub.locals.insert(p.name.clone(), Reg(i as u8));
            sub.next_reg = (i + 1) as u8;
            sub.chunk.num_registers = sub.next_reg as u16;
        }
        for s in &fndef.body { sub.compile_stmt(s); }
        if !sub.is_last_return() { sub.emit(Instr::Return { src: None }); }
        sub.chunk.num_registers = sub.next_reg as u16;

        let fn_val = Value::Function(Arc::new(sub.chunk));
        let ci = self.chunk.add_const(fn_val);
        let r  = self.alloc();
        self.emit(Instr::LoadConst { dst: r, idx: ci });
        self.store_name(&fndef.name, r);
    }

    fn compile_cls_def(&mut self, cls: &ClsDef) {
        let fields: Vec<String> = cls.fields.iter().map(|p| p.name.clone()).collect();
        let cls_val = Value::Class { name: Arc::new(cls.name.clone()), fields };
        let ci = self.chunk.add_const(cls_val);
        let r  = self.alloc();
        self.emit(Instr::LoadConst { dst: r, idx: ci });
        self.store_name(&cls.name, r);
    }

    // ── Name storage / load ───────────────────────────────────────────────────

    fn store_name(&mut self, name: &str, reg: Reg) {
        if self.is_module {
            let ni = self.chunk.add_name(name.to_string());
            self.emit(Instr::StoreGlobal { name: ni, src: reg });
        } else if let Some(existing) = self.locals.get(name).copied() {
            self.emit(Instr::Move { dst: existing, src: reg });
        } else {
            self.locals.insert(name.to_string(), reg);
        }
    }

    fn load_name(&mut self, name: &str) -> Reg {
        if let Some(&r) = self.locals.get(name) { return r; }
        let dst = self.alloc();
        let ni  = self.chunk.add_name(name.to_string());
        self.emit(Instr::LoadGlobal { dst, name: ni });
        dst
    }

    // ── Expressions ───────────────────────────────────────────────────────────

    fn compile_expr(&mut self, expr: &Expr) -> Reg {
        match expr {
            Expr::Null => {
                let r = self.alloc();
                let ci = self.chunk.add_const(Value::Null);
                self.emit(Instr::LoadConst { dst: r, idx: ci });
                r
            }
            Expr::Bool(b) => {
                let r = self.alloc();
                let ci = self.chunk.add_const(Value::Bool(*b));
                self.emit(Instr::LoadConst { dst: r, idx: ci });
                r
            }
            Expr::Int(n) => {
                let r = self.alloc();
                let ci = self.chunk.add_const(Value::Int(*n));
                self.emit(Instr::LoadConst { dst: r, idx: ci });
                r
            }
            Expr::Float(f) => {
                let r = self.alloc();
                let ci = self.chunk.add_const(Value::Float(*f));
                self.emit(Instr::LoadConst { dst: r, idx: ci });
                r
            }
            Expr::Str(s) => {
                let r = self.alloc();
                let ci = self.chunk.add_const(Value::Str(Arc::new(s.clone())));
                self.emit(Instr::LoadConst { dst: r, idx: ci });
                r
            }
            Expr::Ident(name) => self.load_name(name),

            Expr::BinOp { op, left, right } => {
                // Short-circuit for And / Or
                if matches!(op, BinOp::And) {
                    let r_dst = self.alloc();
                    let r_l   = self.compile_expr(left);
                    self.emit(Instr::Move { dst: r_dst, src: r_l });
                    let jf = self.emit_jump_if_false(r_dst);
                    let r_r = self.compile_expr(right);
                    self.emit(Instr::Move { dst: r_dst, src: r_r });
                    self.patch_jump(jf);
                    return r_dst;
                }
                if matches!(op, BinOp::Or) {
                    let r_dst = self.alloc();
                    let r_l   = self.compile_expr(left);
                    self.emit(Instr::Move { dst: r_dst, src: r_l });
                    let jt = self.emit_jump_if_true(r_dst);
                    let r_r = self.compile_expr(right);
                    self.emit(Instr::Move { dst: r_dst, src: r_r });
                    self.patch_jump(jt);
                    return r_dst;
                }
                // Range (..)
                if matches!(op, BinOp::DotDot) {
                    let r_s = self.compile_expr(left);
                    let r_e = self.compile_expr(right);
                    let dst = self.alloc();
                    self.emit(Instr::MakeRange { dst, start: r_s, end: r_e });
                    return dst;
                }
                let r_l = self.compile_expr(left);
                let r_r = self.compile_expr(right);
                let dst = self.alloc();
                let instr = match op {
                    BinOp::Plus    => Instr::AddInt { dst, a: r_l, b: r_r },
                    BinOp::Minus   => Instr::SubInt { dst, a: r_l, b: r_r },
                    BinOp::Star    => Instr::MulInt { dst, a: r_l, b: r_r },
                    BinOp::Slash   => Instr::DivInt { dst, a: r_l, b: r_r },
                    BinOp::Percent => Instr::Mod    { dst, a: r_l, b: r_r },
                    BinOp::EqEq   => Instr::CmpEq  { dst, a: r_l, b: r_r },
                    BinOp::BangEq  => Instr::CmpNe  { dst, a: r_l, b: r_r },
                    BinOp::Lt      => Instr::CmpLt  { dst, a: r_l, b: r_r },
                    BinOp::Gt      => Instr::CmpGt  { dst, a: r_l, b: r_r },
                    BinOp::LtEq    => Instr::CmpLe  { dst, a: r_l, b: r_r },
                    BinOp::GtEq    => Instr::CmpGe  { dst, a: r_l, b: r_r },
                    BinOp::And | BinOp::Or | BinOp::DotDot => unreachable!(),
                };
                self.emit(instr);
                dst
            }

            Expr::UnaryOp { op, expr } => {
                let r = self.compile_expr(expr);
                let dst = self.alloc();
                match op {
                    UnaryOp::Minus => { self.emit(Instr::Negate { dst, src: r }); }
                    UnaryOp::Not   => { self.emit(Instr::Not    { dst, src: r }); }
                }
                dst
            }

            Expr::Call { callee, args } => {
                let r_callee = self.compile_expr(callee);
                let arg_regs = self.compile_call_args(args);
                let dst = self.alloc();
                self.emit(Instr::Call { dst: Some(dst), callee: r_callee, args: arg_regs });
                dst
            }

            Expr::Index { obj, idx } => {
                let r_obj = self.compile_expr(obj);
                let r_idx = self.compile_expr(idx);
                let dst   = self.alloc();
                self.emit(Instr::LoadIndex { dst, obj: r_obj, idx: r_idx });
                dst
            }

            Expr::Field { obj, field } => {
                let r_obj = self.compile_expr(obj);
                let fi    = self.chunk.add_name(field.clone());
                let dst   = self.alloc();
                self.emit(Instr::LoadField { dst, obj: r_obj, field: fi });
                dst
            }

            Expr::List(items) => {
                let regs: Vec<Reg> = items.iter().map(|e| self.compile_expr(e)).collect();
                let dst = self.alloc();
                self.emit(Instr::MakeList { dst, items: regs });
                dst
            }

            Expr::Map(pairs) => {
                let pair_regs: Vec<(Reg, Reg)> = pairs.iter().map(|(k, v)| {
                    (self.compile_expr(k), self.compile_expr(v))
                }).collect();
                let dst = self.alloc();
                self.emit(Instr::MakeMap { dst, pairs: pair_regs });
                dst
            }

            Expr::Lambda { params, body, .. } => {
                let name = "<lambda>";
                let mut sub = Compiler::new_fn(name);
                sub.chunk.param_names = params.iter().map(|p| p.name.clone()).collect();
                for (i, p) in params.iter().enumerate() {
                    sub.locals.insert(p.name.clone(), Reg(i as u8));
                    sub.next_reg = (i + 1) as u8;
                    sub.chunk.num_registers = sub.next_reg as u16;
                }
                for s in body { sub.compile_stmt(s); }
                if !sub.is_last_return() { sub.emit(Instr::Return { src: None }); }
                sub.chunk.num_registers = sub.next_reg as u16;
                let fn_val = Value::Function(Arc::new(sub.chunk));
                let ci = self.chunk.add_const(fn_val);
                let r  = self.alloc();
                self.emit(Instr::LoadConst { dst: r, idx: ci });
                r
            }

            Expr::Interp(parts) => {
                let mut regs: Vec<Reg> = Vec::new();
                for part in parts {
                    match part {
                        InterpPart::Str(s) => {
                            if !s.is_empty() {
                                let ci = self.chunk.add_const(Value::Str(Arc::new(s.clone())));
                                let r = self.alloc();
                                self.emit(Instr::LoadConst { dst: r, idx: ci });
                                regs.push(r);
                            }
                        }
                        InterpPart::Expr(e) => {
                            let r = self.compile_expr(e);
                            let s = self.alloc();
                            self.emit(Instr::Stringify { dst: s, src: r });
                            regs.push(s);
                        }
                    }
                }
                if regs.len() == 1 { return regs[0]; }
                let dst = self.alloc();
                self.emit(Instr::Concat { dst, parts: regs });
                dst
            }

            Expr::Ternary { cond, then, else_ } => {
                let r_dst  = self.alloc();
                let r_cond = self.compile_expr(cond);
                let jf     = self.emit_jump_if_false(r_cond);
                let r_then = self.compile_expr(then);
                self.emit(Instr::Move { dst: r_dst, src: r_then });
                let jmp    = self.emit_jump();
                self.patch_jump(jf);
                let r_else = self.compile_expr(else_);
                self.emit(Instr::Move { dst: r_dst, src: r_else });
                self.patch_jump(jmp);
                r_dst
            }

            Expr::Propagate(inner) => {
                let r = self.compile_expr(inner);
                self.emit(Instr::Propagate { src: r });
                r
            }

            Expr::Spawn(inner) => {
                match inner.as_ref() {
                    Expr::Call { callee, args } => {
                        let r_callee = self.compile_expr(callee);
                        let arg_regs = self.compile_call_args(args);
                        self.emit(Instr::Spawn { callee: r_callee, args: arg_regs });
                    }
                    _ => {
                        let r_fn = self.compile_expr(inner);
                        self.emit(Instr::Spawn { callee: r_fn, args: vec![] });
                    }
                }
                let null_r = self.alloc();
                let ci = self.chunk.add_const(Value::Null);
                self.emit(Instr::LoadConst { dst: null_r, idx: ci });
                null_r
            }
        }
    }

    // helpers
    fn patch_to(&mut self, idx: &usize, target: usize) {
        match &mut self.chunk.instrs[*idx] {
            Instr::Jump { target: t }            => *t = Addr(target as u32),
            Instr::JumpIfFalse { target: t, .. } => *t = Addr(target as u32),
            Instr::JumpIfTrue  { target: t, .. } => *t = Addr(target as u32),
            _ => {}
        }
    }
}

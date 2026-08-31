//! class method lowering — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// IEEE §8.18 access control: is a member with visibility `vis`, DECLARED in
    /// `decl_class`, reachable from the scope currently being lowered? The
    /// accessing class is the class whose METHOD body is being lowered
    /// (`cur_this`); `None` = module/process scope = "outside any class".
    ///
    /// - public: always.
    /// - protected: accessing class is `decl_class` or a descendant of it.
    /// - local: accessing class IS exactly `decl_class`.
    ///
    /// correct-or-loud: when the accessing scope is outside any class, only public
    /// is reachable (no silent allow).
    pub(crate) fn member_access_ok(&self, vis: ast::Visibility, decl_class: &str) -> bool {
        match vis {
            ast::Visibility::Public => true,
            ast::Visibility::Protected => match &self.cur_this {
                Some((_, accessing)) => self.class_is_ancestor(decl_class, accessing),
                None => false,
            },
            ast::Visibility::Local => match &self.cur_this {
                Some((_, accessing)) => accessing == decl_class,
                None => false,
            },
        }
    }

    /// Enforce member access control on a FIELD `decl_class::field` (visibility on
    /// the resolved `ClassField`). Loud-rejects an out-of-scope read/write of a
    /// `local`/`protected` field (IEEE §8.18) instead of silently reading the
    /// wrong/inaccessible storage. No-op for a public field.
    pub(crate) fn check_field_access(&mut self, _decl_class: &str, field: &str, f: &ClassField) {
        if self.member_access_ok(f.vis, &f.decl_class) {
            return;
        }
        let kind = match f.vis {
            ast::Visibility::Local => "local",
            ast::Visibility::Protected => "protected",
            ast::Visibility::Public => return,
        };
        // Name the member's actual DECLARING class (`f.decl_class`), not the static
        // handle class `decl_class` passed in — for an inherited member accessed
        // via a derived handle the two differ, and the access decision itself uses
        // `f.decl_class`, so the message must agree.
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "cannot access {kind} member `{field}` of class `{}` from \
                 here (IEEE §8.18: a {kind} member is reachable only from {})",
                f.decl_class,
                match f.vis {
                    ast::Visibility::Local => "the declaring class's own methods",
                    _ => "the declaring class and its descendants",
                }
            ),
        );
    }

    /// N7 type gate for an assignment `lhs = rhs` (blocking/nonblocking/cont).
    /// Loud-rejects forging a handle from an integral, leaking a handle to an
    /// integral, and assigning `null` to an integral. No-op for non-class code.
    pub(crate) fn check_handle_assign(&mut self, lhs: &ast::Lvalue, rhs: &ast::Expr) {
        let lhs_is_handle = self.lhs_class_name(lhs).is_some();
        let rk = self.ast_handle_kind(rhs);
        if lhs_is_handle {
            // a class handle may only be assigned a handle / null / new / call.
            if rk == HKind::Other {
                self.error(
                    MsgCode::ElabUnsupported,
                    "cannot assign a non-handle value to a class handle (IEEE §8.4 \
                     — handles are not integral; use `new`, another handle, or `null`)",
                );
            }
        } else if matches!(rk, HKind::Handle | HKind::Null) {
            // a handle/null may not be assigned into an integral variable.
            self.error(
                MsgCode::ElabUnsupported,
                "cannot assign a class handle / `null` to a non-handle variable (IEEE §8.4)",
            );
        }
    }

    /// Resolve a member-access path to `(handle_net, static_class, field_name)`.
    /// Forms: `obj.field` (a class-handle variable), `this.field` (inside a
    /// method), or a bare single-segment `field` (a member of the enclosing
    /// `this` object). `None` for any non-class path (so the caller's normal
    /// hierarchical / scalar logic runs unchanged).
    pub(crate) fn resolve_class_member(
        &self,
        path: &ast::HierPath,
    ) -> Option<(u32, String, String)> {
        let segs = &path.segments;
        if segs.len() == 1 {
            // bare `field` inside a method body ⇒ `this.field` — UNLESS a
            // frame-local formal/local of the same name shadows it (IEEE
            // innermost-wins, §8.10/§13.4): then it is the net, not the property.
            if let Some((net, cls)) = self.cur_this.clone() {
                let f = &segs[0].name;
                if self.lookup_net_scoped(f).is_some() {
                    return None; // a shadowing local/formal net wins
                }
                if self.class_field_id(&cls, f).is_some() {
                    return Some((net, cls, f.clone()));
                }
            }
            return None;
        }
        if segs.len() == 2 {
            let obj = &segs[0].name;
            let field = segs[1].name.clone();
            if obj == "this" {
                if let Some((net, cls)) = &self.cur_this {
                    return Some((*net, cls.clone(), field));
                }
            }
            if let Some(net) = self.lookup_net_scoped(obj) {
                if let Some(cls) = self.net_class.get(&net) {
                    return Some((net, cls.clone(), field));
                }
            }
        }
        None
    }

    /// The static class name of a `new`-assignment lvalue: a handle variable
    /// (`h = new`) or a class-handle member (`obj.h = new` / `this.h = new`).
    pub(crate) fn lhs_class_name(&self, lhs: &ast::Lvalue) -> Option<String> {
        let ast::Lvalue::Ident(path) = lhs else {
            return None;
        };
        if path.segments.len() == 1 {
            if let Some(net) = self.lookup_net_scoped(&path.segments[0].name) {
                if let Some(c) = self.net_class.get(&net) {
                    return Some(c.clone());
                }
            }
        }
        if let Some((_, class, field)) = self.resolve_class_member(path) {
            if let Some((_, f)) = self.class_field_id(&class, &field) {
                return f.class_type;
            }
        }
        None
    }

    /// The STATIC class name of a cast OPERAND, for cast-relationship validation.
    /// Resolves only the IEEE-idiomatic forms whose static type vita tracks by
    /// net id: a bare class-handle variable, `this`, or `obj.field` / bare-field
    /// (a handle property). Returns `None` for any other expression (a call, an
    /// arbitrary value, a `null` literal) — the caller treats an unresolvable
    /// operand as a loud reject (correct-or-loud: a cast it cannot validate must
    /// not silently pass). `Paren` is unwrapped.
    pub(crate) fn operand_static_class(&self, e: &ast::Expr) -> Option<String> {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.operand_static_class(inner),
            ast::ExprKind::Ident(p) => {
                if p.segments.len() == 1 {
                    let n = &p.segments[0].name;
                    if n == "this" {
                        return self.cur_this.as_ref().map(|(_, c)| c.clone());
                    }
                    if let Some(net) = self.lookup_net_scoped(n) {
                        return self.net_class.get(&net).cloned();
                    }
                }
                // `obj.field` / bare member where the field is itself a handle.
                if let Some((_, cls, field)) = self.resolve_class_member(p) {
                    if let Some((_, f)) = self.class_field_id(&cls, &field) {
                        return f.class_type;
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Intercept `h = new` / `h = new(args)` (class allocation). Emits a
    /// placeholder blocking-assign tagged in `class_new_sites` (the engine
    /// allocates a heap object + writes its id), then chains the constructor
    /// (`new` method) if one exists. Returns true iff the rhs was a `ClassNew`.
    pub(crate) fn class_blocking_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::ClassNew { args } = &rhs.kind else {
            return false;
        };
        let Some(class_name) = self.lhs_class_name(lhs) else {
            self.error(
                MsgCode::ElabUnsupported,
                "`new` (class allocation) must be assigned to a class handle",
            );
            return true;
        };
        if delay.is_some() {
            self.error(
                MsgCode::ElabUnsupported,
                "an intra-assignment delay on `new` is unsupported (N7)",
            );
        }
        let class_id = self.class_table[&class_name].id;
        // Placeholder rhs (never evaluated — the engine overrides via the marker).
        let rhs0 = self.const_u32_expr(0, 32);
        let lv = self.lower_lvalue(lhs);
        let sid = self.push_stmt(ir::Stmt::BlockingAssign { lhs: lv, rhs: rhs0 });
        self.class_new_sites.insert(sid, class_id);
        b.push_stmt_id(sid);
        // Constructor chain (S2): run the `new` method on the freshly-allocated
        // object, passing the handle as `this`.
        if self.class_find_method(&class_name, "new").is_some() {
            self.lower_ctor_call(b, lhs, &class_name, args);
        } else if !args.is_empty() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "class `{class_name}` has no constructor for {} argument(s)",
                    args.len()
                ),
            );
        }
        true
    }

    /// Lower a method body into the global func-block arena with `cur_this` set so
    /// `this.field` / bare-member accesses resolve against the enclosing object.
    pub(crate) fn lower_class_method_body(&mut self, cname: &str, mi: usize) {
        let method = self.class_table[cname].methods[mi].clone();
        let (Some(fid), Some(this_net)) = (method.fid, method.this_net) else {
            return;
        };
        let body = match (&method.func, &method.task) {
            (Some(f), _) => f.body.clone(),
            (_, Some(t)) => t.body.clone(),
            _ => return,
        };
        // §13.4.4: a method body-local declaration initializer runs at entry.
        let body_decls = match (&method.func, &method.task) {
            (Some(f), _) => f.body_decls.clone(),
            (_, Some(t)) => t.body_decls.clone(),
            _ => Vec::new(),
        };
        // v7 P2-C (mirror of lower_frame_func_body): record `string`-declared method
        // formals so a `string` relational compare in the body routes through
        // `StrCmp` (a frame formal is a scoped net, not in `subst`).
        let m_ports: &[ast::TfPort] = match (&method.func, &method.task) {
            (Some(f), _) => &f.ports,
            (_, Some(t)) => &t.ports,
            _ => &[],
        };
        let fs_base = self.formal_str.len();
        for p in m_ports {
            let is_str = matches!(
                p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                ast::NetVarKind::String
            );
            self.formal_str.push((p.name.name.clone(), is_str));
        }
        let scope_seg = format!("$class${cname}${}", method.name);
        let saved_this = self.cur_this.take();
        let saved_ret = self.cur_return.take();
        let saved_discard = self.cur_discard.take();
        self.cur_this = Some((this_net, cname.to_string()));
        self.cur_discard = method.discard_net;
        let saved_frame = std::mem::replace(&mut self.in_frame_body, true);
        // A class method body reserves no span nets, so it must not inherit an
        // enclosing frame's owner and answer from that frame's window.
        let saved_cfo = self.cur_frame_owner.take();
        // Return var (functions) = base_net + return_slot; None for a void task.
        let m = self.func_metas[fid as usize];
        let retvar = method.func.as_ref().map(|_| m.base_net + m.return_slot);
        // SW2 (IEEE §8.13): a derived ctor that omits an explicit `super.new()`
        // gets one auto-inserted as its first statement, so the base constructor
        // runs. `class_find_method(base,"new")` walks the base chain (a ctor-less
        // intermediate resolves to the nearest ancestor ctor).
        let inject_super: Option<String> = if method.name == "new" {
            self.class_table
                .get(cname)
                .and_then(|ci| ci.base.clone())
                .filter(|base| self.class_find_method(base, "new").is_some())
                .filter(|_| !body_calls_super_new(&body))
        } else {
            None
        };
        let (blocks, entry) = self.with_scope(&scope_seg, |s| {
            let mut b = ProcessBuilder::new();
            // A single exit block: `return` jumps here; the body also falls
            // through to it. `finish()` gives it the implicit `Return` terminator.
            let exit = b.new_block();
            s.cur_return = Some((retvar, exit));
            s.emit_frame_local_inits(&mut b, &body_decls);
            // SW2: auto-inserted super.new() (static dispatch to the base ctor).
            if let Some(base) = &inject_super {
                let this_e = s.push_expr(ir::Expr::Signal {
                    net: this_net,
                    word: None,
                });
                if let Some(call) = s.build_class_call(this_e, base, "new", &[], true) {
                    s.emit_discarded_call(&mut b, call);
                }
            }
            s.lower_stmt(&mut b, &body);
            b.goto(exit);
            b.start_block(exit);
            b.finish()
        });
        self.cur_this = saved_this;
        self.cur_return = saved_ret;
        self.cur_discard = saved_discard;
        self.in_frame_body = saved_frame;
        // A class method body sets `in_frame_body` but reserves no span nets, so
        // it must not inherit an enclosing frame's owner and hit that frame's net.
        self.cur_frame_owner = saved_cfo;
        self.formal_str.truncate(fs_base);
        // Capture the block base AFTER the body closure (round-7): lowering the body may
        // append blocks (a `pkg::f()` inside a method reserves+lowers its frame on
        // demand). For an ordinary method nothing is appended during the closure, so
        // `base` is unchanged → byte-identical. (Mirrors `lower_frame_func_body`.)
        let base = self.func_blocks.len() as u32;
        for mut blk in blocks {
            rebase_terminator(&mut blk.term, base);
            self.func_blocks.push(blk);
        }
        self.funcs[fid as usize].entry = base + entry;
        let m = self.func_metas[fid as usize];
        let entry_bb = self.funcs[fid as usize].entry;
        self.validate_frame_body(&method.name, entry_bb, m.base_net, m.locals_len, false);
    }

    /// Lower every class method (reserve all fids first so mutual/forward method
    /// calls resolve, then lower bodies). Methods are global, self-contained
    /// (this + formals + locals + fields + other methods), so they lower before
    /// any module. Assign virtual slots (S5) before reserving.
    pub(crate) fn lower_class_methods(&mut self) {
        if self.class_order.is_empty() {
            return;
        }
        self.assign_vtables();
        let order = self.class_order.clone();
        for cname in &order {
            let n = self.class_table[cname].methods.len();
            for mi in 0..n {
                self.reserve_class_method(cname, mi);
            }
        }
        // fids now known → fill the vtable (most-derived override per slot).
        self.finalize_vtables();
        for cname in &order {
            let n = self.class_table[cname].methods.len();
            for mi in 0..n {
                self.lower_class_method_body(cname, mi);
            }
        }
    }

    /// The inheritance chain root→…→self (deterministic, cycle-guarded).
    pub(crate) fn class_chain_rootfirst(&self, cname: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut cur = Some(cname.to_string());
        let mut guard = 0;
        while let Some(c) = cur {
            if chain.contains(&c) {
                break;
            }
            cur = self.class_table.get(&c).and_then(|ci| ci.base.clone());
            chain.push(c);
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        chain.reverse();
        chain
    }

    /// Build the call to a class method: `Expr::Call{fid, [this, …args]}`. Resolves
    /// the static method (walking the base chain), records virtual-dispatch info
    /// in `class_calls` keyed by the Call ExprId, and returns the ExprId. `None`
    /// if `path` is not an `obj.method` / `this.method` / `super.method` call.
    pub(crate) fn try_class_method_call(
        &mut self,
        name: &ast::HierPath,
        args: &[ast::Expr],
    ) -> Option<u32> {
        let segs = &name.segments;
        // A bare `m(args)` inside a method body is a self-call `this.m(args)` —
        // but only if `m` is a method of the enclosing object's class (else it is
        // an ordinary free function, left to `inline_function`).
        if segs.len() == 1 {
            let (tnet, cls) = self.cur_this.clone()?;
            let meth = segs[0].name.clone();
            self.class_find_method(&cls, &meth)?;
            let this_e = self.push_expr(ir::Expr::Signal {
                net: tnet,
                word: None,
            });
            return self.build_class_call(this_e, &cls, &meth, args, false);
        }
        if segs.len() != 2 {
            return None;
        }
        let recv = &segs[0].name;
        let meth = &segs[1].name;
        // Resolve the receiver handle + its static class, and whether this is a
        // `super.` call (forces a static, non-virtual dispatch to the base).
        let (this_eid, class, is_super) = if recv == "super" {
            let (tnet, cls) = self.cur_this.clone()?;
            let base = self.class_table.get(&cls).and_then(|ci| ci.base.clone())?;
            let this_e = self.push_expr(ir::Expr::Signal {
                net: tnet,
                word: None,
            });
            (this_e, base, true)
        } else if recv == "this" {
            let (tnet, cls) = self.cur_this.clone()?;
            let this_e = self.push_expr(ir::Expr::Signal {
                net: tnet,
                word: None,
            });
            (this_e, cls, false)
        } else {
            let net = self.lookup_net_scoped(recv)?;
            let cls = self.net_class.get(&net)?.clone();
            let this_e = self.push_expr(ir::Expr::Signal { net, word: None });
            (this_e, cls, false)
        };
        let meth = meth.clone();
        self.build_class_call(this_eid, &class, &meth, args, is_super)
    }

    /// Common method-call builder: resolve `class::meth` (base-chain walk), build
    /// `Expr::Call{fid, [this, …args]}`, and (unless `is_super`) record virtual
    /// dispatch in `class_calls` keyed by the Call ExprId.
    pub(crate) fn build_class_call(
        &mut self,
        this_eid: u32,
        class: &str,
        meth: &str,
        args: &[ast::Expr],
        is_super: bool,
    ) -> Option<u32> {
        let (owner, method) = self.class_find_method(class, meth)?;
        // IEEE §8.18: loud-reject an out-of-scope call of a local/protected method
        // (the method is DECLARED in `owner`, the visibility owner). Never silently
        // dispatch an inaccessible method.
        if !self.member_access_ok(method.vis, &owner) {
            let kind = match method.vis {
                ast::Visibility::Local => "local",
                ast::Visibility::Protected => "protected",
                ast::Visibility::Public => "",
            };
            if !kind.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "cannot call {kind} method `{meth}` of class `{owner}` from here \
                         (IEEE §8.18)"
                    ),
                );
            }
        }
        // G4: reject a `string`-returning class method call (same reason as the module /
        // package function paths — a frame String return slot reads back empty, a
        // silent-wrong). Guard the CALL rather than silently returning an empty string.
        if method.func.as_ref().is_some_and(|f| f.ret_string) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "method `{class}::{meth}`: a `string` return type is not yet supported \
                     (return the text via a `string` output/ref formal instead)"
                ),
            );
            return Some(self.placeholder_expr());
        }
        let fid = match method.fid {
            Some(f) => f,
            None => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("method `{class}::{meth}` was not lowered"),
                );
                return Some(self.placeholder_expr());
            }
        };
        // Output/ref task ports are outside the MVP (only `this` + inputs bind).
        // §13.5.3: fill omitted trailing args with their default values (mirroring
        // the frame/inline/task call paths) — without this, an omitted default arg
        // bound 0/X instead of the default expression (a silent-wrong).
        let ports: &[ast::TfPort] = match (&method.func, &method.task) {
            (Some(f), _) => &f.ports,
            (_, Some(t)) => &t.ports,
            _ => &[],
        };
        let Some(eff_args) = self.fill_default_args(meth, ports, args) else {
            return Some(self.placeholder_expr()); // loud already emitted
        };
        // A FILLED default (index ≥ args.len()) lowers in the CALLER scope, but IEEE
        // resolves it in the class scope; a non-literal (name/call) default is
        // scope-ambiguous — loud-reject rather than silently bind a caller-scope name.
        for a in &eff_args[args.len()..] {
            if !Self::default_is_scope_safe(a) {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "method `{owner}::{meth}`: a non-literal default argument value is \
                         unsupported (it would resolve in the caller's scope, not the class \
                         scope) — pass the argument explicitly"
                    ),
                );
                return Some(self.placeholder_expr());
            }
        }
        let mut call_args = vec![this_eid];
        for a in eff_args {
            call_args.push(self.lower_expr(a));
        }
        let eid = self.push_expr(ir::Expr::Call {
            func: fid,
            args: call_args,
        });
        // Virtual dispatch: a `super.` call is ALWAYS static; otherwise a virtual
        // method redirects at run time via the receiver's dynamic class.
        let vslot = if is_super { None } else { method.vslot };
        if vslot.is_some() {
            self.class_calls.insert(eid, (vslot, fid));
        }
        Some(eid)
    }

    /// Run the constructor on the freshly-`new`ed object (`lhs`). Lowers a
    /// discarded `Expr::Call` to the `new` method with `this` = the new handle.
    pub(crate) fn lower_ctor_call(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        class_name: &str,
        args: &[ast::Expr],
    ) {
        let Some((owner, method)) = self.class_find_method(class_name, "new") else {
            return;
        };
        let Some(fid) = method.fid else {
            self.error(
                MsgCode::ElabUnsupported,
                &format!("constructor of `{class_name}` was not lowered"),
            );
            return;
        };
        // `this` = read the freshly-allocated handle (the lvalue as a value).
        let this_eid = self.ctor_this_expr(lhs);
        // §13.5.3: fill omitted trailing ctor args with their default values (the
        // same gap as `build_class_call` — an omitted default bound 0/X silently).
        let ports: &[ast::TfPort] = match (&method.func, &method.task) {
            (Some(f), _) => &f.ports,
            (_, Some(t)) => &t.ports,
            _ => &[],
        };
        let Some(eff_args) = self.fill_default_args("new", ports, args) else {
            return; // loud already emitted
        };
        // A filled ctor default lowers in the caller scope (see `build_class_call`);
        // a non-literal (name/call) default is scope-ambiguous — loud-reject.
        for a in &eff_args[args.len()..] {
            if !Self::default_is_scope_safe(a) {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "constructor of `{owner}`: a non-literal default argument value is \
                         unsupported (it would resolve in the caller's scope, not the class \
                         scope) — pass the argument explicitly"
                    ),
                );
                return;
            }
        }
        let mut call_args = vec![this_eid];
        for a in eff_args {
            call_args.push(self.lower_expr(a));
        }
        let call = self.push_expr(ir::Expr::Call {
            func: fid,
            args: call_args,
        });
        // A ctor is a void method (no virtual dispatch — IEEE: `new` is not
        // virtual). Discard the call result via a throwaway assign to a fresh net.
        self.emit_discarded_call(b, call);
    }

    /// Lower a class field READ (`obj.field`) to `Signal{net, word: field-id}`.
    /// `None` if `path` is not a class member access.
    pub(crate) fn try_class_field_read(&mut self, path: &ast::HierPath) -> Option<u32> {
        let (net, class, field) = self.resolve_class_member(path)?;
        match self.class_field_id(&class, &field) {
            Some((fid, f)) => {
                // IEEE §8.18: loud-reject an out-of-scope read of a local/protected
                // field (never silently read inaccessible storage).
                self.check_field_access(&class, &field, &f);
                let word = self.const_u32_expr(fid, 32);
                let eid = self.push_expr(ir::Expr::Signal {
                    net,
                    word: Some(word),
                });
                // The Signal's net is the 32-bit handle; record the FIELD width so
                // the engine width table reports `obj.field`'s width correctly.
                self.class_field_widths.insert(eid, (f.width, f.signed));
                Some(eid)
            }
            None => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("class `{class}` has no member `{field}`"),
                );
                Some(self.placeholder_expr())
            }
        }
    }
}

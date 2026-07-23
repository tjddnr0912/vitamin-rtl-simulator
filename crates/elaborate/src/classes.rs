//! class registry / vtables — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// N7: resolved metadata for one class, built by the whole-design prescan
/// (forward-reference safe). `fields` are in stable field-id order with
/// BASE-class fields FIRST (so a derived object up-cast to its base reads the
/// same field-ids). `methods` are the directly-declared methods (inherited ones
/// resolve by walking `base`).
#[derive(Clone)]
pub(crate) struct ClassInfo {
    pub(crate) id: u32,
    pub(crate) base: Option<String>,
    pub(crate) fields: Vec<ClassField>,
    pub(crate) methods: Vec<ClassMethod>,
    /// N7-REST: names of this class's OWN `rand` data members (inherited rand-ness
    /// is recovered by walking the base chain when bounds are built).
    pub(crate) rand_fields: Vec<String>,
    /// N7-REST B2: names of this class's OWN `randc` (cyclic) members (a subset of
    /// `rand_fields` — they are also drawn, but cyclically not uniformly).
    pub(crate) randc_fields: Vec<String>,
    /// N7-REST: this class's OWN `constraint` blocks (base constraints apply too).
    pub(crate) constraints: Vec<ast::ConstraintDecl>,
}

/// One class data member (resolved layout).
#[derive(Clone)]
pub(crate) struct ClassField {
    pub(crate) name: String,
    pub(crate) width: u32,
    pub(crate) signed: bool,
    /// 4-state (logic/reg/integer) ⇒ default X; 2-state (int/bit/handle) ⇒ default 0.
    pub(crate) four_state: bool,
    /// For a class-typed (handle) member, the sub-class name (nested handles).
    pub(crate) class_type: Option<String>,
    /// SW1 (IEEE §8.8): a folded constant declaration initializer (`int x = 42`),
    /// applied at `new` instead of the bare type default. `None` = no initializer
    /// (use the type default 0/X). Non-constant initializers loud-reject.
    pub(crate) init: Option<ir::BitPacked>,
    /// IEEE §8.18 access control (`local`/`protected`/public).
    pub(crate) vis: ast::Visibility,
    /// The class that DECLARES this field (the visibility owner). In a flattened
    /// derived layout this is the base class for an inherited field — so a
    /// `protected` check knows the accessing scope must descend from it.
    pub(crate) decl_class: String,
}

/// N7 type-gate classification of an expression (handle vs integral).
#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum HKind {
    /// A class-handle value (`obj`, `this`, `new`, a handle field).
    Handle,
    /// The `null` literal.
    Null,
    /// A call that might return a handle — lenient (never false-reject).
    Unknown,
    /// A clearly-integral value (literal, arithmetic, non-handle net, …).
    Other,
}

/// One class method (declared directly in a class). `fid`/`vslot` are filled in
/// when the body lowers (S3/S5). The constructor is the method named `new`.
#[derive(Clone)]
pub(crate) struct ClassMethod {
    pub(crate) name: String,
    pub(crate) is_virtual: bool,
    /// IEEE §8.18 access control (`local`/`protected`/public).
    pub(crate) vis: ast::Visibility,
    pub(crate) func: Option<ast::FunctionDef>,
    pub(crate) task: Option<ast::TaskDef>,
    pub(crate) fid: Option<u32>,
    pub(crate) vslot: Option<u32>,
    /// The frame net allocated for the implicit `this` (slot 0), filled at
    /// reservation. Used to set `cur_this` while lowering the body.
    pub(crate) this_net: Option<u32>,
    /// A frame-local 64-bit scratch net that discards the result of a nested
    /// void call (`super.new()` / `obj.m();` inside the body) — its write must
    /// land in-frame, not on a freshly-allocated module net.
    pub(crate) discard_net: Option<u32>,
}

/// Const-fold a net/var initializer literal into a `BitPacked` of `width`.
/// Non-literal initializers → None (procedural; deferred), caller defaults.
/// SW2: does a constructor body already contain an explicit `super.new(...)`
/// call? Scans the top-level statements (recursing into nested `begin…end`
/// blocks) for a `super.new` subroutine call — IEEE §8.13 requires it be the
/// first statement, but scanning the block is robust against placement.
pub(crate) fn body_calls_super_new(body: &ast::Stmt) -> bool {
    match body {
        ast::Stmt::UserTaskCall { name, .. } => {
            name.segments.len() == 2
                && name.segments[0].name == "super"
                && name.segments[1].name == "new"
        }
        ast::Stmt::Block { stmts, .. } => stmts.iter().any(body_calls_super_new),
        _ => false,
    }
}

impl Elaborator<'_> {
    pub(crate) fn register_classes(&mut self, unit: &ast::SourceUnit) {
        let mut decls: Vec<&ast::ClassDecl> = Vec::new();
        for it in &unit.items {
            match it {
                ast::TopItem::Class(c) => decls.push(c),
                ast::TopItem::Module(m) | ast::TopItem::Interface(m) | ast::TopItem::Package(m) => {
                    for bi in &m.body {
                        if let ast::ModuleItem::Class(c) = bi {
                            decls.push(c);
                        }
                    }
                }
                _ => {}
            }
        }
        // Pass 1: own fields + methods + base name, in declaration order.
        for c in &decls {
            if self.class_table.contains_key(&c.name.name) {
                self.error(
                    MsgCode::DupUnit,
                    &format!("class `{}` declared more than once", c.name.name),
                );
                continue;
            }
            let id = self.class_order.len() as u32;
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            let mut rand_fields = Vec::new();
            let mut randc_fields = Vec::new();
            let mut constraints = Vec::new();
            let cname = c.name.name.clone();
            for item in &c.items {
                match item {
                    ast::ClassItem::Property(vis, d) => {
                        self.collect_class_fields(d, *vis, &cname, &mut fields)
                    }
                    ast::ClassItem::RandProperty { randc, decl } => {
                        let before = fields.len();
                        // A rand member is always public in this slice (the parser
                        // rejects `local`/`protected rand`).
                        self.collect_class_fields(
                            decl,
                            ast::Visibility::Public,
                            &cname,
                            &mut fields,
                        );
                        for f in &fields[before..] {
                            rand_fields.push(f.name.clone());
                            // `randc` (cyclic random, B2): also a rand field, but drawn
                            // as a random permutation that visits every value once per
                            // cycle (per-instance state in the engine).
                            if *randc {
                                randc_fields.push(f.name.clone());
                            }
                        }
                    }
                    ast::ClassItem::Constraint(cd) => constraints.push(cd.clone()),
                    ast::ClassItem::Func {
                        is_virtual,
                        vis,
                        def,
                    } => methods.push(ClassMethod {
                        name: def.name.name.clone(),
                        is_virtual: *is_virtual,
                        vis: *vis,
                        func: Some(def.clone()),
                        task: None,
                        fid: None,
                        vslot: None,
                        this_net: None,
                        discard_net: None,
                    }),
                    ast::ClassItem::Task {
                        is_virtual,
                        vis,
                        def,
                    } => methods.push(ClassMethod {
                        name: def.name.name.clone(),
                        is_virtual: *is_virtual,
                        vis: *vis,
                        func: None,
                        task: Some(def.clone()),
                        fid: None,
                        vslot: None,
                        this_net: None,
                        discard_net: None,
                    }),
                    ast::ClassItem::Error(_) => {}
                }
            }
            self.class_order.push(c.name.name.clone());
            self.class_table.insert(
                c.name.name.clone(),
                ClassInfo {
                    id,
                    base: c.extends.as_ref().map(|b| b.name.clone()),
                    fields,
                    methods,
                    rand_fields,
                    randc_fields,
                    constraints,
                },
            );
        }
        // Pass 2: flatten the inheritance chain into base-first field lists, using
        // a SNAPSHOT of each class's OWN fields (mutation-order independent). A
        // missing/cyclic base is loud (never an infinite loop).
        let own: std::collections::BTreeMap<String, Vec<ClassField>> = self
            .class_table
            .iter()
            .map(|(k, v)| (k.clone(), v.fields.clone()))
            .collect();
        let names = self.class_order.clone();
        for name in &names {
            // Walk self→base→…→root, guarding against cycles / unknown bases.
            let mut chain: Vec<String> = Vec::new();
            let mut cur = Some(name.clone());
            let mut guard = 0;
            while let Some(n) = cur {
                if chain.contains(&n) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("cyclic class inheritance involving `{n}`"),
                    );
                    break;
                }
                let base = self.class_table.get(&n).and_then(|ci| ci.base.clone());
                chain.push(n);
                cur = match base {
                    Some(b) if own.contains_key(&b) => Some(b),
                    Some(b) => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!("class extends unknown class `{b}`"),
                        );
                        None
                    }
                    None => None,
                };
                guard += 1;
                if guard > 256 {
                    break;
                }
            }
            // Emit root→self (reverse of the chain). A derived field whose name
            // SHADOWS an inherited base field (IEEE §8.14) gets its OWN slot,
            // appended AFTER the base's — distinct storage. Because the order is
            // root→self, the base field keeps its low slot (so a base method
            // reaches it) and the most-derived field is last (so a derived method
            // and external `obj.f` reach it — `class_field_id` resolves by the LAST
            // matching name). A base field's slot is therefore identical in the
            // base and every derived layout, keeping per-object storage consistent.
            let mut flat: Vec<ClassField> = Vec::new();
            for n in chain.iter().rev() {
                if let Some(fs) = own.get(n) {
                    for f in fs {
                        flat.push(f.clone());
                    }
                }
            }
            if let Some(ci) = self.class_table.get_mut(name) {
                ci.fields = flat;
            }
        }
    }

    /// Resolve a class property declaration into `ClassField`s (one per name).
    /// `vis` is the member's `local`/`protected`/public access control and
    /// `decl_class` the class that declares it (the visibility owner).
    pub(crate) fn collect_class_fields(
        &mut self,
        d: &ast::NetVarDecl,
        vis: ast::Visibility,
        decl_class: &str,
        out: &mut Vec<ClassField>,
    ) {
        // Class-typed (handle) member: a 32-bit object-id, default null (0).
        if matches!(d.kind, ast::NetVarKind::ClassHandle) {
            let ct = d.class_type.as_ref().map(|i| i.name.clone());
            for n in &d.names {
                if !n.unpacked.is_empty() {
                    // an array of handles is outside the MVP — loud, NOT a silent
                    // scalar-handle that drops the `[N]`.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "an array-of-handles class member is outside the N7 MVP",
                    );
                    continue;
                }
                out.push(ClassField {
                    name: n.name.name.clone(),
                    width: 32,
                    signed: false,
                    four_state: false,
                    class_type: ct.clone(),
                    // A handle field defaults to null (0); an explicit `= expr`
                    // handle initializer is outside the MVP (null is the default).
                    init: None,
                    vis,
                    decl_class: decl_class.to_string(),
                });
            }
            return;
        }
        // Real members are deferred (the heap default-init + read/write would
        // need a real lane) — loud, not a silent X-as-real.
        if matches!(d.kind, ast::NetVarKind::Real | ast::NetVarKind::Realtime) {
            self.error(
                MsgCode::ElabUnsupported,
                "real class members are outside the N7 MVP",
            );
            return;
        }
        if matches!(d.kind, ast::NetVarKind::String) {
            self.error(
                MsgCode::ElabUnsupported,
                "string class members are outside the N7 MVP",
            );
            return;
        }
        let (base_w, _, _, signed) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
        let mut width = base_w;
        for pr in &d.packed {
            let (pw, _, _, _) = self.range_to_dims(ast::NetVarKind::Logic, Some(pr), false);
            width = width.saturating_mul(pw.max(1));
        }
        let four_state = !net_kind_is_two_state(d.kind);
        for n in &d.names {
            if !n.unpacked.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "array class members are outside the N7 MVP",
                );
                continue;
            }
            // SW1: a declaration initializer (`int x = 42`) is folded to a constant
            // sized to the field width and applied at `new` (IEEE §8.8). A
            // non-constant initializer loud-rejects (no silent drop, no runtime fold).
            let init = match &n.init {
                Some(e) => match fold_init(e, width) {
                    Some(bits) => Some(bits),
                    None => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "a non-constant class field initializer is outside the N7 MVP \
                             (assign it in the constructor)",
                        );
                        None
                    }
                },
                None => None,
            };
            out.push(ClassField {
                name: n.name.name.clone(),
                width,
                signed,
                four_state,
                class_type: None,
                init,
                vis,
                decl_class: decl_class.to_string(),
            });
        }
    }

    /// Drain `class_table` into the engine sidecar: `[class_id]` → per-field
    /// `(width, signed, four_state)` in stable field-id order.
    pub(crate) fn class_layout_table(&self) -> Vec<Vec<(u32, bool, bool)>> {
        self.class_order
            .iter()
            .map(|name| {
                self.class_table
                    .get(name)
                    .map(|ci| {
                        ci.fields
                            .iter()
                            .map(|f| (f.width, f.signed, f.four_state))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    /// SW1: per-class folded field initializers, parallel to `class_layout_table`
    /// (`[class_id][field_id]` → `Some(bits)` if the field has a `= const`
    /// initializer, else `None`). Consumed by the engine `new` default-init.
    pub(crate) fn class_field_inits(&self) -> Vec<Vec<Option<ir::BitPacked>>> {
        self.class_order
            .iter()
            .map(|name| {
                self.class_table
                    .get(name)
                    .map(|ci| ci.fields.iter().map(|f| f.init.clone()).collect())
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Field-id (index into the flattened field list) of `field` in `class`.
    pub(crate) fn class_field_id(&self, class: &str, field: &str) -> Option<(u32, ClassField)> {
        let ci = self.class_table.get(class)?;
        // LAST match wins (IEEE §8.14 shadowing): in the root→self flat layout the
        // most-derived field of a shadowed name sits last, so a reference resolved
        // against the derived class picks the derived slot; against a base class
        // (a base method) the base layout only holds the base slot. Without
        // shadowing each name is unique, so this is identical to a first-match.
        ci.fields
            .iter()
            .rposition(|f| f.name == field)
            .map(|i| (i as u32, ci.fields[i].clone()))
    }

    /// N7 type gate: classify an expression as a class HANDLE value, the NULL
    /// literal, an UNKNOWN (a call that might return a handle — be lenient), or
    /// OTHER (a clearly-integral value). Used to reject mixing handles with
    /// integral operands (IEEE §8.4/§11.4) — closes the forge/use-after-free hole.
    pub(crate) fn ast_handle_kind(&self, e: &ast::Expr) -> HKind {
        match &e.kind {
            ast::ExprKind::Null => HKind::Null,
            ast::ExprKind::ClassNew { .. } => HKind::Handle,
            ast::ExprKind::Paren { inner } => self.ast_handle_kind(inner),
            // A.5: a class cast `Base'(expr)` yields a HANDLE (an up-cast is an
            // identity on the handle value). A single-seg path naming a class is
            // the class-cast form; any other cast target (size/prim/signing) is
            // integral → Other. (The legality of the cast itself is checked in
            // `lower_cast`; here we only classify the RESULT for the assign gate.)
            ast::ExprKind::Cast {
                target: ast::CastTarget::Named(p),
                ..
            } if p.segments.len() == 1 && self.class_table.contains_key(&p.segments[0].name) => {
                HKind::Handle
            }
            // A function/method call MIGHT return a handle — be lenient so a
            // legitimate `h = factory()` / `int x = f()` is never false-rejected.
            ast::ExprKind::Call { .. } => HKind::Unknown,
            ast::ExprKind::Ident(p) => {
                if p.segments.len() == 1 {
                    let n = &p.segments[0].name;
                    if n == "this" || n == "super" {
                        return HKind::Handle;
                    }
                    if let Some(net) = self.lookup_net_scoped(n) {
                        return if self.net_class.contains_key(&net) {
                            HKind::Handle
                        } else {
                            HKind::Other
                        };
                    }
                }
                // `obj.field` / bare member where the field is itself a handle.
                if let Some((_, cls, field)) = self.resolve_class_member(p) {
                    if let Some((_, f)) = self.class_field_id(&cls, &field) {
                        return if f.class_type.is_some() {
                            HKind::Handle
                        } else {
                            HKind::Other
                        };
                    }
                }
                HKind::Other
            }
            _ => HKind::Other, // literals, arithmetic, selects, concat, …
        }
    }

    /// Does `class` (or a base) declare a method named `name`?
    pub(crate) fn class_find_method(
        &self,
        class: &str,
        name: &str,
    ) -> Option<(String, ClassMethod)> {
        let mut cur = Some(class.to_string());
        let mut guard = 0;
        while let Some(c) = cur {
            if let Some(ci) = self.class_table.get(&c) {
                if let Some(m) = ci.methods.iter().find(|m| m.name == name) {
                    return Some((c.clone(), m.clone()));
                }
                cur = ci.base.clone();
            } else {
                return None;
            }
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        None
    }

    /// True iff `anc` is a (strict or improper) ancestor of `desc` — i.e. `desc`
    /// is `anc` itself or `anc` appears on `desc`'s base chain (IEEE §8.16). The
    /// 256-iteration guard mirrors `class_find_method` (a malformed cyclic base
    /// chain terminates loudly rather than spinning). Used to validate a class
    /// cast: an UP-cast (`anc` = the cast target, `desc` = the operand class) is
    /// legal; a DOWN-cast / unrelated cast is not.
    pub(crate) fn class_is_ancestor(&self, anc: &str, desc: &str) -> bool {
        let mut cur = Some(desc.to_string());
        let mut guard = 0;
        while let Some(c) = cur {
            if c == anc {
                return true;
            }
            cur = self.class_table.get(&c).and_then(|ci| ci.base.clone());
            guard += 1;
            if guard > 256 {
                break;
            }
        }
        false
    }

    // ── N7 method lowering ─────────────────────────────────────────────────
    // Every method (function OR task) is lowered as a frame-FUNCTION with `this`
    // injected as formal slot 0; a call is an `Expr::Call{fid, [this, …args]}`
    // (a void task-method's call expression is simply discarded). This reuses the
    // B-track frame-call machinery verbatim and sidesteps `TaskCallInfo`. Field
    // writes inside a body route to the heap (the frame executor + validator are
    // taught the class-field exception). Output (`output`/`ref`) task ports are
    // outside this MVP (loud at the call site).

    /// Reserve a method's FuncDef + frame nets (slot 0 = `this`, then params,
    /// then — for functions — the return var, then body_decls). Records the fid +
    /// this-net back into the class method entry.
    pub(crate) fn reserve_class_method(&mut self, cname: &str, mi: usize) {
        let method = self.class_table[cname].methods[mi].clone();
        let fid = self.funcs.len() as u32;
        let base_net = self.nets.len() as u32;
        let scope_seg = format!("$class${cname}${}", method.name);
        let is_func = method.func.is_some();
        let (ret_width, ret_signed) = match &method.func {
            Some(f) => self.func_return_dims(f),
            None => (1, false), // task: a dummy 1-bit return, discarded by the caller
        };
        // A 2-state return type (`function bit`/`byte`/…) can never hold X/Z — register
        // the return + 2-state formals/locals for X/Z→0 coercion (§6.11.3), mirroring the
        // frame-function reserve; `reserve_class_method` previously did this for NONE, so
        // a 2-state class-method local silently kept an X-write (`bit x = 8'hxA` → `xa`).
        let ret_two_state = method.func.as_ref().is_some_and(|f| f.ret_two_state);
        let (ports, body_decls): (Vec<ast::TfPort>, Vec<ast::NetVarDecl>) =
            match (&method.func, &method.task) {
                (Some(f), _) => (f.ports.clone(), f.body_decls.clone()),
                (_, Some(t)) => (t.ports.clone(), t.body_decls.clone()),
                _ => (Vec::new(), Vec::new()),
            };
        // Output/inout method ports need copy-OUT to the caller lvalue, which the
        // discarded-`Expr::Call` model does not provide — loud, NOT a silent drop
        // of the write-back. (Methods communicate via `this`/fields + the return.)
        if ports.iter().any(|p| p.dir != ast::PortDir::Input) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "method `{cname}::{}` has an output/inout port — outside the N7 MVP \
                     (use the return value or object fields)",
                    method.name
                ),
            );
        }
        let nports = ports.len() as u32;
        let n_params = 1 + nports; // `this` + declared formals
                                   // v7: per-formal `string` bitmask, indexed by FRAME SLOT (`this` = slot 0,
                                   // formal i = slot 1+i). A method call lowers to `Expr::Call{args:[this, a0,
                                   // …]}`, so the arg index EQUALS the slot index — the same `formal_is_string`
                                   // materialization path as frame functions (§4.5.87) then binds a string
                                   // LITERAL actual as a heap string instead of truncating it to the 1-bit
                                   // slot. See `FuncMeta.str_params`.
        let mut str_params: u64 = 0;
        for (i, p) in ports.iter().enumerate() {
            if matches!(
                p.net_or_var.unwrap_or(ast::NetVarKind::Reg),
                ast::NetVarKind::String
            ) {
                let slot = i + 1; // `this` occupies slot 0
                if slot < 64 {
                    str_params |= 1u64 << slot;
                } else {
                    self.error_unsupported(
                        p.span,
                        "a `string` formal beyond parameter index 62 is unsupported \
                         (the 64-wide frame-call string mask reserves slot 0 for `this`)",
                    );
                }
            }
        }
        let mname = method.name.clone();
        let cname_s = cname.to_string();
        let this_net = self.with_scope(&scope_seg, |s| {
            // slot 0: implicit `this` (a 32-bit class handle).
            let tn = s.nets.len() as u32;
            s.add_net(
                "this",
                ir::NetVar {
                    kind: ir::NetKind::Integer,
                    width: 32,
                    msb: 31,
                    lsb: 0,
                    signed: false,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(ast::NetVarKind::ClassHandle, 32),
                },
            );
            s.class_handle_nets.insert(tn);
            s.net_class.insert(tn, cname_s.clone());
            // slots 1..=nports: declared formals.
            for p in &ports {
                let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
                let (w, msb, lsb, signed) = s.range_to_dims(kind, p.range.as_ref(), p.signed);
                let net = s.nets.len() as u32;
                s.add_net(
                    &p.name.name,
                    ir::NetVar {
                        kind: map_net_kind_or_wire(kind),
                        width: w,
                        msb,
                        lsb,
                        signed,
                        array_len: 1,
                        dir: ir::PortDir::Internal,
                        init: default_init(kind, w),
                    },
                );
                if net_kind_is_two_state(kind) {
                    s.intro_kind.insert(net, kind);
                }
            }
            // Return var at slot `n_params`, named after the method. ALWAYS
            // allocated (even for void task-methods — they get a discarded 1-bit
            // dummy) so `return_slot < locals_len` holds and the frame router's
            // range check passes; only a FUNCTION's body assigns it.
            let _ = is_func;
            let ret_net = s.nets.len() as u32;
            s.add_net(
                &mname,
                ir::NetVar {
                    kind: if ret_width == 32 && ret_signed {
                        ir::NetKind::Integer
                    } else {
                        ir::NetKind::Reg
                    },
                    width: ret_width,
                    msb: ret_width.saturating_sub(1),
                    lsb: 0,
                    signed: ret_signed,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(ast::NetVarKind::Reg, ret_width),
                },
            );
            // A 2-state return coerces X/Z→0 (§6.11.3); the specific 2-state kind is
            // immaterial to coercion, so pick one by width (mirrors `reserve_frame_func`).
            if ret_two_state {
                let k = match ret_width {
                    0..=8 => ast::NetVarKind::Byte,
                    9..=16 => ast::NetVarKind::Shortint,
                    17..=32 => ast::NetVarKind::Int,
                    _ => ast::NetVarKind::Longint,
                };
                s.intro_kind.insert(ret_net, k);
            }
            // body_decls (scalars; a multi-dim PACKED local widens to full width +
            // registers packed_dims/dim_desc — same net-based reserve as a frame local).
            for d in &body_decls {
                for decl in &d.names {
                    s.reserve_frame_local_decl(&decl.name.name, d, &decl.unpacked);
                }
            }
            // A frame-local 64-bit scratch slot for discarding nested void-call
            // results inside this body (the write must land in-frame).
            let dn = s.nets.len() as u32;
            s.add_net(
                "$discard",
                ir::NetVar {
                    kind: ir::NetKind::Reg,
                    width: 64,
                    msb: 63,
                    lsb: 0,
                    signed: false,
                    array_len: 1,
                    dir: ir::PortDir::Internal,
                    init: default_init(ast::NetVarKind::Reg, 64),
                },
            );
            (tn, dn)
        });
        let (this_net, discard_net) = this_net;
        let locals_len = self.nets.len() as u32 - base_net;
        self.funcs.push(ir::FuncDef {
            entry: 0,
            n_params,
            locals_len,
            is_task: false,
        });
        self.func_metas.push(FuncMeta {
            base_net,
            n_params,
            return_slot: n_params, // return var sits right after this+formals
            locals_len,
            is_automatic: true, // class methods are automatic (fresh locals per call)
            ret_width,
            ret_signed,
            auto_override: 0,
            str_params,
            has_hier_call: false,
        });
        self.frame_func_names.push(method.name.clone()); // %m
        if let Some(ci) = self.class_table.get_mut(cname) {
            ci.methods[mi].fid = Some(fid);
            ci.methods[mi].this_net = Some(this_net);
            ci.methods[mi].discard_net = Some(discard_net);
        }
    }

    /// S5: assign a virtual slot per virtual method name (shared across the
    /// inheritance chain so an override reuses the base's slot), and build
    /// `class_vtable[class_id][vslot] = most-derived fid`. Non-virtual methods get
    /// no slot. Run after fids are known — so call it AFTER reservation? No: vslots
    /// must exist at reservation-independent time; we assign slot NUMBERS here
    /// (pre-reservation) and fill the fid table in `finalize_vtables` post-reserve.
    pub(crate) fn assign_vtables(&mut self) {
        // Slot numbering: per ROOT class lineage, a virtual method name gets a
        // stable slot. A derived override of a base virtual reuses the base slot.
        let order = self.class_order.clone();
        for cname in &order {
            // Collect the virtual method names visible in this class (base chain),
            // base-first, deduped → slot index = position.
            let mut slots: Vec<String> = Vec::new();
            let chain = self.class_chain_rootfirst(cname);
            for c in &chain {
                let methods = self.class_table[c].methods.clone();
                for m in &methods {
                    if m.is_virtual && !slots.contains(&m.name) {
                        slots.push(m.name.clone());
                    }
                }
            }
            // Assign vslot to THIS class's own methods by name position. A method
            // gets a slot if it is declared `virtual` OR its name matches an
            // ancestor virtual (IEEE §8.20: virtuality is INHERITED — an override
            // need not repeat the keyword), so a keyword-less override still
            // dispatches dynamically.
            let nm = self.class_table[cname].methods.len();
            for mi in 0..nm {
                let name = self.class_table[cname].methods[mi].name.clone();
                // In `slots` ⇒ the name is virtual somewhere in the chain (self or
                // an ancestor) ⇒ this method occupies that vtable slot.
                if let Some(pos) = slots.iter().position(|s| *s == name) {
                    self.class_table.get_mut(cname).unwrap().methods[mi].vslot = Some(pos as u32);
                }
            }
        }
    }

    /// After all method fids are reserved, fill `class_vtable[class_id][vslot]`
    /// with the most-derived override fid for each class.
    pub(crate) fn finalize_vtables(&mut self) {
        let order = self.class_order.clone();
        // Determine the max vslot across all classes.
        let mut max_slot = 0u32;
        for cname in &order {
            for m in &self.class_table[cname].methods {
                if let Some(v) = m.vslot {
                    max_slot = max_slot.max(v + 1);
                }
            }
        }
        self.class_vtable = vec![Vec::new(); order.len()];
        for cname in &order {
            let cid = self.class_table[cname].id as usize;
            let mut table = vec![u32::MAX; max_slot as usize];
            // Walk root→self; a later (more-derived) class overrides the slot.
            for c in self.class_chain_rootfirst(cname) {
                for m in &self.class_table[&c].methods {
                    if let (Some(v), Some(fid)) = (m.vslot, m.fid) {
                        table[v as usize] = fid;
                    }
                }
            }
            self.class_vtable[cid] = table;
        }
    }

    /// Read the handle VALUE of a `new`-lvalue (single-seg net, or `obj.field` /
    /// `this.field` handle member) to pass as the ctor's `this`.
    pub(crate) fn ctor_this_expr(&mut self, lhs: &ast::Lvalue) -> u32 {
        if let ast::Lvalue::Ident(path) = lhs {
            if path.segments.len() == 1 {
                if let Some(net) = self.lookup_net_scoped(&path.segments[0].name) {
                    return self.push_expr(ir::Expr::Signal { net, word: None });
                }
            }
            // obj.h / this.h handle member → read the field (the new handle id).
            if let Some(eid) = self.try_class_field_read(path) {
                return eid;
            }
        }
        self.placeholder_expr()
    }
}

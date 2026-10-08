//! Where the constant domain may size `$bits` of a select by the select alone
//! (`bits_of_selfdet`'s select and signing arms, §4.5.N): a use in the module's own
//! top level, over a name no nested scope of the module declares.
//!
//! The select's base is resolved by NAME through the module's flat tables (formals,
//! parameters, the decl-order `bits_prescan`, the net table). None of them sees a
//! generate block's, a subroutine's, a block's or an interface's own declaration, so
//! a same-named module object would answer for it — and runtime positions ask the
//! constant domain first (a replication count, an indexed part-select width), so a
//! wrong width would override a right lowering. Rather than resolve scopes here, the
//! arms answer only where no such scope can be involved, decided from the module's
//! AST (nested nets may not exist yet when a parameter is bound); everywhere else
//! they decline, as before. Both halves are asked: the select's text sits outside
//! every nested scope, and the fold itself runs in the module's own scope (the
//! current prefix is the instance's — not a generate block's, where a module
//! typedef's range is re-folded at its use).

use super::*;
use crate::block_local::gather_nested_block_locals;

/// An AST census of one module, taken when its body is elaborated: the spans of its
/// nested scopes (generate constructs, subroutines, classes, declaring blocks, the
/// action blocks of deferred and concurrent assertions), every name declared in one
/// of them, every enum label the module declares at any nesting, and its ANSI ports
/// that are packed vectors.
#[derive(Debug, Default, Clone)]
pub(crate) struct SelectScope {
    lo: u32,
    hi: u32,
    nested: Vec<(u32, u32)>,
    /// The condition, case expression and labels, and loop header of a generate
    /// construct written at the module's top level: inside the construct's span but
    /// evaluated in the module's scope (§27.5), so not nested.
    headers: Vec<(u32, u32)>,
    names: std::collections::BTreeSet<String>,
    vector_ports: std::collections::BTreeSet<String>,
}

impl SelectScope {
    pub(crate) fn of_module(module: &ast::ModuleDecl) -> Self {
        let mut sc = SelectScope {
            lo: module.span.lo,
            hi: module.span.hi,
            ..Default::default()
        };
        if let ast::PortList::Ansi(ports) = &module.ports {
            for p in ports {
                let integral = !matches!(
                    p.net_or_var,
                    Some(
                        ast::NetVarKind::Real
                            | ast::NetVarKind::Realtime
                            | ast::NetVarKind::String
                            | ast::NetVarKind::Event
                    )
                );
                if integral
                    && p.range.is_some()
                    && p.iface.is_none()
                    && p.shape_param.is_none()
                    && p.packed.is_empty()
                    && p.unpacked.is_empty()
                {
                    sc.vector_ports.insert(p.name.name.clone());
                }
            }
        }
        for item in &module.body {
            sc.top_item(item);
        }
        sc
    }

    fn top_item(&mut self, item: &ast::ModuleItem) {
        match item {
            ast::ModuleItem::Proc(p) => self.blocks(&p.body),
            ast::ModuleItem::Func(f) => {
                self.nested.push((f.span.lo, f.span.hi));
                self.routine(&f.name, &f.ports, &f.body_decls, &f.body_enums, &f.body);
            }
            ast::ModuleItem::Task(t) => {
                self.nested.push((t.span.lo, t.span.hi));
                self.routine(&t.name, &t.ports, &t.body_decls, &t.body_enums, &t.body);
            }
            ast::ModuleItem::Generate(g) => {
                self.nested.push((g.span.lo, g.span.hi));
                for gi in &g.items {
                    self.gen_header(gi);
                    self.gen_item(gi);
                }
            }
            ast::ModuleItem::Class(c) => self.nested.push((c.span.lo, c.span.hi)),
            ast::ModuleItem::Typedef(td) => self.enum_labels(td),
            _ => {}
        }
    }

    /// An enum typedef's labels: constants that shadow a same-named object.
    fn enum_labels(&mut self, td: &ast::TypedefDecl) {
        if let ast::TypedefKind::Enum { labels, .. } = &td.kind {
            for l in labels {
                self.names.insert(l.name.name.clone());
            }
        }
    }

    /// Every declaring `begin … end` / `fork` block of a statement, and its names,
    /// taken by the walker the block-local lowering classifies with
    /// (`gather_nested_block_locals`, timing-control bodies included) so the census
    /// and the lowering enumerate the same scopes; the outermost block counts too (a
    /// process body's own declarations are its scope, not the module's). And every
    /// deferred or concurrent assertion's action block (`actions`).
    fn blocks(&mut self, s: &ast::Stmt) {
        let mut out = Vec::new();
        gather_nested_block_locals(s, false, &mut out);
        for (sp, names) in out {
            self.nested.push((sp.lo, sp.hi));
            for (n, _) in names {
                self.names.insert(n);
            }
        }
        self.actions(s);
    }

    /// An assertion's action is its own scope: the block-local collector does not
    /// enter it, so its spans and declarations are taken here, at any depth.
    fn actions(&mut self, s: &ast::Stmt) {
        use ast::Stmt as S;
        let action = |me: &mut Self, a: &ast::Stmt| {
            let sp = a.span();
            me.nested.push((sp.lo, sp.hi));
            me.blocks(a);
        };
        match s {
            S::DeferredAssert { then_s, else_s, .. } => {
                action(self, then_s);
                action(self, else_s);
            }
            S::ConcurrentAssert { pass, fail, .. } => {
                for a in [pass, fail].into_iter().flatten() {
                    action(self, a);
                }
            }
            S::Block { stmts, .. } | S::Fork { stmts, .. } => {
                stmts.iter().for_each(|st| self.actions(st));
            }
            S::If { then_s, else_s, .. } => {
                self.actions(then_s);
                if let Some(e) = else_s {
                    self.actions(e);
                }
            }
            S::Case { items, .. } => {
                for it in items {
                    let (ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. }) =
                        it;
                    self.actions(body);
                }
            }
            S::For { body, .. }
            | S::While { body, .. }
            | S::Repeat { body, .. }
            | S::Forever { body, .. } => self.actions(body),
            S::DelayCtrl { body: Some(b), .. }
            | S::EventCtrl { body: Some(b), .. }
            | S::Wait { body: Some(b), .. } => self.actions(b),
            _ => {}
        }
    }

    fn routine(
        &mut self,
        name: &ast::Ident,
        ports: &[ast::TfPort],
        decls: &[ast::NetVarDecl],
        enums: &[ast::TypedefDecl],
        body: &ast::Stmt,
    ) {
        self.names.insert(name.name.clone());
        for p in ports {
            self.names.insert(p.name.name.clone());
        }
        for td in enums {
            self.enum_labels(td);
        }
        for d in decls {
            for n in &d.names {
                self.names.insert(n.name.name.clone());
            }
        }
        self.blocks(body);
    }

    /// The module-scope expressions of a top-level generate construct.
    fn gen_header(&mut self, gi: &ast::GenItem) {
        let mut h = |e: &ast::Expr| self.headers.push((e.span.lo, e.span.hi));
        match gi {
            ast::GenItem::If { cond, .. } => h(cond),
            ast::GenItem::Case {
                scrutinee, items, ..
            } => {
                h(scrutinee);
                for it in items {
                    if let ast::GenCaseItem::Match { labels, .. } = it {
                        labels.iter().for_each(&mut h);
                    }
                }
            }
            ast::GenItem::For {
                init, cond, step, ..
            } => {
                h(&init.value);
                h(cond);
                h(&step.value);
            }
            ast::GenItem::Block { .. } | ast::GenItem::Item(_) => {}
        }
    }

    fn gen_item(&mut self, gi: &ast::GenItem) {
        match gi {
            ast::GenItem::For { init, body, .. } => {
                self.names.insert(init.lvalue.name.clone());
                for b in body {
                    self.gen_item(b);
                }
            }
            ast::GenItem::If { then_b, else_b, .. } => {
                for b in then_b.iter().chain(else_b) {
                    self.gen_item(b);
                }
            }
            ast::GenItem::Case { items, .. } => {
                for it in items {
                    let (ast::GenCaseItem::Match { body, .. }
                    | ast::GenCaseItem::Default { body, .. }) = it;
                    for b in body {
                        self.gen_item(b);
                    }
                }
            }
            ast::GenItem::Block { items, .. } => {
                for b in items {
                    self.gen_item(b);
                }
            }
            ast::GenItem::Item(mi) => self.gen_module_item(mi),
        }
    }

    /// A declaration inside a generate construct: every name it binds.
    fn gen_module_item(&mut self, mi: &ast::ModuleItem) {
        match mi {
            ast::ModuleItem::NetVar(d) => {
                for n in &d.names {
                    self.names.insert(n.name.name.clone());
                }
            }
            ast::ModuleItem::Param(p) => {
                self.names.insert(p.name.name.clone());
            }
            ast::ModuleItem::Genvar { names, .. } => {
                for n in names {
                    self.names.insert(n.name.clone());
                }
            }
            ast::ModuleItem::Instance(inst) => {
                for i in &inst.instances {
                    self.names.insert(i.name.name.clone());
                }
            }
            ast::ModuleItem::Func(f) => {
                self.routine(&f.name, &f.ports, &f.body_decls, &f.body_enums, &f.body)
            }
            ast::ModuleItem::Task(t) => {
                self.routine(&t.name, &t.ports, &t.body_decls, &t.body_enums, &t.body)
            }
            ast::ModuleItem::Typedef(td) => self.enum_labels(td),
            ast::ModuleItem::Proc(p) => self.blocks(&p.body),
            ast::ModuleItem::Generate(g) => {
                for gi in &g.items {
                    self.gen_item(gi);
                }
            }
            _ => {}
        }
    }

    /// Is a use at `span` in the module's own text and outside every nested scope
    /// (or in a top-level generate construct's header)?
    fn top_level(&self, span: ast::Span) -> bool {
        let within = |&(a, b): &(u32, u32)| span.lo >= a && span.hi <= b;
        span.lo != 0
            && span.lo >= self.lo
            && span.hi <= self.hi
            && (self.headers.iter().any(within) || !self.nested.iter().any(within))
    }

    /// The use at `span` names `root` from the module's top level, and no nested
    /// scope of the module declares `root`.
    pub(crate) fn admits(&self, span: ast::Span, root: &str) -> bool {
        self.top_level(span) && !self.names.contains(root)
    }

    /// `root` is an ANSI port the census saw as a packed vector.
    pub(crate) fn vector_port(&self, root: &str) -> bool {
        self.vector_ports.contains(root)
    }
}

impl Elaborator<'_> {
    /// Is `e`, the base of a select, a packed integral VECTOR the constant domain
    /// resolves the way [`Self::bits_of_view`] does — a parameter or genvar, or a
    /// body variable with at most one packed dimension whose every unpacked
    /// dimension is indexed away (the prescan first, then the net table), or a
    /// select of one? A whole or partly indexed unpacked array, a multi-dimensional
    /// packed array (its select is an element), a string, a `real` / `event`, a
    /// formal, a hierarchical or package name, and every other shape answer
    /// `false`, and the `$bits` stays with the resolvers that answered before.
    pub(crate) fn select_base_is_packed(&self, e: &ast::Expr, use_span: ast::Span) -> bool {
        match &e.kind {
            ast::ExprKind::PartSelect { base, .. } | ast::ExprKind::IndexedPart { base, .. } => {
                self.select_base_is_packed(base, use_span)
            }
            // A bit-select chain over a name, parenthesised or not.
            _ => {
                let Some((root, depth)) = ident_index_chain(e) else {
                    return false;
                };
                let Some(sc) = self.select_scope.as_ref() else {
                    return false;
                };
                if !sc.admits(use_span, root) || !self.folding_in_module_scope() {
                    return false;
                }
                if self.subst_lookup(root).is_some() || self.out_subst_lookup(root).is_some() {
                    return false;
                }
                if self.lookup_scoped(root).is_some() {
                    return depth == 0;
                }
                if let Some((_, dims, bit_selectable, _)) = self.bits_prescan.get(root) {
                    return *bit_selectable && depth == dims.len();
                }
                // A header port is in no prescan; the census saw its declared shape.
                if sc.vector_port(root) {
                    return depth == 0;
                }
                let Some(net) = self.lookup_net_scoped(root) else {
                    return false;
                };
                let Some(nv) = self.nets.get(net as usize) else {
                    return false;
                };
                if !matches!(
                    nv.kind,
                    ir::NetKind::Wire
                        | ir::NetKind::Reg
                        | ir::NetKind::Logic
                        | ir::NetKind::Integer
                ) {
                    return false;
                }
                // One packed dimension at most: a select of `logic [3:0][7:0] p` is an
                // element of 8 bits, not a bit.
                let Some((dims, unpacked)) = self.net_dims_desc(net) else {
                    return false;
                };
                dims.len() - unpacked <= 1 && depth == unpacked
            }
        }
    }

    /// Is the fold running in the module's own scope — the current prefix the
    /// instance's, not a generate block's or a routine's below it?
    fn folding_in_module_scope(&self) -> bool {
        !self.inst_prefix.is_empty() && self.cur_prefix == self.inst_prefix
    }

    /// May the signing arms size `$signed(op)` / `$unsigned(op)` at `use_span`? At
    /// the module's top level only (`select_scope.rs`), never over a nested scope's
    /// name, and only over an integral operand: `$signed` takes no `real`,
    /// `realtime` or `event`, nor a name this rule cannot place.
    pub(crate) fn signing_operand_admitted(&self, op: &ast::Expr, use_span: ast::Span) -> bool {
        let Some(sc) = self.select_scope.as_ref() else {
            return false;
        };
        if !self.folding_in_module_scope() {
            return false;
        }
        let Some((root, _)) = ident_index_chain(op) else {
            // Not a name or a select chain (a concatenation, a literal): its own
            // parts answer through the arms above, which apply the same rule.
            return sc.admits(use_span, "");
        };
        if !sc.admits(use_span, root)
            || self.subst_lookup(root).is_some()
            || self.out_subst_lookup(root).is_some()
        {
            return false;
        }
        if self.lookup_scoped(root).is_some() {
            return self.walk_scopes(root, &self.real_param_val).is_none();
        }
        if let Some(p) = self.bits_prescan.get(root) {
            return !p.3;
        }
        if sc.vector_port(root) {
            return true;
        }
        self.lookup_net_scoped(root)
            .and_then(|n| self.nets.get(n as usize))
            .is_some_and(|nv| {
                matches!(
                    nv.kind,
                    ir::NetKind::Wire
                        | ir::NetKind::Reg
                        | ir::NetKind::Logic
                        | ir::NetKind::Integer
                )
            })
    }
}

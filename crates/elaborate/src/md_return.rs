//! §3 ⑤ⓖ: a SELECT on the result of a call to a function whose return type has
//! more than one packed dimension.
//!
//! The parser declares such a return FLAT (`range` is the product of the dims, and
//! `FunctionDef::ret_packed` keeps them), so `f(x)[1]` — a select on a call, which
//! vita accepts as an extension (W2004) — would select BIT 1 of the flat value,
//! where verilator, which accepts the same spelling, names the outer ELEMENT 1
//! (`typedef logic [2:0][3:0] t`, `f(12'h5a3)[1]`: vita `1`, verilator `a`).
//! iverilog rejects every select on a call.
//!
//! The element needs the callee's dims at the select, and a call reaches its callee
//! along several routes — a module or generate-scoped function, a package function,
//! a class method, a hierarchical call bound only after every instance exists — so
//! the refusal is keyed on the NAME: every function in the design whose return has
//! more than one packed dimension lends its name, and a select on a call of that name
//! is loud. A same-named function with a one-dimensional return is refused with it;
//! every design containing such a return was a parse error before the dims had a
//! slot, so nothing that ran before is refused now.
use super::*;

/// The names of every function in `unit` whose return type has more than one packed
/// dimension. A `FunctionDef` sits in exactly two AST containers, `ModuleItem::Func`
/// and `ClassItem::Func`; this walks every place either can occur — module,
/// interface and package bodies, generate constructs at any depth, and classes at
/// the top level or inside a module.
pub(crate) fn md_return_fn_names(unit: &ast::SourceUnit) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for it in &unit.items {
        match it {
            ast::TopItem::Module(m) | ast::TopItem::Interface(m) | ast::TopItem::Package(m) => {
                module_items(&m.body, &mut out)
            }
            ast::TopItem::Class(c) => class_items(c, &mut out),
            ast::TopItem::Import(_)
            | ast::TopItem::Bind(_)
            | ast::TopItem::Error(_)
            | ast::TopItem::InsideNameUse(_) => {}
        }
    }
    out
}

fn note(f: &ast::FunctionDef, out: &mut BTreeSet<String>) {
    if !f.ret_packed.is_empty() {
        out.insert(f.name.name.clone());
    }
}

// Every match below is exhaustive on purpose: a new container variant must be
// decided here, not fall through a catch-all.
fn module_items(items: &[ast::ModuleItem], out: &mut BTreeSet<String>) {
    use ast::ModuleItem as M;
    for it in items {
        match it {
            M::Func(f) => note(f, out),
            M::Generate(g) => gen_items(&g.items, out),
            M::Class(c) => class_items(c, out),
            M::NetVar(_)
            | M::Param(_)
            | M::PortDecl(_)
            | M::ContAssign(_)
            | M::Proc(_)
            | M::Instance(_)
            | M::Genvar { .. }
            | M::Task(_)
            | M::Defparam(_)
            | M::Typedef(_)
            | M::Import(_)
            | M::Modport(_)
            | M::SequenceDecl(_)
            | M::PropertyDecl(_)
            | M::Covergroup(_)
            | M::CoverInstance(_)
            | M::LetDecl(_)
            | M::DefaultDisableIff(_)
            | M::Clocking(_)
            | M::Error(_) => {}
        }
    }
}

fn gen_items(items: &[ast::GenItem], out: &mut BTreeSet<String>) {
    for it in items {
        match it {
            ast::GenItem::For { body, .. } => gen_items(body, out),
            ast::GenItem::If { then_b, else_b, .. } => {
                gen_items(then_b, out);
                gen_items(else_b, out);
            }
            ast::GenItem::Case { items, .. } => {
                for ci in items {
                    match ci {
                        ast::GenCaseItem::Match { body, .. }
                        | ast::GenCaseItem::Default { body, .. } => gen_items(body, out),
                    }
                }
            }
            ast::GenItem::Block { items, .. } => gen_items(items, out),
            ast::GenItem::Item(mi) => module_items(std::slice::from_ref(&**mi), out),
        }
    }
}

fn class_items(c: &ast::ClassDecl, out: &mut BTreeSet<String>) {
    for it in &c.items {
        match it {
            ast::ClassItem::Func { def, .. } => note(def, out),
            ast::ClassItem::Property(..)
            | ast::ClassItem::RandProperty { .. }
            | ast::ClassItem::Constraint(_)
            | ast::ClassItem::Task { .. }
            | ast::ClassItem::Error(_) => {}
        }
    }
}

impl Elaborator<'_> {
    /// Is `base` — the BASE of an enclosing select — a call (parentheses peeled) of a
    /// function named in `md_return_fns`? Reports and returns `true` when it is, so the
    /// caller returns a placeholder instead of selecting from the flat value. Sits
    /// beside [`Self::reject_dyn_md_elem_select`], the same refusal for a heap element.
    pub(crate) fn reject_md_return_call_select(&mut self, base: &ast::Expr) -> bool {
        if self.md_return_fns.is_empty() {
            return false;
        }
        let Some(n) = self.md_return_callee(base, &mut Vec::new()) else {
            return false;
        };
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "a select on the result of a call to `{n}`, whose return type has more than \
                 one packed dimension, is unsupported in v1 (assign the result to a variable \
                 of that type, then select)"
            ),
        );
        true
    }

    /// The md-return callee name whose flat value `base` would select from, or `None`.
    /// Parentheses are peeled; a `?:` answers for either arm (verilator keeps the
    /// element type when both arms carry it, and refusing the mixed case is only
    /// loud); a call the `Call` arm routes to a `let`, and a bare name the `Ident` arm
    /// routes to a zero-argument `let` (the same conditions each arm tests), is
    /// expanded exactly as those arms expand it — the body with the formals replaced by
    /// the arguments — and the expansion is checked. `visiting` holds the lets being
    /// expanded: a let already on it is recursive, which the expansion refuses.
    fn md_return_callee(&self, base: &ast::Expr, visiting: &mut Vec<String>) -> Option<String> {
        let mut b = base;
        while let ast::ExprKind::Paren { inner } = &b.kind {
            b = inner;
        }
        match &b.kind {
            ast::ExprKind::Ternary { then_e, else_e, .. } => self
                .md_return_callee(then_e, visiting)
                .or_else(|| self.md_return_callee(else_e, visiting)),
            ast::ExprKind::Call { name, args } => {
                if name.segments.len() == 1 {
                    let n = &name.segments[0].name;
                    if self.let_table.contains_key(n) && !self.has_func(n) && !self.has_task(n) {
                        return self.let_expansion_callee(n, args, visiting);
                    }
                }
                let n = &name.segments.last()?.name;
                self.md_return_fns.contains(n).then(|| n.clone())
            }
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                let n = &path.segments[0].name;
                if self.let_table.contains_key(n)
                    && self.lookup_net_scoped(n).is_none()
                    && !self.has_func(n)
                    && !self.has_task(n)
                {
                    return self.let_expansion_callee(n, &[], visiting);
                }
                None
            }
            ast::ExprKind::MethodCall { method, .. } => self
                .md_return_fns
                .contains(&method.name)
                .then(|| method.name.clone()),
            _ => None,
        }
    }

    fn let_expansion_callee(
        &self,
        name: &str,
        args: &[ast::Expr],
        visiting: &mut Vec<String>,
    ) -> Option<String> {
        let decl = self.let_table.get(name)?;
        if visiting.iter().any(|v| v == name) || decl.formals.len() != args.len() {
            return None;
        }
        let body = if decl.formals.is_empty() {
            decl.body.clone()
        } else {
            subst_expr(&decl.body, &sva_formal_map(&decl.formals, args))
        };
        visiting.push(name.to_string());
        let found = self.md_return_callee(&body, visiting);
        visiting.pop();
        found
    }
}

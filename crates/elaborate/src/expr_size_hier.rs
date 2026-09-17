//! Static resolution of a HIERARCHICAL leaf inside a §11.6.1 width region.
//!
//! A cross-instance read (`u.hs`, `m.u2.hs`, `u.arr[0]`) and a cross-instance call
//! (`u.hf(x)`) are PLACEHOLDERS while the referring body is lowered — the child
//! instance is elaborated afterwards (`elaborate_instance` step 8), so no net and
//! no `FuncId` exists yet and `ir_bits_of` answers `None`. The size-cast /
//! inline-body / case region walks in [`super::expr_size_ctx`] therefore treated
//! every such leaf as OPAQUE and either declined the region or sized the leaf from
//! a fabricated 32.
//!
//! The DECLARATION is available, though: the instance names a module, the module
//! declares the net or function, and the declared sign and width are what §11.8.1
//! asks for. This module answers exactly that, from a per-module fact table built
//! once from the AST, and only for the shapes where the answer cannot be anything
//! else. Everything else declines and the region keeps its pre-slice lowering.
//!
//! What is deliberately NOT resolved (each one keeps the pre-slice behaviour):
//! a range that does not fold to literals (a parameter width, overridden or not),
//! a name reached through a generate scope, an instance array, an interface, a
//! class member, an upward reference, a non-ANSI port, a name the child also
//! declares as a block local (v1 flattens those onto the module net of the same
//! bare name), and any design that uses `bind` (a bind can inject a scope name
//! into a module this walk reads as instance-free).

use super::*;

/// What a child module's declaration says about one net, for the two questions
/// the region walks ask: §11.8.1's extension sign, and Table 11-21's self width.
///
/// `width` is the width of ONE element — the whole net when `dims == 0`, the
/// element when the name is an unpacked array (`dims` selects are a value, fewer
/// are not, more are a bit; see [`Elaborator::select_chain`]).
#[derive(Clone, Copy)]
pub(crate) struct HierNet {
    pub(crate) signed: bool,
    pub(crate) width: u32,
    pub(crate) dims: u32,
}

/// One module's static surface, as far as a hierarchical leaf can reach it.
///
/// Every entry is UNAMBIGUOUS by construction: a name declared more than once in
/// the module — through a body declaration and a port declaration, through a
/// generate block, or as a procedural block local (v1 flattens those to a module
/// net of the same bare name) — is in none of these maps, so a query on it
/// declines rather than answering from the declaration this walk happened to see.
#[derive(Default)]
pub(crate) struct ModuleFacts {
    /// instance name → instantiated module name (plain body instances only)
    insts: BTreeMap<String, String>,
    /// net name → its declared shape
    nets: BTreeMap<String, HierNet>,
    /// function name → (declared return width, declared return sign)
    funcs: BTreeMap<String, (u32, bool)>,
}

/// Build the per-module fact table once, in declaration order. First declaration
/// wins on a duplicate module name, matching [`crate::build_module_map`].
pub(crate) fn build_module_facts(order: &[&ast::ModuleDecl]) -> BTreeMap<String, ModuleFacts> {
    let mut out: BTreeMap<String, ModuleFacts> = BTreeMap::new();
    for m in order {
        if out.contains_key(&m.name.name) {
            continue;
        }
        out.insert(m.name.name.clone(), module_facts(m));
    }
    out
}

fn module_facts(m: &ast::ModuleDecl) -> ModuleFacts {
    let mut seen: BTreeMap<String, u32> = BTreeMap::new();
    for p in &m.params {
        note(&mut seen, &p.name.name);
    }
    match &m.ports {
        ast::PortList::Ansi(ports) => {
            for p in ports {
                note(&mut seen, &p.name.name);
            }
        }
        ast::PortList::NonAnsi(names) => {
            for n in names {
                note(&mut seen, &n.name);
            }
        }
        ast::PortList::None => {}
    }
    for it in &m.body {
        census_item(it, &mut seen);
    }

    let mut f = ModuleFacts::default();
    let unique = |seen: &BTreeMap<String, u32>, n: &str| seen.get(n) == Some(&1);
    if let ast::PortList::Ansi(ports) = &m.ports {
        for p in ports {
            if p.iface.is_some() || !unique(&seen, &p.name.name) {
                continue;
            }
            if let Some(h) = net_shape(
                p.net_or_var.unwrap_or(ast::NetVarKind::Wire),
                p.signed,
                p.range.as_ref(),
                &p.packed,
                &p.unpacked,
                p.shape_param.as_ref(),
                None,
            ) {
                f.nets.insert(p.name.name.clone(), h);
            }
        }
    }
    for it in &m.body {
        match it {
            ast::ModuleItem::NetVar(d) => {
                for n in &d.names {
                    if !unique(&seen, &n.name.name) {
                        continue;
                    }
                    if let Some(h) = net_shape(
                        d.kind,
                        d.signed,
                        d.range.as_ref(),
                        &d.packed,
                        &n.unpacked,
                        d.shape_param.as_ref(),
                        d.class_type.as_ref(),
                    ) {
                        f.nets.insert(n.name.name.clone(), h);
                    }
                }
            }
            ast::ModuleItem::Instance(mi) => {
                for ii in &mi.instances {
                    if ii.unpacked.is_empty() && unique(&seen, &ii.name.name) {
                        f.insts
                            .insert(ii.name.name.clone(), mi.module_name.name.clone());
                    }
                }
            }
            ast::ModuleItem::Func(fd) => {
                if !unique(&seen, &fd.name.name) {
                    continue;
                }
                if let Some(ws) = func_ret_shape(fd) {
                    f.funcs.insert(fd.name.name.clone(), ws);
                }
            }
            _ => {}
        }
    }
    f
}

fn note(seen: &mut BTreeMap<String, u32>, n: &str) {
    *seen.entry(n.to_string()).or_insert(0) += 1;
}

/// Count every name the module declares that a bare hierarchical segment could
/// bind to. Over-counting only costs a decline; MISSING one would let a query
/// answer from a declaration the design shadows, so generate bodies and
/// procedural block locals are walked too.
fn census_item(it: &ast::ModuleItem, seen: &mut BTreeMap<String, u32>) {
    match it {
        ast::ModuleItem::NetVar(d) => {
            for n in &d.names {
                note(seen, &n.name.name);
            }
        }
        ast::ModuleItem::Param(p) => note(seen, &p.name.name),
        ast::ModuleItem::PortDecl(pd) => {
            for n in &pd.names {
                note(seen, &n.name);
            }
        }
        ast::ModuleItem::Instance(mi) => {
            for ii in &mi.instances {
                note(seen, &ii.name.name);
            }
        }
        ast::ModuleItem::Genvar { names, .. } => {
            for n in names {
                note(seen, &n.name);
            }
        }
        ast::ModuleItem::Func(f) => note(seen, &f.name.name),
        ast::ModuleItem::Task(t) => note(seen, &t.name.name),
        ast::ModuleItem::Typedef(t) => note(seen, &t.name.name),
        ast::ModuleItem::Class(c) => note(seen, &c.name.name),
        ast::ModuleItem::Covergroup(c) => note(seen, &c.name.name),
        ast::ModuleItem::CoverInstance(c) => note(seen, &c.name.name),
        ast::ModuleItem::LetDecl(l) => note(seen, &l.name.name),
        ast::ModuleItem::Clocking(c) => {
            if let Some(n) = &c.name {
                note(seen, &n.name);
            }
        }
        // v1 flattens a procedural block local onto a module net of the same
        // BARE name, so such a name is not answerable from the module-level
        // declaration alone.
        ast::ModuleItem::Proc(p) => {
            let mut decls = Vec::new();
            collect_block_local_decls(&p.body, &mut decls);
            for d in &decls {
                for n in &d.names {
                    note(seen, &n.name.name);
                }
            }
        }
        ast::ModuleItem::Generate(g) => {
            for gi in &g.items {
                census_gen(gi, seen);
            }
        }
        _ => {}
    }
}

fn census_gen(gi: &ast::GenItem, seen: &mut BTreeMap<String, u32>) {
    let label = |l: &Option<ast::Ident>, seen: &mut BTreeMap<String, u32>| {
        if let Some(n) = l {
            note(seen, &n.name);
        }
    };
    match gi {
        ast::GenItem::For { label: l, body, .. } => {
            label(l, seen);
            for g in body {
                census_gen(g, seen);
            }
        }
        ast::GenItem::If {
            label: l,
            then_b,
            else_b,
            ..
        } => {
            label(l, seen);
            for g in then_b.iter().chain(else_b) {
                census_gen(g, seen);
            }
        }
        ast::GenItem::Block {
            label: l, items, ..
        } => {
            label(l, seen);
            for g in items {
                census_gen(g, seen);
            }
        }
        ast::GenItem::Case { items, .. } => {
            for ci in items {
                let body = match ci {
                    ast::GenCaseItem::Match { body, .. } => body,
                    ast::GenCaseItem::Default { body, .. } => body,
                };
                for g in body {
                    census_gen(g, seen);
                }
            }
        }
        ast::GenItem::Item(b) => census_item(b, seen),
    }
}

/// The declared `(sign, element width, unpacked-dim count)` of one net, or `None`
/// where the answer would be a guess. `ast_kind_range_width` is the repository's
/// rule for "a width the AST states without a scope": it folds DECIMAL LITERAL
/// bounds only, so `[W-1:0]` declines whether or not the instantiation overrides
/// `W` — the parameter environment of another module is not resolvable here.
fn net_shape(
    kind: ast::NetVarKind,
    signed: bool,
    range: Option<&ast::Range>,
    packed: &[ast::Range],
    unpacked: &[ast::Dim],
    shape_param: Option<&ast::Ident>,
    class_type: Option<&ast::Ident>,
) -> Option<HierNet> {
    // A `parameter type` carrier and a class handle carry no bit width here; a
    // SECOND packed dimension makes the whole-name read a flattened vector and
    // the select rules a different question, so both decline.
    if shape_param.is_some() || class_type.is_some() || !packed.is_empty() {
        return None;
    }
    let width = ast_kind_range_width(kind, range)?;
    if width == 0 {
        return None;
    }
    let mut dims = 0u32;
    for d in unpacked {
        match d {
            // A FIXED unpacked dimension: its bounds decide which element an
            // index names, which the engine answers at run time — only the
            // COUNT of dimensions is needed here.
            ast::Dim::Range(_) | ast::Dim::Size(_) => dims += 1,
            // dynamic / queue / associative: no static element geometry
            _ => return None,
        }
    }
    Some(HierNet {
        signed: crate::array_geom::kind_signedness(kind, signed),
        width,
        dims,
    })
}

/// The declared `(return width, return sign)` of a function — the same rule the
/// bare-call arm of `ctx_signed_impl` applies (`ast_func_return_width` for the
/// width, `kind_signedness` for the sign). A `real`/`realtime`/`string` return is
/// not a bit vector and declines.
fn func_ret_shape(f: &ast::FunctionDef) -> Option<(u32, bool)> {
    if f.ret_string {
        return None;
    }
    let kind = match f.ret_type {
        ast::ParamType::Integer => ast::NetVarKind::Integer,
        ast::ParamType::Real | ast::ParamType::Realtime => return None,
        ast::ParamType::Time => ast::NetVarKind::Time,
        ast::ParamType::Implicit => ast::NetVarKind::Reg,
    };
    let w = ast_func_return_width(f)?;
    (w > 0).then(|| (w, crate::array_geom::kind_signedness(kind, f.signed)))
}

impl Elaborator<'_> {
    /// The module a hierarchical path's INSTANCE segments name, walked DOWNWARD
    /// from the module being lowered.
    ///
    /// Downward-only is what makes this agree with `hier_resolve`, the resolver
    /// that patches the placeholder later: that walk commits to the INNERMOST
    /// enclosing scope in which the leading segment is found, and for a body
    /// instance of the module being lowered that is this very instance.
    ///
    /// The guards keep the walk inside the case where "innermost" and "this
    /// module's body" are the same thing. A reference lowered in a GENERATE body
    /// may see a generate-local scope first (and `with_rtn_decl_scope` lowers a
    /// §27.3 routine's body in that scope with `in_generate_body` already false,
    /// which is why the prefix is tested too). A `cur_prefix` that has moved away
    /// from `inst_prefix` for any other reason — an interface body, a package
    /// pre-sweep, a covergroup scope — is not this module's body at all.
    ///
    /// The one accepted extension is a run of SYNTHESIZED scope segments
    /// (`$func$…`, `$blk$…`, a class method's): a subroutine or named-block scope
    /// holds nets, never instances, so the outward walk in `hier_resolve` passes
    /// straight through it to this module's body. A leading segment that names a
    /// net in one of them commits `hier_resolve` to "`.member` on a plain net",
    /// which is loud — never a different silent answer.
    fn hier_leaf_scope(&self, insts: &[ast::Ident]) -> Option<&ModuleFacts> {
        // A `bind` attaches a child to a module without an instantiation in its
        // body, so the fact table's instance map is not the whole scope list.
        if !self.bind_targets.is_empty() || self.in_generate_body {
            return None;
        }
        let rest = self.cur_prefix.strip_prefix(self.inst_prefix.as_str())?;
        if !rest.is_empty()
            && !rest
                .strip_prefix('.')?
                .split('.')
                .all(|s| s.starts_with('$'))
        {
            return None;
        }
        let mut f = self.module_facts.get(&self.cur_module)?;
        for seg in insts {
            f = self.module_facts.get(f.insts.get(&seg.name)?)?;
        }
        Some(f)
    }

    /// The declaration a hierarchical NAME (`u.hs`, `m.u2.hs`, `u.arr`) resolves
    /// to, or `None` to keep the pre-slice behaviour. A single-segment path is not
    /// hierarchical and is answered by the ordinary name routes.
    pub(crate) fn hier_leaf_net(&self, path: &ast::HierPath) -> Option<HierNet> {
        let (leaf, insts) = path.segments.split_last()?;
        if insts.is_empty() {
            return None;
        }
        self.hier_leaf_scope(insts)?.nets.get(&leaf.name).copied()
    }

    /// The `(return width, return sign)` a hierarchical CALL (`u.hf(x)`) resolves
    /// to. The two-segment package spelling `pk::f(…)` is not a hierarchical call
    /// and never reaches here (`pkg_call_head` answers it first).
    pub(crate) fn hier_leaf_func(&self, path: &ast::HierPath) -> Option<(u32, bool)> {
        let (leaf, insts) = path.segments.split_last()?;
        if insts.is_empty() {
            return None;
        }
        self.hier_leaf_scope(insts)?.funcs.get(&leaf.name).copied()
    }

    /// The Table 11-21 self width of a PART-SELECT whose base is a hierarchical
    /// name (`u.hs[3:0]`). The bounds are constant expressions evaluated in the
    /// REFERRING scope (`const_eval_in_scope`, the same call the bare-name twin in
    /// `size_ctx_self_width` makes); the base only has to be a plain vector, which
    /// is what `hier_leaf_net` reports as `dims == 0`.
    pub(crate) fn hier_part_select_width(
        &self,
        base: &ast::Expr,
        msb: &ast::Expr,
        lsb: &ast::Expr,
    ) -> Option<u32> {
        let ast::ExprKind::Ident(p) = &base.kind else {
            return None;
        };
        if self.hier_leaf_net(p)?.dims != 0 {
            return None;
        }
        let m = self.const_eval_in_scope(msb)?;
        let l = self.const_eval_in_scope(lsb)?;
        u32::try_from(m.abs_diff(l) + 1).ok()
    }
}

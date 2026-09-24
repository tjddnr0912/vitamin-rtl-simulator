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
//! The DECLARATION is available, though: the instance names a module (or an
//! interface — `ifc w();` is the same shape, and its members `w.hi` are the same
//! kind of leaf), the module declares the net or function, and the declared sign
//! and width are what §11.8.1
//! asks for. This module answers exactly that, from a per-module fact table built
//! once from the AST, and only for the shapes where the answer cannot be anything
//! else. Everything else declines and the region keeps its pre-slice lowering.
//!
//! A PARAMETER width (`[W-1:0]`) is folded in the CHILD's parameter environment:
//! the child's defaults, the instance's `#()` overrides folded in the referring
//! scope (the same fold `elaborate_instance` makes), and the child's localparams,
//! in declaration order — one environment per instance segment of the path. The
//! fold is a strict i64 fold over the shapes whose 32-bit self-determined value
//! is the same number (`+ - *`, non-negative `/ %`, `$clog2`, a package
//! constant); anything else, a typed parameter, a fill override, a `defparam`
//! anywhere in the design, an unbound name, or a bound outside `0..=i32::MAX`
//! declines. A LITERAL range never needs the environment, so a design whose
//! environment cannot be built keeps every answer §4.5.507 gave.
//!
//! What is deliberately NOT resolved (each one keeps the pre-slice behaviour):
//! a range beyond the strict fold above (a `-G` override reaches only a root and
//! is read through the live scope), a name reached through a generate scope, an
//! instance array, an interface reached through a PORT or a modport (`p.hi`,
//! `w.mp.hi` — only a body instance `ifc w();` is in the instance map), a
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
/// What [`Elaborator::hier_body_write_callee`] answers for a hierarchical call
/// whose callee's body writes a module net.
pub(crate) struct HierBodyWriteCallee<'a> {
    pub(crate) def: &'a ast::FunctionDef,
    pub(crate) ret_width: u32,
    pub(crate) ret_signed: bool,
    /// One entry per formal, in port order (all inputs, all plain vectors).
    pub(crate) formal_widths: Vec<u32>,
}

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
    /// instance name → (instantiated module name, its `#()` overrides) — plain
    /// body instances only
    insts: BTreeMap<String, (String, Vec<ast::ParamConn>)>,
    /// net name → its declared shape, the width still unfolded
    nets: BTreeMap<String, NetFact>,
    /// function name → (declared return width unfolded, declared return sign)
    funcs: BTreeMap<String, (WidthFact, bool)>,
    /// function name → its declaration, for the functions whose BODY writes a
    /// module net (`ast_func_body_writes_outside`): the §3.b route decided
    /// from the AST, which is the only form of it a CALLING module can see
    /// while it is lowered (the callee's instance does not exist yet).
    body_write_funcs: BTreeMap<String, ast::FunctionDef>,
    /// The module's parameters in declaration order (header, then body), or
    /// `None` when no environment can be built for it: a parameter name the
    /// module declares twice, or a `defparam` anywhere in the design (a
    /// `defparam` rebinds a child parameter from outside the `#()` channel).
    params: Option<Vec<ParamSlot>>,
}

/// A declared width before the parameter environment is known.
#[derive(Clone)]
enum WidthFact {
    /// A width the declaration states without a scope (an atom kind, no range, a
    /// decimal-literal range) — §4.5.507's answer, needing no environment.
    Lit(u32),
    /// A vector range that names something; folded per instance by `fold_width`.
    Range(ast::Range),
}

#[derive(Clone)]
struct NetFact {
    signed: bool,
    width: WidthFact,
    dims: u32,
}

/// One parameter of a module, for building its environment.
#[derive(Clone)]
struct ParamSlot {
    name: String,
    /// Bindable through `#()` (a header `parameter`; a body `parameter` only when
    /// there is no header list — `param_ports`' rule).
    overridable: bool,
    /// `integer`-typed → the bound value is coerced to 32 signed bits; untyped →
    /// taken as folded; any other type leaves the name UNBOUND (a range reading
    /// it then declines).
    ty: Option<ast::ParamType>,
    default: ast::Expr,
}

/// A parameter environment: name → (folded value, `wide`), for one instance of a
/// module. `wide` = the value's self-determined width is at least 32 bits (an
/// unsized literal, `integer`, or arithmetic with such an operand). A NARROW
/// value (`3'd6`, or a name whose width the fold cannot see) wraps in the
/// binder's self-determined arithmetic — `P + P` with `P = 3'd6` binds 4 — so
/// arithmetic over narrow operands only is declined, never folded in i64.
type ParamEnv = BTreeMap<String, (i64, bool)>;

/// Where a fold reads a NAME: the live scope of the module being lowered (the
/// first instance segment's overrides), or a built environment (everything
/// deeper, and every default).
enum FoldScope<'a> {
    Live,
    Env(&'a ParamEnv),
}

/// Build the per-module fact table once, in declaration order: `modules` first,
/// then `ifaces` (an interface instance `ifc w();` names its declaration exactly
/// as a module instance does, and its parameters are bound by the same
/// `bind_params`). First declaration wins on a duplicate name, matching
/// [`crate::build_module_map`]; a module and an interface of one name is E2001.
pub(crate) fn build_module_facts(
    modules: &[&ast::ModuleDecl],
    ifaces: &[&ast::ModuleDecl],
) -> BTreeMap<String, ModuleFacts> {
    // A `defparam` is collected while the module that CONTAINS it is lowered,
    // which can be after the region that asks — so it is a property of the
    // whole design here, read from the AST, not of the elaborator's state.
    // MODULES only: a `defparam` inside an interface body is E3009 the moment
    // the interface is instantiated, and an uninstantiated interface binds
    // nothing, so scanning it would only drop every module's environment for a
    // design in which no parameter moves.
    let any_defparam = modules.iter().any(|m| {
        m.body.iter().any(|it| {
            matches!(it, ast::ModuleItem::Defparam(_))
                || matches!(it, ast::ModuleItem::Generate(g) if gen_has_defparam(&g.items))
        })
    });
    let mut out: BTreeMap<String, ModuleFacts> = BTreeMap::new();
    for m in modules.iter().chain(ifaces) {
        if out.contains_key(&m.name.name) {
            continue;
        }
        let mut f = module_facts(m);
        if any_defparam {
            f.params = None;
        }
        out.insert(m.name.name.clone(), f);
    }
    out
}

fn gen_has_defparam(items: &[ast::GenItem]) -> bool {
    items.iter().any(|gi| match gi {
        ast::GenItem::For { body, .. } => gen_has_defparam(body),
        ast::GenItem::If { then_b, else_b, .. } => {
            gen_has_defparam(then_b) || gen_has_defparam(else_b)
        }
        ast::GenItem::Block { items, .. } => gen_has_defparam(items),
        ast::GenItem::Case { items, .. } => items.iter().any(|ci| match ci {
            ast::GenCaseItem::Match { body, .. } | ast::GenCaseItem::Default { body, .. } => {
                gen_has_defparam(body)
            }
        }),
        ast::GenItem::Item(b) => matches!(&**b, ast::ModuleItem::Defparam(_)),
    })
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
    f.params = param_slots(m, &seen);
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
                        f.insts.insert(
                            ii.name.name.clone(),
                            (mi.module_name.name.clone(), mi.param_overrides.clone()),
                        );
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
                if crate::frames_classify_write::ast_func_body_writes_outside(fd) {
                    f.body_write_funcs.insert(fd.name.name.clone(), fd.clone());
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

/// The module's parameters in the order their defaults may reference each other:
/// the header list, then the body's `parameter`/`localparam` items. `None` when a
/// parameter name is declared more than once (a generate-local `localparam` of
/// the same name would make "which one" a scope question this walk cannot ask).
fn param_slots(m: &ast::ModuleDecl, seen: &BTreeMap<String, u32>) -> Option<Vec<ParamSlot>> {
    let has_header = !m.params.is_empty();
    let slot = |p: &ast::ParamDecl, overridable: bool| -> Option<ParamSlot> {
        if seen.get(&p.name.name) != Some(&1) {
            return None;
        }
        // `integer` / `int` parse as `Integer` + `signed`; `integer unsigned`,
        // a ranged or `signed` untyped declaration, and every 1-bit vector kind
        // (the parser gives `parameter logic P` a `[0:0]`) are width channels
        // this fold does not model, so the slot stays unbound. A bare
        // `parameter unsigned P` is recorded by the parser as plain untyped
        // (the keyword is dropped), so it binds exactly as the binder binds it.
        let ty = match (p.ty, p.range.is_some(), p.signed) {
            (ast::ParamType::Implicit, false, false) => Some(ast::ParamType::Implicit),
            (ast::ParamType::Integer, false, true) => Some(ast::ParamType::Integer),
            _ => None,
        };
        Some(ParamSlot {
            name: p.name.name.clone(),
            overridable: overridable && matches!(p.kind, ast::ParamKind::Parameter),
            ty,
            default: p.value.clone(),
        })
    };
    let mut out = Vec::new();
    for p in &m.params {
        out.push(slot(p, true)?);
    }
    for it in &m.body {
        if let ast::ModuleItem::Param(p) = it {
            out.push(slot(p, !has_header)?);
        }
    }
    Some(out)
}

/// The declared width of a vector kind + optional range, unfolded: an atom kind
/// and a decimal-literal range are `Lit` (§4.5.507's `ast_kind_range_width`); any
/// other range is kept for the per-instance fold.
fn width_fact(kind: ast::NetVarKind, range: Option<&ast::Range>) -> Option<WidthFact> {
    match ast_kind_range_width(kind, range) {
        Some(0) => None,
        Some(w) => Some(WidthFact::Lit(w)),
        None => {
            let r = range?;
            if !ast_kind_is_bit_vector(kind) || matches!(kind, ast::NetVarKind::Time) {
                return None;
            }
            Some(WidthFact::Range(r.clone()))
        }
    }
}

/// The declared `(sign, element width, unpacked-dim count)` of one net, or `None`
/// where the answer would be a guess.
fn net_shape(
    kind: ast::NetVarKind,
    signed: bool,
    range: Option<&ast::Range>,
    packed: &[ast::Range],
    unpacked: &[ast::Dim],
    shape_param: Option<&ast::Ident>,
    class_type: Option<&ast::Ident>,
) -> Option<NetFact> {
    // A `parameter type` carrier and a class handle carry no bit width here; a
    // SECOND packed dimension makes the whole-name read a flattened vector and
    // the select rules a different question, so both decline.
    if shape_param.is_some() || class_type.is_some() || !packed.is_empty() {
        return None;
    }
    let width = width_fact(kind, range)?;
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
    Some(NetFact {
        signed: crate::array_geom::kind_signedness(kind, signed),
        width,
        dims,
    })
}

/// The declared `(return width, return sign)` of a function — the same rule the
/// bare-call arm of `ctx_signed_impl` applies (`ast_func_return_width` for the
/// width, `kind_signedness` for the sign). A `real`/`realtime`/`string` return is
/// not a bit vector and declines.
fn func_ret_shape(f: &ast::FunctionDef) -> Option<(WidthFact, bool)> {
    if f.ret_string {
        return None;
    }
    let kind = match f.ret_type {
        ast::ParamType::Integer => ast::NetVarKind::Integer,
        ast::ParamType::Real | ast::ParamType::Realtime => return None,
        ast::ParamType::Time => ast::NetVarKind::Time,
        ast::ParamType::Implicit => ast::NetVarKind::Reg,
    };
    let w = match ast_func_return_width(f) {
        Some(0) => return None,
        Some(w) => WidthFact::Lit(w),
        None => WidthFact::Range(f.range.clone()?),
    };
    Some((w, crate::array_geom::kind_signedness(kind, f.signed)))
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
    fn hier_leaf_scope(&self, insts: &[ast::Ident]) -> Option<(&ModuleFacts, Option<ParamEnv>)> {
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
        // The environment of the instance the path has reached so far: `None`
        // for the module being lowered (its parameters are the LIVE scope), then
        // one built per segment. Building one can fail without failing the walk —
        // a literal-width leaf never reads it.
        let mut env: Option<ParamEnv> = None;
        let mut live = true;
        for seg in insts {
            let (module, overrides) = f.insts.get(&seg.name)?;
            let child = self.module_facts.get(module)?;
            env = if live || env.is_some() {
                self.child_env(child, overrides, env.as_ref())
            } else {
                None
            };
            live = false;
            f = child;
        }
        Some((f, env))
    }

    /// The parameter environment of ONE instance of `child`, given its `#()`
    /// overrides and the environment they are written in (`None` = the live
    /// scope of the module being lowered, folded exactly as `elaborate_instance`
    /// folds an override: `const_eval_in_scope`).
    fn child_env(
        &self,
        child: &ModuleFacts,
        overrides: &[ast::ParamConn],
        parent: Option<&ParamEnv>,
    ) -> Option<ParamEnv> {
        let slots = child.params.as_ref()?;
        let fold = |e: &ast::Expr| -> Option<(i64, bool)> {
            // A fill override (`#(.W('1))`) is re-folded at the CHILD's declared
            // width by the binder; this fold has no width to give it.
            if expr_as_fill(e).is_some() {
                return None;
            }
            match parent {
                None => self.env_fold(e, &FoldScope::Live),
                Some(env) => self.env_fold(e, &FoldScope::Env(env)),
            }
        };
        let positional: Vec<&ParamSlot> = slots.iter().filter(|s| s.overridable).collect();
        let mut bound: BTreeMap<&str, (i64, bool)> = BTreeMap::new();
        let mut pos_i = 0usize;
        for ov in overrides {
            let (name, value) = match ov {
                ast::ParamConn::Positional(e) => {
                    let s = positional.get(pos_i)?;
                    pos_i += 1;
                    (s.name.as_str(), fold(e)?)
                }
                ast::ParamConn::Named { name, value, .. } => {
                    let s = slots
                        .iter()
                        .find(|s| s.name == name.name)
                        .filter(|s| s.overridable)?;
                    (s.name.as_str(), fold(value.as_ref()?)?)
                }
            };
            // The binder's "last override wins" is a rule about a shape the
            // parser already reports; here a repeat is a decline.
            if bound.insert(name, value).is_some() {
                return None;
            }
        }
        let mut env = ParamEnv::new();
        for s in slots {
            let v = match bound.get(s.name.as_str()) {
                Some(v) => Some(*v),
                None => self.env_fold(&s.default, &FoldScope::Env(&env)),
            };
            let v = match (s.ty, v) {
                (Some(ast::ParamType::Integer), Some((v, _))) => {
                    (coerce_int_width(v, 32, true), true)
                }
                (Some(ast::ParamType::Implicit), Some(v)) => v,
                // A typed, wide, real or unfoldable parameter stays UNBOUND: a
                // range that reads it declines, a range that does not is
                // unaffected.
                _ => continue,
            };
            env.insert(s.name.clone(), v);
        }
        Some(env)
    }

    /// A strict fold of a constant expression: `(value, wide)`, or `None`. The
    /// value is the number the child's own 32-bit self-determined fold
    /// (`const_range_bound_fold`) and its binder give; where that is not
    /// guaranteed the fold declines rather than answering. The rules:
    ///
    ///  * every node lands in `i32::MIN..=i32::MAX` (a 32-bit ring op on values
    ///    that fit is the same number in i64);
    ///  * `+ - * / %` need at least one WIDE operand — the self-determined width
    ///    of the result is then ≥ 32 and nothing wraps; `/` and `%` also need
    ///    non-negative operands (a 32-bit unsigned quotient of a negative
    ///    pattern is a different number);
    ///  * a name resolves in `scope` only: in the built environment it is
    ///    `(value, wide)`; in the live scope it is the binder's own fold
    ///    (`const_eval_in_scope`) and NARROW, because its declared width is not
    ///    read here. A shape this fold does not know is treated the same way in
    ///    the live scope (the binder folds it with the same call) and declines
    ///    in an environment.
    fn env_fold(&self, e: &ast::Expr, scope: &FoldScope<'_>) -> Option<(i64, bool)> {
        let live = |e: &ast::Expr| -> Option<(i64, bool)> {
            match scope {
                FoldScope::Live => Some((self.const_eval_in_scope(e)?, false)),
                FoldScope::Env(_) => None,
            }
        };
        let (v, wide) = match &e.kind {
            ast::ExprKind::IntLit { kind, raw } => {
                let v = const_eval_i64_lit(e)?;
                let wide = match kind {
                    ast::IntLitKind::Decimal => true,
                    _ => parse_int_literal(raw, *kind).is_some_and(|cv| cv.width >= 32),
                };
                (v, wide)
            }
            ast::ExprKind::Paren { inner } => self.env_fold(inner, scope)?,
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => match scope {
                FoldScope::Env(env) => *env.get(&p.segments[0].name)?,
                FoldScope::Live => live(e)?,
            },
            // A package constant's declared width is not recorded in `pkg_consts`.
            ast::ExprKind::PkgScoped { pkg, name } => {
                (*self.pkg_consts.get(&pkg.name)?.get(&name.name)?, false)
            }
            ast::ExprKind::Unary {
                op: ast::UnOp::Minus,
                operand,
            } => {
                let (v, w) = self.env_fold(operand, scope)?;
                (v.checked_neg()?, w)
            }
            ast::ExprKind::Unary {
                op: ast::UnOp::Plus,
                operand,
            } => self.env_fold(operand, scope)?,
            ast::ExprKind::Binary {
                op:
                    op @ (ast::BinOp::Add
                    | ast::BinOp::Sub
                    | ast::BinOp::Mul
                    | ast::BinOp::Div
                    | ast::BinOp::Mod),
                lhs,
                rhs,
            } => {
                let (a, wa) = self.env_fold(lhs, scope)?;
                let (b, wb) = self.env_fold(rhs, scope)?;
                if !(wa || wb) {
                    return None;
                }
                if matches!(op, ast::BinOp::Div | ast::BinOp::Mod) && (a < 0 || b < 0) {
                    return None;
                }
                (const_binop(*op, a, b)?, true)
            }
            ast::ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                let (a, _) = self.env_fold(&args[0], scope)?;
                if a < 0 {
                    return None;
                }
                let v = if a <= 1 {
                    0
                } else {
                    i64::from(64 - (a - 1).leading_zeros())
                };
                (v, true)
            }
            _ => live(e)?,
        };
        (i64::from(i32::MIN)..=i64::from(i32::MAX))
            .contains(&v)
            .then_some((v, wide))
    }

    /// One declared width for one instance: a literal needs nothing; a range is
    /// folded in that instance's environment, both bounds in `0..=i32::MAX` and
    /// the width inside the net cap (the child would refuse it loudly otherwise).
    fn fold_width(&self, w: &WidthFact, env: Option<&ParamEnv>) -> Option<u32> {
        match w {
            WidthFact::Lit(n) => Some(*n),
            WidthFact::Range(r) => {
                let env = FoldScope::Env(env?);
                let (m, _) = self.env_fold(&r.msb, &env)?;
                let (l, _) = self.env_fold(&r.lsb, &env)?;
                if m < 0 || l < 0 {
                    return None;
                }
                let w = m.abs_diff(l) + 1;
                (w <= MAX_NET_WIDTH).then_some(w as u32)
            }
        }
    }

    /// The declaration a hierarchical NAME (`u.hs`, `m.u2.hs`, `u.arr`) resolves
    /// to, or `None` to keep the pre-slice behaviour. A single-segment path is not
    /// hierarchical and is answered by the ordinary name routes.
    /// The declared `(width, sign)` of a CLASS-FIELD read `obj.field` /
    /// `this.field`, resolved by `resolve_class_member` — the resolver
    /// `try_class_field_read` uses, and which `lower_expr` asks BEFORE the
    /// hierarchical route, so every walk that consults this does so first too.
    /// The read lowers to `Signal{net: <handle>, word: Some(fid)}` with the field's
    /// shape in `class_field_widths`, which `ir_bits_of` / `expr_self_signed` read,
    /// so `lower_size_leaf` resizes it by the same width this answers. `None` for a
    /// path that is not a class member, and for a handle-typed member.
    pub(crate) fn class_field_leaf(&self, path: &ast::HierPath) -> Option<(u32, bool)> {
        // A bare member inside a method body reaches these walks through
        // `bare_ident_route`, which does not resolve it; only the dotted spelling
        // is answered here.
        if path.segments.len() < 2 {
            return None;
        }
        let (_, class, field) = self.resolve_class_member(path)?;
        let (_, f) = self.class_field_id(&class, &field)?;
        // A handle member is an object id, not a bit-vector value.
        if f.class_type.is_some() {
            return None;
        }
        Some((f.width.max(1), f.signed))
    }

    pub(crate) fn hier_leaf_net(&self, path: &ast::HierPath) -> Option<HierNet> {
        let (leaf, insts) = path.segments.split_last()?;
        if insts.is_empty() {
            return None;
        }
        let (f, env) = self.hier_leaf_scope(insts)?;
        let n = f.nets.get(&leaf.name)?;
        Some(HierNet {
            signed: n.signed,
            width: self.fold_width(&n.width, env.as_ref())?,
            dims: n.dims,
        })
    }

    /// The `(return width, return sign)` a hierarchical CALL (`u.hf(x)`) resolves
    /// to. The two-segment package spelling `pk::f(…)` is not a hierarchical call
    /// and never reaches here (`pkg_call_head` answers it first).
    pub(crate) fn hier_leaf_func(&self, path: &ast::HierPath) -> Option<(u32, bool)> {
        let (leaf, insts) = path.segments.split_last()?;
        if insts.is_empty() {
            return None;
        }
        let (f, env) = self.hier_leaf_scope(insts)?;
        let (w, sg) = f.funcs.get(&leaf.name)?;
        Some((self.fold_width(w, env.as_ref())?, *sg))
    }

    /// §3.b: the callee of a hierarchical call `u.fw(x)` when that callee's BODY
    /// writes a module net, together with everything the CALLING module needs to
    /// emit the call as a statement before the child instance exists: the
    /// declaration (its ports, for the copy-in), the return shape (for the
    /// temp the expression reads instead), and each formal's width (for sizing
    /// the actual the way a local call does).
    ///
    /// Declines — leaving the call to `resolve_deferred_hier_call`, which
    /// refuses it by name — whenever an answer could be wrong: a path this
    /// walk cannot follow (outward, absolute, through a generate scope or an
    /// instance array, or under `bind`), a formal that is not an input vector
    /// (an output/inout, an unpacked array, a string, a `parameter type`
    /// carrier), a `real`/`string` return, or a width the instance's parameter
    /// environment cannot fold.
    pub(crate) fn hier_body_write_callee(
        &self,
        path: &ast::HierPath,
    ) -> Option<HierBodyWriteCallee<'_>> {
        let (leaf, insts) = path.segments.split_last()?;
        if insts.is_empty() {
            return None;
        }
        let (f, env) = self.hier_leaf_scope(insts)?;
        let def = f.body_write_funcs.get(&leaf.name)?;
        let (w, sg) = f.funcs.get(&leaf.name)?;
        let ret_width = self.fold_width(w, env.as_ref())?;
        let mut formal_widths = Vec::with_capacity(def.ports.len());
        for p in &def.ports {
            if !matches!(p.dir, ast::PortDir::Input) {
                return None;
            }
            let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
            if matches!(kind, ast::NetVarKind::String) {
                return None;
            }
            let h = net_shape(
                kind,
                p.signed,
                p.range.as_ref(),
                &[],
                &p.unpacked,
                p.shape_param.as_ref(),
                None,
            )?;
            if h.dims != 0 {
                return None;
            }
            formal_widths.push(self.fold_width(&h.width, env.as_ref())?);
        }
        Some(HierBodyWriteCallee {
            def,
            ret_width,
            ret_signed: *sg,
            formal_widths,
        })
    }

    /// Does any module in the design declare a function whose body writes a
    /// module net? The gate on the hoist pre-pass for a HIERARCHICAL call to
    /// one: a design without such a function never enters it (byte-identical).
    pub(crate) fn facts_have_body_write_funcs(&self) -> bool {
        self.module_facts
            .values()
            .any(|f| !f.body_write_funcs.is_empty())
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

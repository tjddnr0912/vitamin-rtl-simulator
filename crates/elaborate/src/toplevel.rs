//! module map / root picking — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// module-name → (decl, declaration index). `BTreeMap` so any iteration over the
/// map is deterministic; the decl index is the tie-break for top selection.
pub(crate) type ModuleMap<'a> = BTreeMap<&'a str, (&'a ast::ModuleDecl, usize)>;

/// How to find each child-port's connection expr, resolved in the PARENT scope.
/// Borrows directly from the `ast::ModuleInstance` so no per-port allocation.
pub(crate) enum PortBinding<'a> {
    None,                                // the top instance — no incoming bindings
    Named(&'a [ast::PortConn], bool),    // .p(expr); the bool is the `.*` wildcard
    Positional(&'a [Option<ast::Expr>]), // (expr, expr, …) with skip slots
}

/// A parameter override resolved to a value IN THE PARENT SCOPE before it is
/// pushed into the child. `name` is `Some` for `.W(v)` (named) / `None` for a
/// positional `#(v)` (bound to the child's i-th param by position). `value` is
/// `None` when the override expr did not const-fold (caller warns; child keeps
/// its default). Resolving here — not in `bind_params` — is what lets
/// `child #(.W(PARENT_W))` see the parent's `PARENT_W` (Fix 1 / Finding M1).
#[derive(Clone)]
pub(crate) struct ResolvedOverride {
    pub(crate) name: Option<String>,
    pub(crate) value: Option<i64>,
    pub(crate) is_named: bool,
    /// Set when the override expression IS an unsized fill literal (`'1`/`'0`/…).
    /// Its width is the CHILD param's declared width — unknown here in the parent
    /// — so resolution defers sizing to `bind_params`, which re-folds the fill at
    /// the param width (else `#(.P('1))` would silently truncate to 32 bits).
    pub(crate) fill: Option<(ast::IntLitKind, String)>,
}

/// Build the module-name map + the declaration-ordered list. First decl wins on a
/// duplicate name (caller warns). Deterministic: single pass over `unit.items`.
pub(crate) fn build_module_map(unit: &ast::SourceUnit) -> (ModuleMap<'_>, Vec<&ast::ModuleDecl>) {
    let mut map: ModuleMap<'_> = BTreeMap::new();
    let mut order: Vec<&ast::ModuleDecl> = Vec::new();
    for it in &unit.items {
        if let ast::TopItem::Module(m) = it {
            let idx = order.len();
            map.entry(m.name.name.as_str()).or_insert((m, idx));
            order.push(m);
        }
    }
    (map, order)
}

/// Collect every module name instantiated ANYWHERE in `order` — directly in a
/// module body OR nested inside a `generate` construct — restricted to names that
/// resolve to a known module (an unknown name is an instantiation error surfaced
/// later in the recursion). The set of modules NOT in here is the ROOT set.
/// Descending generates is load-bearing: a module instantiated ONLY inside a
/// `generate` is still instantiated, so it must not also be elaborated as a
/// spurious extra root (which would double-lower its body). Deterministic
/// (declaration-order walk into a `BTreeSet`).
pub(crate) fn collect_instantiated<'a>(
    map: &ModuleMap<'a>,
    order: &[&'a ast::ModuleDecl],
) -> std::collections::BTreeSet<&'a str> {
    fn from_item<'a>(
        item: &'a ast::ModuleItem,
        map: &ModuleMap<'a>,
        set: &mut std::collections::BTreeSet<&'a str>,
    ) {
        match item {
            ast::ModuleItem::Instance(inst) => {
                if map.contains_key(inst.module_name.name.as_str()) {
                    set.insert(inst.module_name.name.as_str());
                }
            }
            ast::ModuleItem::Generate(g) => {
                for gi in &g.items {
                    from_genitem(gi, map, set);
                }
            }
            _ => {}
        }
    }
    fn from_genitem<'a>(
        gi: &'a ast::GenItem,
        map: &ModuleMap<'a>,
        set: &mut std::collections::BTreeSet<&'a str>,
    ) {
        match gi {
            ast::GenItem::Item(boxed) => from_item(boxed, map, set),
            ast::GenItem::For { body, .. } => {
                for g in body {
                    from_genitem(g, map, set);
                }
            }
            ast::GenItem::Block { items, .. } => {
                for g in items {
                    from_genitem(g, map, set);
                }
            }
            ast::GenItem::If { then_b, else_b, .. } => {
                for g in then_b.iter().chain(else_b) {
                    from_genitem(g, map, set);
                }
            }
            ast::GenItem::Case { items, .. } => {
                for ci in items {
                    let body = match ci {
                        ast::GenCaseItem::Match { body, .. } => body,
                        ast::GenCaseItem::Default { body, .. } => body,
                    };
                    for g in body {
                        from_genitem(g, map, set);
                    }
                }
            }
        }
    }
    let mut set = std::collections::BTreeSet::new();
    for m in order {
        for item in &m.body {
            from_item(item, map, &mut set);
        }
    }
    set
}

/// Collect every design-unit name instantiated anywhere in `unit` — directly in
/// a module body or nested inside `generate` — WITHOUT resolving against a
/// module map (unresolved names are exactly what a worklib closure walk needs:
/// they may live in another compilation unit). Interface instances surface here
/// too (they parse as `ModuleItem::Instance`). Deterministic (decl-order walk
/// into a `BTreeSet`).
pub fn instantiated_names(unit: &ast::SourceUnit) -> std::collections::BTreeSet<String> {
    fn from_item(item: &ast::ModuleItem, set: &mut std::collections::BTreeSet<String>) {
        match item {
            ast::ModuleItem::Instance(inst) => {
                set.insert(inst.module_name.name.clone());
            }
            ast::ModuleItem::Generate(g) => {
                for gi in &g.items {
                    from_genitem(gi, set);
                }
            }
            _ => {}
        }
    }
    fn from_genitem(gi: &ast::GenItem, set: &mut std::collections::BTreeSet<String>) {
        match gi {
            ast::GenItem::Item(boxed) => from_item(boxed, set),
            ast::GenItem::For { body, .. } => {
                for g in body {
                    from_genitem(g, set);
                }
            }
            ast::GenItem::Block { items, .. } => {
                for g in items {
                    from_genitem(g, set);
                }
            }
            ast::GenItem::If { then_b, else_b, .. } => {
                for g in then_b.iter().chain(else_b) {
                    from_genitem(g, set);
                }
            }
            ast::GenItem::Case { items, .. } => {
                for ci in items {
                    let body = match ci {
                        ast::GenCaseItem::Match { body, .. } => body,
                        ast::GenCaseItem::Default { body, .. } => body,
                    };
                    for g in body {
                        from_genitem(g, set);
                    }
                }
            }
        }
    }
    let mut set = std::collections::BTreeSet::new();
    for it in &unit.items {
        let body = match it {
            ast::TopItem::Module(m) => &m.body,
            ast::TopItem::Interface(m) => &m.body,
            _ => continue,
        };
        for item in body {
            from_item(item, &mut set);
        }
    }
    set
}

/// Pick ALL TOP (root) modules: every module never instantiated by another, in
/// DECLARATION order (deterministic flat-IR layout). IEEE 1364 / iverilog
/// elaborate every uninstantiated module as an independent root, so two
/// independent top modules both simulate — the old single-pick dropped all but
/// the last-declared. A duplicate module name yields at most one root, resolved
/// to its canonical (first-declared) decl via `map` so a root never diverges from
/// how the same name is instantiated elsewhere. Degenerate (every module
/// instantiated — a cycle or a pure library, so the set is empty): fall back to
/// the last-declared single module so `run` still produces IR. Deterministic.
pub(crate) fn pick_roots<'a>(
    map: &ModuleMap<'a>,
    order: &[&'a ast::ModuleDecl],
    bind_checkers: &std::collections::BTreeSet<String>,
) -> Vec<&'a ast::ModuleDecl> {
    let instantiated = collect_instantiated(map, order);
    let canon = |name: &str, fallback: &'a ast::ModuleDecl| -> &'a ast::ModuleDecl {
        map.get(name).map(|(d, _)| *d).unwrap_or(fallback)
    };
    let mut roots: Vec<&ast::ModuleDecl> = Vec::new();
    let mut added: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for m in order {
        let name = m.name.name.as_str();
        // Round-9: a module attached ONLY via `bind` is not instantiated in any
        // body, so it would otherwise be picked as a spurious extra root (a
        // second, floating-port `$scope`). Exclude the bind-checker names.
        if instantiated.contains(name) || bind_checkers.contains(name) || !added.insert(name) {
            continue;
        }
        roots.push(canon(name, m));
    }
    if roots.is_empty() {
        if let Some(m) = order.last() {
            roots.push(canon(m.name.name.as_str(), m));
        }
    }
    roots
}

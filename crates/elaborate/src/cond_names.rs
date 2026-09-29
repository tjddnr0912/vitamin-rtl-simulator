//! §3 ⑤ⓙ: which names a generate condition holding a STRING LITERAL may read.
//!
//! `const_truth_in_scope` reads such a condition in the wide bit domain, and a name
//! there resolves through `wide_name_bits` — the maps as elaborate has filled them at
//! that moment. They are not a scope model: a generate block binds its declarations
//! by POSITION, once per generate phase, and some declarations bind in no map at all.
//! Measured, each a silent wrong answer where PRE was loud (E3010):
//!
//! * a block's `localparam X = "b";` is a string, which no integral map holds, so an
//!   OUTER `localparam int X` answered `if (X < "ab")` (`6e6f`, else) where all three
//!   oracles read the inner `62` (then);
//! * the same declaration written AFTER the condition is unbound in the Nets phase and
//!   bound in the later ones, so the Nets phase built the outer answer's branch and
//!   the later phases declined without a report: the branch's nets existed with no
//!   driver (`N1 zz`) and its processes were gone;
//! * a generate `typedef enum` whose labels do not bind (`gen_enum.rs`, rules 1–3)
//!   left an outer `localparam int LBL = 98` answering for its label `LBL = 97`
//!   (verilator and sv2v → iverilog `97`, then).
//!
//! Round 2 of the same review found the rest of that class: the maps also see through an
//! instance-array element into its parent (`t.u[0]` reads `t`'s `X`), take a procedural
//! block-local into `symbols` at its position, and ignore a generate block's `import`
//! and `let`. Every one of those reaches a name only from INSIDE a generate block, or a
//! name the module does not declare. So the condition step runs only where none of them
//! can: a condition written at the module's top level (outside every generate block,
//! branch and loop body), whose every name the module declares exactly once, at its top
//! level, as a parameter with a written integral type — plus, in a generate-for's own
//! condition, that loop's variable, which the loop binds before every test. The census
//! below answers the declaration half from the SOURCE, once per definition, over every
//! declaration at every depth, so it cannot depend on item order or on the phase.
//! Everything else declines and stays loud (E3010), as before; the literal-free spelling
//! of the same condition is untouched (docs/PROBE_CATALOG.md records where that spelling
//! reads the wrong object).

use super::*;
use crate::decl_collide::{collect_item, UnitKind};

/// One declaration of a name, as far as the condition's reading cares.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Site {
    /// A parameter written at the unit's top level with an integral type on the
    /// declaration (`int`, `integer`, a packed range, `byte`, `shortint`, …).
    TopTyped,
    /// Anything else — a nested or untyped parameter, a label, a net, a routine, …
    Other,
}

/// What a definition's source says about the names a top-level condition holding a
/// string literal may read.
#[derive(Default)]
pub(crate) struct CondCensus {
    /// The names it declares exactly once, anywhere, and that once as a top-level
    /// parameter with a written integral type.
    pub(crate) safe: BTreeSet<String>,
    /// The stems of its `parameter type` carriers (`T$w`, `T$s`, …), which the parser
    /// writes for a type parameter — for the refusal's wording only.
    pub(crate) carrier_stems: BTreeSet<String>,
}

impl CondCensus {
    /// Is `name` a carrier the parser minted for a type parameter?
    pub(crate) fn is_carrier(&self, name: &str) -> bool {
        name.split_once('$')
            .is_some_and(|(stem, _)| self.carrier_stems.contains(stem))
    }
}

/// The census of `unit` (see [`CondCensus`]).
pub(crate) fn cond_census(unit: &ast::ModuleDecl, kind: UnitKind) -> CondCensus {
    let carrier_stems =
        crate::decl_collide::carrier_stems(unit.params.iter().map(|p| p.name.name.as_str()).chain(
            unit.body.iter().filter_map(|it| match it {
                ast::ModuleItem::Param(p) => Some(p.name.name.as_str()),
                _ => None,
            }),
        ))
        .into_iter()
        .map(str::to_string)
        .collect();
    CondCensus {
        safe: cond_safe_names(unit, kind),
        carrier_stems,
    }
}

/// The names of `unit` a condition holding a string literal may read: those it declares
/// exactly once, anywhere, and that once as a top-level parameter with a written
/// integral type.
fn cond_safe_names(unit: &ast::ModuleDecl, kind: UnitKind) -> BTreeSet<String> {
    let mut sites: Sites<'_> = BTreeMap::new();
    for p in &unit.params {
        put(&mut sites, &p.name.name, param_site(p));
    }
    if let ast::PortList::Ansi(ports) = &unit.ports {
        for p in ports {
            put(&mut sites, &p.name.name, Site::Other);
        }
    }
    for it in &unit.body {
        match it {
            ast::ModuleItem::Param(p) => put(&mut sites, &p.name.name, param_site(p)),
            ast::ModuleItem::Generate(g) => gen_names(&g.items, kind, &mut sites),
            mi => item_names(mi, kind, &mut sites),
        }
    }
    sites
        .into_iter()
        .filter(|(_, s)| s == &[Site::TopTyped])
        .map(|(n, _)| n.to_string())
        .collect()
}

type Sites<'a> = BTreeMap<&'a str, Vec<Site>>;

fn put<'a>(sites: &mut Sites<'a>, name: &'a str, site: Site) {
    sites.entry(name).or_default().push(site);
}

/// A top-level parameter counts only with its type written on it: an untyped or
/// `string` one (the parser keeps no `string` keyword) takes its type from its value.
fn param_site(p: &ast::ParamDecl) -> Site {
    let typed = matches!(p.ty, ast::ParamType::Integer)
        || (matches!(p.ty, ast::ParamType::Implicit) && p.range.is_some());
    if typed {
        Site::TopTyped
    } else {
        Site::Other
    }
}

/// The names a plain module item declares (`decl_collide`'s collector: parameters,
/// nets and variables, routines, typedefs and enum labels, ports, genvars, instances,
/// block labels), each as [`Site::Other`].
fn item_names<'a>(mi: &'a ast::ModuleItem, kind: UnitKind, sites: &mut Sites<'a>) {
    let mut found = Vec::new();
    collect_item(mi, kind, false, &mut found);
    for s in found {
        put(sites, s.name, Site::Other);
    }
}

/// Every name a generate construct declares, at every depth: its blocks' labels, a
/// loop's variable (a genvar), and the items of every branch, arm, body and region.
fn gen_names<'a>(items: &'a [ast::GenItem], kind: UnitKind, sites: &mut Sites<'a>) {
    for it in items {
        let label = match it {
            ast::GenItem::For {
                init, label, body, ..
            } => {
                put(sites, &init.lvalue.name, Site::Other);
                gen_names(body, kind, sites);
                label
            }
            ast::GenItem::If {
                then_b,
                else_b,
                label,
                ..
            } => {
                gen_names(then_b, kind, sites);
                gen_names(else_b, kind, sites);
                label
            }
            ast::GenItem::Case { items, .. } => {
                for ci in items {
                    let (ast::GenCaseItem::Match { body, .. }
                    | ast::GenCaseItem::Default { body, .. }) = ci;
                    gen_names(body, kind, sites);
                }
                &None
            }
            ast::GenItem::Block { label, items, .. } => {
                gen_names(items, kind, sites);
                label
            }
            ast::GenItem::Item(b) => {
                match b.as_ref() {
                    ast::ModuleItem::Generate(g) => gen_names(&g.items, kind, sites),
                    mi => item_names(mi, kind, sites),
                }
                &None
            }
        };
        if let Some(l) = label {
            put(sites, &l.name, Site::Other);
        }
    }
}

impl Elaborator<'_> {
    /// May a top-level generate-if condition holding a string literal read every
    /// single-segment name in `e`? See the module doc; `false` when the current unit has
    /// no census.
    pub(crate) fn cond_names_ok(&self, e: &ast::Expr) -> bool {
        let Some(census) = self.cond_census.get(&self.cur_module) else {
            return false;
        };
        !crate::param_query::ast_any(e, &|x| match &x.kind {
            ast::ExprKind::Ident(p) => match p.segments.as_slice() {
                [seg] => !census.safe.contains(&seg.name),
                _ => false,
            },
            _ => false,
        })
    }
}

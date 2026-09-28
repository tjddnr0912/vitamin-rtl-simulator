//! §3 ⑤ⓗ: a `typedef enum` declared inside a generate block.
//!
//! IEEE 1800-2017 §6.19 declares an enum's labels in the scope that holds the
//! typedef, and §27.3 makes each generate block such a scope. Both oracles resolve a
//! name by SCOPE; elaborate binds a block's declarations by POSITION, once per
//! generate phase (`lower_gen_module_item`), and the parser folds some reads
//! (`$bits(<type>)`, typedef widths) positionally before elaborate sees them. The two
//! agree for a typedef whose labels depend on nothing that position can change, so a
//! typedef's labels bind — at its position, in every phase — only when it is CARRIED:
//!
//! 1. it is written directly in its block, not inside a bare `begin … end` or a
//!    region nested there (vita flattens those into the block; both oracles give a
//!    bare `begin … end` a scope of its own);
//! 2. its base bounds and label values are literals and operators, casts and system
//!    functions over them, or a keyword base (`int`, `byte`, a bare `logic`). A
//!    literal the parser synthesized from a fold of a NAME that position can change
//!    (`$bits(t)` becomes a decimal whose span is the call's) does not count. The
//!    parser's other synthesized spellings that pass — a typedef-name base's range,
//!    a package constant respelled to its value, an enum method's `first` / `last` /
//!    `num` — are facts position cannot change: a type is declared before its use and
//!    its table is scoped, and a package constant is fixed (§6.20.1);
//! 3. nothing above it in its block reads one of its labels. The walk that checks is
//!    conservative: an item it does not model (a process, an instance, a nested
//!    generate construct, a routine) counts as a read;
//! 4. each of its label names is declared once in its block: no other item, nested
//!    generate block label (a `then`, `else`, `else if`, `case`-arm or loop block),
//!    the block's loop variable (§27.4), an explicit import, or a declaration inside
//!    a bare `begin … end` or region there takes it. Such a design is illegal (both
//!    oracles refuse the pair) or scoped apart by the oracles where vita flattens;
//!    either way a binding here would answer where both oracles do not.
//!
//! Any other generate typedef keeps the path it had before this slice: its labels
//! are not bound, so a read is E3010 (or finds an outer object, as it always did).
//! A typedef in a module-level `generate … endgenerate` region is the module's
//! (§27.2), not a block's; it stays on that path too.

use super::*;
use crate::decl_collide::{collect_item, UnitKind};

/// A generate body that is a declarative region of its own (§27.3), with the loop
/// variable of a `for` body: §27.4 declares it in each iteration's block as an
/// implicit localparam.
struct GenScope<'a> {
    items: &'a [ast::GenItem],
    genvar: Option<&'a ast::Ident>,
}

/// The generate scopes directly under `items` — a `for` body, each `if` branch and
/// `case` arm, a labelled block — not the ones nested inside them. An unlabelled
/// `begin … end` and a region are searched through: elaborate gives them no scope.
fn child_scopes<'a>(items: &'a [ast::GenItem], out: &mut Vec<GenScope<'a>>) {
    for it in items {
        match it {
            ast::GenItem::For { init, body, .. } => out.push(GenScope {
                items: body,
                genvar: Some(&init.lvalue),
            }),
            ast::GenItem::If { then_b, else_b, .. } => {
                for b in [then_b, else_b] {
                    out.push(GenScope {
                        items: b,
                        genvar: None,
                    });
                }
            }
            ast::GenItem::Case { items, .. } => {
                for ci in items {
                    let (ast::GenCaseItem::Match { body, .. }
                    | ast::GenCaseItem::Default { body, .. }) = ci;
                    out.push(GenScope {
                        items: body,
                        genvar: None,
                    });
                }
            }
            ast::GenItem::Block {
                label: Some(_),
                items,
                ..
            } => out.push(GenScope {
                items,
                genvar: None,
            }),
            ast::GenItem::Block {
                label: None, items, ..
            } => child_scopes(items, out),
            ast::GenItem::Item(b) => {
                if let ast::ModuleItem::Generate(g) = b.as_ref() {
                    child_scopes(&g.items, out);
                }
            }
        }
    }
}

/// Every generate scope a definition holds, nested ones included.
fn unit_scopes(unit: &ast::ModuleDecl) -> Vec<GenScope<'_>> {
    let mut all = Vec::new();
    for it in &unit.body {
        if let ast::ModuleItem::Generate(g) = it {
            child_scopes(&g.items, &mut all);
        }
    }
    let mut i = 0;
    while i < all.len() {
        let mut kids = Vec::new();
        child_scopes(all[i].items, &mut kids);
        all.extend(kids);
        i += 1;
    }
    all
}

/// The items of one scope in source order, a bare `begin … end` or region spliced in
/// where it stands — vita's reading of the block.
fn level_items<'a>(items: &'a [ast::GenItem], out: &mut Vec<&'a ast::GenItem>) {
    for it in items {
        match it {
            ast::GenItem::Block {
                label: None, items, ..
            } => level_items(items, out),
            ast::GenItem::Item(b) => match b.as_ref() {
                ast::ModuleItem::Generate(g) => level_items(&g.items, out),
                _ => out.push(it),
            },
            _ => out.push(it),
        }
    }
}

/// Every name one generate scope declares, with how many times: its direct items
/// (an explicit import included), the labels of the generate blocks nested in it —
/// a loop, a `then` block, an `else` block and every block of an `else if` chain,
/// a `case` arm's block, a free-standing labelled block — its loop variable, and
/// whatever a bare `begin … end` or region inside it declares.
fn scope_names<'a>(s: &GenScope<'a>, kind: UnitKind) -> BTreeMap<&'a str, usize> {
    let mut names: Vec<&'a str> = Vec::new();
    if let Some(g) = s.genvar {
        names.push(&g.name);
    }
    level_names(s.items, kind, &mut names);
    let mut out = BTreeMap::new();
    for n in names {
        *out.entry(n).or_insert(0) += 1;
    }
    out
}

fn level_names<'a>(items: &'a [ast::GenItem], kind: UnitKind, out: &mut Vec<&'a str>) {
    for it in items {
        match it {
            ast::GenItem::Item(b) => match b.as_ref() {
                ast::ModuleItem::Generate(g) => level_names(&g.items, kind, out),
                ast::ModuleItem::Import(imp) => {
                    out.extend(imp.item.iter().map(|n| n.name.as_str()))
                }
                mi => {
                    let mut sites = Vec::new();
                    collect_item(mi, kind, false, &mut sites);
                    out.extend(sites.iter().map(|s| s.name));
                }
            },
            ast::GenItem::Block {
                label: None, items, ..
            } => level_names(items, kind, out),
            ast::GenItem::Block { label: Some(l), .. }
            | ast::GenItem::For { label: Some(l), .. } => out.push(&l.name),
            ast::GenItem::For { label: None, .. } => {}
            ast::GenItem::If { .. } => if_chain_labels(it, out),
            ast::GenItem::Case { items, .. } => {
                for ci in items {
                    let (ast::GenCaseItem::Match { body, .. }
                    | ast::GenCaseItem::Default { body, .. }) = ci;
                    if let [ast::GenItem::Block { label: Some(l), .. }] = body.as_slice() {
                        out.push(&l.name);
                    }
                }
            }
        }
    }
}

/// The block labels of an `if` construct: the `then` block's (hoisted onto the node),
/// the kept `else` block's, and those of an `else if` chain.
fn if_chain_labels<'a>(it: &'a ast::GenItem, out: &mut Vec<&'a str>) {
    let ast::GenItem::If { label, else_b, .. } = it else {
        return;
    };
    if let Some(l) = label {
        out.push(&l.name);
    }
    match else_b.as_slice() {
        [ast::GenItem::Block { label: Some(l), .. }] => out.push(&l.name),
        [next @ ast::GenItem::If { .. }] => if_chain_labels(next, out),
        _ => {}
    }
}

/// The spans of the generate typedefs of `unit` whose labels bind (see the module
/// doc). Per definition: the question is about the source, not an instance.
pub(crate) fn gen_carried_typedefs(unit: &ast::ModuleDecl, kind: UnitKind) -> BTreeSet<(u32, u32)> {
    let mut out = BTreeSet::new();
    for s in unit_scopes(unit) {
        let mut flat = Vec::new();
        level_items(s.items, &mut flat);
        let names = scope_names(&s, kind);
        for it in s.items {
            let ast::GenItem::Item(b) = it else { continue };
            let ast::ModuleItem::Typedef(td) = b.as_ref() else {
                continue;
            };
            let Some(at) = flat.iter().position(|f| std::ptr::eq(*f, it)) else {
                continue;
            };
            if carried(td, &flat[..at], &names) {
                out.insert((td.span.lo, td.span.hi));
            }
        }
    }
    out
}

fn carried(td: &ast::TypedefDecl, above: &[&ast::GenItem], names: &BTreeMap<&str, usize>) -> bool {
    let ast::TypedefKind::Enum { base, labels, .. } = &td.kind else {
        return false;
    };
    let is_label = |n: &str| labels.iter().any(|l| l.name.name == n);
    // A keyword base (`int`, `byte`, a bare `logic`) is synthesized by `dec_range`:
    // two decimals and every span empty, no name behind it.
    let keyword_base = |r: &ast::Range| {
        r.span.lo == r.span.hi
            && matches!(
                (&r.msb.kind, &r.lsb.kind),
                (ast::ExprKind::IntLit { .. }, ast::ExprKind::IntLit { .. })
            )
    };
    base.as_ref()
        .is_none_or(|r| keyword_base(r) || (literal_only(&r.msb) && literal_only(&r.lsb)))
        && labels
            .iter()
            .all(|l| l.value.as_ref().is_none_or(literal_only))
        && above.iter().all(|it| {
            let mut names = Vec::new();
            item_reads(it, &mut names) && !names.iter().any(|n| is_label(n))
        })
        && labels
            .iter()
            .all(|l| names.get(l.name.name.as_str()) == Some(&1))
}

/// `e` reads no name: literals as the source spells them, and operators, casts and
/// system functions over them.
fn literal_only(e: &ast::Expr) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        // A literal the parser synthesized from a fold carries the span of what it
        // replaced (`dec_lit` for `$bits(t)`), which is longer than its digits.
        K::IntLit { raw, .. } => raw.len() == (e.span.hi - e.span.lo) as usize,
        K::Paren { inner } => literal_only(inner),
        K::Unary { operand, .. } => literal_only(operand),
        K::Binary { lhs, rhs, .. } => literal_only(lhs) && literal_only(rhs),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => literal_only(cond) && literal_only(then_e) && literal_only(else_e),
        K::Concat { parts } => parts.iter().all(literal_only),
        K::Replicate { count, value } => literal_only(count) && value.iter().all(literal_only),
        K::SysCall { args, .. } => args.iter().all(literal_only),
        K::Cast { target, expr } => {
            (match target {
                ast::CastTarget::Prim(_) | ast::CastTarget::Signing { .. } => true,
                ast::CastTarget::Size(n) => literal_only(n),
                ast::CastTarget::Named(_) | ast::CastTarget::SigningParam { .. } => false,
            }) && literal_only(expr)
        }
        _ => false,
    }
}

/// The single-segment names `e` reads, pushed onto `out`; `false` when `e` holds a
/// shape this walk does not model (a hierarchical name, a method call, …), which a
/// caller must read as "may read anything".
fn expr_reads<'a>(e: &'a ast::Expr, out: &mut Vec<&'a str>) -> bool {
    use ast::ExprKind as K;
    match &e.kind {
        K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::Null | K::Dollar => true,
        // A package-qualified name is not a generate block's.
        K::PkgScoped { .. } => true,
        K::Ident(p) => match p.segments.as_slice() {
            [s] => {
                out.push(&s.name);
                true
            }
            _ => false,
        },
        K::TimeLit { num, .. } => expr_reads(num, out),
        K::Paren { inner } => expr_reads(inner, out),
        K::Unary { operand, .. } => expr_reads(operand, out),
        K::Binary { lhs, rhs, .. } => expr_reads(lhs, out) && expr_reads(rhs, out),
        K::Ternary {
            cond,
            then_e,
            else_e,
        } => expr_reads(cond, out) && expr_reads(then_e, out) && expr_reads(else_e, out),
        K::MinTypMax { min, typ, max } => {
            expr_reads(min, out) && expr_reads(typ, out) && expr_reads(max, out)
        }
        K::BitSelect { base, index } => expr_reads(base, out) && expr_reads(index, out),
        K::PartSelect { base, msb, lsb } => {
            expr_reads(base, out) && expr_reads(msb, out) && expr_reads(lsb, out)
        }
        K::IndexedPart {
            base,
            offset,
            width,
            ..
        } => expr_reads(base, out) && expr_reads(offset, out) && expr_reads(width, out),
        K::Concat { parts } | K::AssignPattern(parts) => parts.iter().all(|p| expr_reads(p, out)),
        K::AssignPatternKeyed(kv) => kv.iter().all(|(_, v)| expr_reads(v, out)),
        K::Replicate { count, value } => {
            expr_reads(count, out) && value.iter().all(|p| expr_reads(p, out))
        }
        K::Call { name, args } => match name.segments.as_slice() {
            [s] => {
                out.push(&s.name);
                args.iter().all(|a| expr_reads(a, out))
            }
            _ => false,
        },
        K::SysCall { args, .. } => args.iter().all(|a| expr_reads(a, out)),
        K::NamedArg { value, .. } => value.as_ref().is_none_or(|v| expr_reads(v, out)),
        K::Cast { target, expr } => {
            let t = match target {
                ast::CastTarget::Prim(_) | ast::CastTarget::Signing { .. } => true,
                ast::CastTarget::Size(n) => expr_reads(n, out),
                ast::CastTarget::Named(p) => match p.segments.as_slice() {
                    [s] => {
                        out.push(&s.name);
                        true
                    }
                    _ => false,
                },
                ast::CastTarget::SigningParam { shape_param } => {
                    out.push(&shape_param.name);
                    true
                }
            };
            t && expr_reads(expr, out)
        }
        K::MethodCall { .. }
        | K::RandomizeWith(_)
        | K::ArrayMethodWith(_)
        | K::New { .. }
        | K::ClassNew { .. }
        | K::Dist { .. }
        | K::Error => false,
    }
}

fn range_reads<'a>(r: &'a ast::Range, out: &mut Vec<&'a str>) -> bool {
    expr_reads(&r.msb, out) && expr_reads(&r.lsb, out)
}

/// The names one item of a scope reads, for the check that nothing above a typedef
/// reads its labels; `false` for an item this walk does not model.
fn item_reads<'a>(it: &'a ast::GenItem, out: &mut Vec<&'a str>) -> bool {
    let ast::GenItem::Item(b) = it else {
        return false;
    };
    match b.as_ref() {
        ast::ModuleItem::Param(p) => {
            p.range.as_ref().is_none_or(|r| range_reads(r, out)) && expr_reads(&p.value, out)
        }
        ast::ModuleItem::NetVar(d) => {
            d.delay.is_none()
                && d.class_type.is_none()
                && d.class_args.is_empty()
                && d.range.as_ref().is_none_or(|r| range_reads(r, out))
                && d.packed.iter().all(|r| range_reads(r, out))
                && d.shape_param.as_ref().is_none_or(|s| {
                    out.push(&s.name);
                    true
                })
                && d.names.iter().all(|n| {
                    n.unpacked.iter().all(|dim| match dim {
                        ast::Dim::Range(r) => range_reads(r, out),
                        ast::Dim::Size(e) => expr_reads(e, out),
                        ast::Dim::Dyn => true,
                        ast::Dim::Queue(q) => q.as_ref().is_none_or(|e| expr_reads(e, out)),
                        ast::Dim::Assoc(_) => false,
                    }) && n.init.as_ref().is_none_or(|e| expr_reads(e, out))
                })
        }
        ast::ModuleItem::Typedef(td) => match &td.kind {
            ast::TypedefKind::Enum { base, labels, .. } => {
                base.as_ref().is_none_or(|r| range_reads(r, out))
                    && labels
                        .iter()
                        .all(|l| l.value.as_ref().is_none_or(|v| expr_reads(v, out)))
            }
            ast::TypedefKind::Alias { range, packed, .. } => {
                range.as_ref().is_none_or(|r| range_reads(r, out))
                    && packed.iter().all(|r| range_reads(r, out))
            }
            ast::TypedefKind::Struct { members } => members.iter().all(|m| {
                m.range.as_ref().is_none_or(|r| range_reads(r, out))
                    && m.packed_dims.iter().all(|r| range_reads(r, out))
            }),
        },
        ast::ModuleItem::Genvar { .. } | ast::ModuleItem::Import(_) => true,
        _ => false,
    }
}

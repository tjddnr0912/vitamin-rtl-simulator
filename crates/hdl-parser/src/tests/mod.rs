use super::*;

mod expr_t;
mod inst_gen;
mod misc_forms;
mod stmt_s;
mod sva_ca;

fn p(src: &str) -> (Option<SourceUnit>, Vec<ParseError>) {
    let (toks, lex_errs) = hdl_lexer::lex(src);
    assert!(lex_errs.is_empty(), "lex errors: {lex_errs:?}");
    parse(&toks, src)
}
fn first_module(su: &SourceUnit) -> &ModuleDecl {
    match &su.items[0] {
        TopItem::Module(m) => m,
        _ => panic!("not a module"),
    }
}
/// Parse a bare expression via `assign x = <expr>;` and return the RHS.
fn expr_of(src: &str) -> Expr {
    let wrapped = format!("module m; assign x = {src};\nendmodule");
    let (su, errs) = p(&wrapped);
    assert!(errs.is_empty(), "parse errors for `{src}`: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    match &m.body[0] {
        ModuleItem::ContAssign(ca) => ca.assigns[0].1.clone(),
        _ => panic!(),
    }
}
fn bin(e: &Expr) -> (BinOp, &Expr, &Expr) {
    match &e.kind {
        ExprKind::Binary { op, lhs, rhs } => (*op, lhs, rhs),
        other => panic!("not binary: {other:?}"),
    }
}
/// Parse a module body; return the first ProceduralBlock.
fn proc_of(body: &str) -> ProceduralBlock {
    let src = format!("module m;\n{body}\nendmodule");
    let (su, errs) = p(&src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    match m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_))) {
        Some(ModuleItem::Proc(pb)) => pb.clone(),
        _ => panic!("no procedural block in body"),
    }
}
fn as_block(s: &Stmt) -> (&Option<Ident>, &Vec<NetVarDecl>, &Vec<Stmt>) {
    match s {
        Stmt::Block {
            label,
            decls,
            stmts,
            ..
        } => (label, decls, stmts),
        other => panic!("not a Block: {other:?}"),
    }
}

// ════════════════════ module instantiation (PR3) ════════════════════
/// Return the first ModuleInstance in a module body.
fn inst_of(body: &str) -> ModuleInstance {
    let src = format!("module m;\n{body}\nendmodule");
    let (su, errs) = p(&src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    match m.body.iter().find(|i| matches!(i, ModuleItem::Instance(_))) {
        Some(ModuleItem::Instance(mi)) => mi.clone(),
        _ => panic!("no module instance in body"),
    }
}
fn id_name(e: &Expr) -> &str {
    match &e.kind {
        ExprKind::Ident(p) => p.segments[0].name.as_str(),
        other => panic!("not a bare ident: {other:?}"),
    }
}

// ───────────────────────── PR3: generate / genvar ─────────────────────────

/// Parse a single generate construct wrapped in a module; return its items.
fn gen_of(body: &str) -> Vec<GenItem> {
    let src = format!("module m;\n{body}\nendmodule");
    let (su, errs) = p(&src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    match m.body.iter().find_map(|i| match i {
        ModuleItem::Generate(g) => Some(g),
        _ => None,
    }) {
        Some(g) => g.items.clone(),
        None => panic!("no generate construct in: {src}"),
    }
}

// ─────────────────────── function / task definitions ───────────────────────
fn item_of(body: &str) -> ModuleItem {
    let src = format!("module m;\n{body}\nendmodule");
    let (su, errs) = p(&src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    m.body[0].clone()
}

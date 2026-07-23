use super::*;

/// SV cast `casting_type'(expr)` (§6.24) parses to `ExprKind::Cast` with the
/// right `CastTarget`; malformed casts are loud parse errors (never silent).
#[test]
fn cast_parse_forms_and_malformed() {
    fn rhs_of(src_body: &str) -> Option<ExprKind> {
        let src = format!("module t; initial x = {src_body}; endmodule");
        let (toks, le) = hdl_lexer::lex(&src);
        assert!(le.is_empty(), "lex errors in {src_body:?}: {le:?}");
        let (unit, errs) = parse(&toks, &src);
        if !errs.is_empty() {
            return None; // signals a (loud) parse error to the caller
        }
        let TopItem::Module(ref m) = unit.as_ref()?.items[0] else {
            return None;
        };
        let ModuleItem::Proc(ref pb) = m.body[0] else {
            return None;
        };
        // initial <stmt>; — dig out the blocking-assign rhs.
        fn find_rhs(s: &Stmt) -> Option<ExprKind> {
            match s {
                Stmt::Blocking { rhs, .. } => Some(rhs.kind.clone()),
                Stmt::Block { stmts, .. } => stmts.iter().find_map(find_rhs),
                _ => None,
            }
        }
        find_rhs(&pb.body)
    }
    use CastPrim as P;
    // type cast → Prim
    assert!(matches!(
        rhs_of("int'(8'hFF)"),
        Some(ExprKind::Cast {
            target: CastTarget::Prim(P::Int),
            ..
        })
    ));
    assert!(matches!(
        rhs_of("byte'(a)"),
        Some(ExprKind::Cast {
            target: CastTarget::Prim(P::Byte),
            ..
        })
    ));
    assert!(matches!(
        rhs_of("logic'(a)"),
        Some(ExprKind::Cast {
            target: CastTarget::Prim(P::Logic),
            ..
        })
    ));
    // signing cast → Signing
    assert!(matches!(
        rhs_of("signed'(a)"),
        Some(ExprKind::Cast {
            target: CastTarget::Signing { signed: true },
            ..
        })
    ));
    assert!(matches!(
        rhs_of("unsigned'(a)"),
        Some(ExprKind::Cast {
            target: CastTarget::Signing { signed: false },
            ..
        })
    ));
    // size cast → Size; typedef/class name → Named
    assert!(matches!(
        rhs_of("8'(a)"),
        Some(ExprKind::Cast {
            target: CastTarget::Size(_),
            ..
        })
    ));
    assert!(matches!(
        rhs_of("(W+1)'(a)"),
        Some(ExprKind::Cast {
            target: CastTarget::Size(_),
            ..
        })
    ));
    assert!(matches!(
        rhs_of("my_t'(a)"),
        Some(ExprKind::Cast {
            target: CastTarget::Named(_),
            ..
        })
    ));
    // precedence: cast binds tighter than `+` → `(8'(a)) + b`
    assert!(matches!(rhs_of("8'(a) + b"), Some(ExprKind::Binary { .. })));
    // replication wrapping a cast still parses
    assert!(matches!(
        rhs_of("{2{8'(a)}}"),
        Some(ExprKind::Replicate { .. })
    ));
    // malformed casts → loud parse error (None)
    for bad in ["int'", "int'(", "8'(", "8'(a", "signed'5"] {
        assert!(
            rhs_of(bad).is_none(),
            "expected loud parse error for {bad:?}"
        );
    }
}

/// v7 AST flip: package/import/string/pkg:: parse to their dedicated
/// shapes (semantics land in the follow-on slices — parse-only here).
#[test]
fn v7_package_import_string_pkgscoped_parse() {
    let src = r#"
package p;
  parameter W = 8;
endpackage
import p::*;
module t;
  import p::W;
  string s;
  integer x;
  initial x = p::W;
endmodule
"#;
    let (toks, lex_errs) = hdl_lexer::lex(src);
    assert!(lex_errs.is_empty());
    let (unit, errs) = parse(&toks, src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let unit = unit.unwrap();
    assert!(matches!(unit.items[0], TopItem::Package(ref m) if m.name.name == "p"));
    assert!(
        matches!(unit.items[1], TopItem::Import(ref i) if i.pkg.name == "p" && i.item.is_none())
    );
    let TopItem::Module(ref m) = unit.items[2] else {
        panic!("expected module, got {:?}", unit.items[2]);
    };
    assert!(matches!(
        m.body[0],
        ModuleItem::Import(ref i) if i.pkg.name == "p"
            && i.item.as_ref().map(|x| x.name.as_str()) == Some("W")
    ));
    assert!(matches!(
        m.body[1],
        ModuleItem::NetVar(ref d) if matches!(d.kind, NetVarKind::String)
    ));
    // the initial body holds `x = p::W` — walk to the PkgScoped expr.
    let ModuleItem::Proc(ref pb) = m.body[3] else {
        panic!("expected proc, got {:?}", m.body[3]);
    };
    let mut found = false;
    fn walk(s: &Stmt, found: &mut bool) {
        if let Stmt::Blocking { rhs, .. } = s {
            if matches!(
                rhs.kind,
                ExprKind::PkgScoped { ref pkg, ref name }
                    if pkg.name == "p" && name.name == "W"
            ) {
                *found = true;
            }
        }
        if let Stmt::Block { stmts, .. } = s {
            for st in stmts {
                walk(st, found);
            }
        }
    }
    walk(&pb.body, &mut found);
    assert!(found, "p::W must parse as PkgScoped");
}

/// Review-finding regressions (2026-06-11): the foreach rename walker
/// must leave NO single-segment reference to the source-level index name
/// anywhere in the desugared tree — including block-local decl
/// initializers/dims and event-control sensitivity exprs (the two arms a
/// review caught as missed → silent outer-variable capture).
#[test]
fn foreach_rename_covers_decl_inits_and_event_ctrl() {
    let src = r#"
module t;
  integer q [$];
  integer r;
  initial begin
    foreach (q[i]) begin
      integer k = q[i];
      @(q[i]) r = q[i];
    end
  end
endmodule
"#;
    let (toks, lex_errs) = hdl_lexer::lex(src);
    assert!(lex_errs.is_empty());
    let (unit, errs) = parse(&toks, src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let unit = unit.unwrap();
    // walk the whole AST; collect every single-segment ident name.
    fn idents_in_expr(e: &Expr, out: &mut Vec<String>) {
        match &e.kind {
            ExprKind::Ident(p) => {
                if p.segments.len() == 1 {
                    out.push(p.segments[0].name.clone());
                }
            }
            ExprKind::Unary { operand, .. } => idents_in_expr(operand, out),
            ExprKind::Binary { lhs, rhs, .. } => {
                idents_in_expr(lhs, out);
                idents_in_expr(rhs, out);
            }
            ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                idents_in_expr(cond, out);
                idents_in_expr(then_e, out);
                idents_in_expr(else_e, out);
            }
            ExprKind::BitSelect { base, index } => {
                idents_in_expr(base, out);
                idents_in_expr(index, out);
            }
            ExprKind::PartSelect { base, msb, lsb } => {
                idents_in_expr(base, out);
                idents_in_expr(msb, out);
                idents_in_expr(lsb, out);
            }
            ExprKind::Call { args, .. } | ExprKind::SysCall { args, .. } => {
                for a in args {
                    idents_in_expr(a, out);
                }
            }
            ExprKind::Paren { inner } => idents_in_expr(inner, out),
            _ => {}
        }
    }
    fn idents_in_stmt(s: &Stmt, out: &mut Vec<String>) {
        match s {
            Stmt::Blocking { lhs, rhs, .. } | Stmt::NonBlocking { lhs, rhs, .. } => {
                if let Lvalue::Ident(p) = lhs {
                    if p.segments.len() == 1 {
                        out.push(p.segments[0].name.clone());
                    }
                }
                if let Lvalue::BitSelect { index, .. } = lhs {
                    idents_in_expr(index, out);
                }
                idents_in_expr(rhs, out);
            }
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                idents_in_stmt(init, out);
                idents_in_expr(cond, out);
                idents_in_stmt(step, out);
                idents_in_stmt(body, out);
            }
            Stmt::Block { decls, stmts, .. } => {
                for d in decls {
                    for n in &d.names {
                        if let Some(e) = &n.init {
                            idents_in_expr(e, out);
                        }
                    }
                }
                for st in stmts {
                    idents_in_stmt(st, out);
                }
            }
            Stmt::EventCtrl { ctrl, body, .. } => {
                if let Sensitivity::List(evs) = ctrl {
                    for ev in evs {
                        idents_in_expr(&ev.expr, out);
                    }
                }
                if let Some(b) = body {
                    idents_in_stmt(b, out);
                }
            }
            _ => {}
        }
    }
    let mut names = Vec::new();
    for it in &unit.items {
        if let TopItem::Module(m) = it {
            for item in &m.body {
                if let ModuleItem::Proc(pb) = item {
                    idents_in_stmt(&pb.body, &mut names);
                }
            }
        }
    }
    assert!(
        !names.iter().any(|n| n == "i"),
        "the source index name must be fully renamed; leftover refs: {names:?}"
    );
    assert!(
        names.iter().any(|n| n.starts_with("__foreach_i_")),
        "the synthetic index must appear: {names:?}"
    );
}

// ft1. ANSI combinational function: width range, one input formal, single
//      `f = <expr>` body reachable as a Block with one Blocking to the func name.
#[test]
fn ft1_parse_ansi_function_def() {
    let it = item_of("function [7:0] add1(input [7:0] x); add1 = x + 1; endfunction");
    let ModuleItem::Func(f) = it else {
        panic!("not a function: {it:?}");
    };
    assert_eq!(f.name.name, "add1");
    assert!(!f.automatic);
    assert!(f.range.is_some(), "expected [7:0] return range");
    assert_eq!(f.ret_type, ParamType::Implicit);
    assert_eq!(f.ports.len(), 1);
    assert_eq!(f.ports[0].dir, PortDir::Input);
    assert_eq!(f.ports[0].name.name, "x");
    assert!(f.ports[0].range.is_some());
    // body: a single blocking assign `add1 = x + 1`
    let Stmt::Blocking { lhs, rhs, .. } = &*f.body else {
        panic!("expected single Blocking body, got {:?}", f.body);
    };
    assert!(matches!(lhs, Lvalue::Ident(p) if p.segments[0].name == "add1"));
    assert!(matches!(&rhs.kind, ExprKind::Binary { op: BinOp::Add, .. }));
}

// ft2. Non-ANSI function: formal declared in the body prefix, hoisted into ports.
#[test]
fn ft2_parse_non_ansi_function_def() {
    let it = item_of(
        "function [3:0] f; input [3:0] a; reg [3:0] t; begin t = a; f = t; end endfunction",
    );
    let ModuleItem::Func(f) = it else {
        panic!("not a function: {it:?}");
    };
    assert_eq!(f.name.name, "f");
    // non-ANSI input `a` hoisted into ports
    assert_eq!(f.ports.len(), 1);
    assert_eq!(f.ports[0].dir, PortDir::Input);
    assert_eq!(f.ports[0].name.name, "a");
    // local `reg t` lands in body_decls
    assert_eq!(f.body_decls.len(), 1);
    assert_eq!(f.body_decls[0].names[0].name.name, "t");
    // body is a begin..end with two blocking assigns
    let Stmt::Block { stmts, .. } = &*f.body else {
        panic!("expected begin-end body, got {:?}", f.body);
    };
    assert_eq!(stmts.len(), 2);
}

// ft3. Task with input + output formals (ANSI), begin-end body.
#[test]
fn ft3_parse_task_def() {
    let it = item_of("task drive(input [7:0] d, output [7:0] q); begin q = d; end endtask");
    let ModuleItem::Task(t) = it else {
        panic!("not a task: {it:?}");
    };
    assert_eq!(t.name.name, "drive");
    assert!(!t.automatic);
    assert_eq!(t.ports.len(), 2);
    assert_eq!(t.ports[0].dir, PortDir::Input);
    assert_eq!(t.ports[0].name.name, "d");
    assert_eq!(t.ports[1].dir, PortDir::Output);
    assert_eq!(t.ports[1].name.name, "q");
    let Stmt::Block { stmts, .. } = &*t.body else {
        panic!("expected begin-end body, got {:?}", t.body);
    };
    assert_eq!(stmts.len(), 1);
}

// ft4. Sticky direction across comma-grouped formals: `input a, b` → both Input.
#[test]
fn ft4_sticky_direction_and_empty_task() {
    let it = item_of("function f(input a, b); f = a & b; endfunction");
    let ModuleItem::Func(f) = it else {
        panic!("not a function");
    };
    assert_eq!(f.ports.len(), 2);
    assert_eq!(f.ports[0].dir, PortDir::Input);
    assert_eq!(f.ports[1].dir, PortDir::Input);
    assert_eq!(f.ports[1].name.name, "b");

    // empty-bodied task with no port list: `task t; endtask`
    let it2 = item_of("task t; endtask");
    let ModuleItem::Task(t) = it2 else {
        panic!("not a task");
    };
    assert!(t.ports.is_empty());
    assert!(matches!(&*t.body, Stmt::Null(_)));
}

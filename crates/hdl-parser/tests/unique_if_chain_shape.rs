//! `unique if … else if …` (IEEE 1800-2017 §12.4.2) — the parse shape of the
//! violation arm (§4.5.585). The qualifier covers the whole `else if` series, so the
//! synthesized `UNIQUE_VIOLATION_TASK` call goes on the LAST `if` of the series and
//! carries the FIRST `if`'s span. The series follows `else` only into a bare `if`
//! written right after it (an attribute may sit between; the lexer drops it). An
//! immediate `assert` after `else` is a `Stmt::If` in the tree and ends the series, and
//! so does a qualified `unique` / `priority` / `unique0` / `priority0 if`, the series'
//! final `else` statement under IEEE 1800-2017 Syntax 12-2. Chains are armed only in
//! procedural code outside a subroutine: every function and task body keeps the
//! lone-`if` rule (`first_if_arm_only` in `lib.rs`, set by `tf_body`, names the
//! reasons). The flag is scoped to the body: a procedural block written after a
//! function, a task or a class is armed as usual. Runtime behaviour is pinned in
//! `crates/cli/tests/unique_if_chain.rs`.
use hdl_ast::{
    ClassDecl, ClassItem, ModuleItem, SourceUnit, Span, Stmt, TopItem, UNIQUE_VIOLATION_TASK,
};

fn parse(src: &str) -> SourceUnit {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, errs) = hdl_parser::parse(&toks, src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    su.expect("a source unit")
}

fn items(su: &SourceUnit) -> &[ModuleItem] {
    su.items
        .iter()
        .find_map(|i| match i {
            TopItem::Module(m) => Some(&m.body[..]),
            _ => None,
        })
        .expect("a module")
}

fn class(su: &SourceUnit) -> &ClassDecl {
    su.items
        .iter()
        .find_map(|i| match i {
            TopItem::Class(c) => Some(c),
            _ => None,
        })
        .expect("a class")
}

/// The body of the class method named `name`.
fn method<'a>(c: &'a ClassDecl, name: &str) -> &'a Stmt {
    c.items
        .iter()
        .find_map(|i| match i {
            ClassItem::Func { def, .. } if def.name.name == name => Some(&*def.body),
            ClassItem::Task { def, .. } if def.name.name == name => Some(&*def.body),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no method {name}"))
}

/// The body of the module task named `name` (an item `function void` is one).
fn task<'a>(su: &'a SourceUnit, name: &str) -> &'a Stmt {
    items(su)
        .iter()
        .find_map(|i| match i {
            ModuleItem::Task(t) if t.name.name == name => Some(&*t.body),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no task {name}"))
}

fn func<'a>(su: &'a SourceUnit, name: &str) -> &'a Stmt {
    items(su)
        .iter()
        .find_map(|i| match i {
            ModuleItem::Func(f) if f.name.name == name => Some(&*f.body),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no function {name}"))
}

/// Every procedural block's body, in source order.
fn proc_bodies(su: &SourceUnit) -> Vec<&Stmt> {
    items(su)
        .iter()
        .filter_map(|i| match i {
            ModuleItem::Proc(pb) => Some(&*pb.body),
            _ => None,
        })
        .collect()
}

fn proc_body(su: &SourceUnit) -> &Stmt {
    items(su)
        .iter()
        .find_map(|i| match i {
            ModuleItem::Proc(pb) => Some(&*pb.body),
            _ => None,
        })
        .expect("a procedural block")
}

/// The first `if` reachable through blocks.
fn first_if(s: &Stmt) -> &Stmt {
    fn go(s: &Stmt) -> Option<&Stmt> {
        match s {
            Stmt::If { .. } => Some(s),
            Stmt::Block { stmts, .. } | Stmt::Fork { stmts, .. } => stmts.iter().find_map(go),
            _ => None,
        }
    }
    go(s).unwrap_or_else(|| panic!("no if in {s:?}"))
}

fn is_arm(s: &Stmt) -> bool {
    matches!(s, Stmt::SysTaskCall { name, .. } if name.name == UNIQUE_VIOLATION_TASK)
}

/// Every violation arm under `s`, by its span.
fn arms(s: &Stmt) -> Vec<Span> {
    let mut v = Vec::new();
    fn go(s: &Stmt, v: &mut Vec<Span>) {
        match s {
            Stmt::SysTaskCall { span, .. } if is_arm(s) => v.push(*span),
            Stmt::If { then_s, else_s, .. } => {
                go(then_s, v);
                if let Some(e) = else_s {
                    go(e, v);
                }
            }
            Stmt::Block { stmts, .. } | Stmt::Fork { stmts, .. } => {
                stmts.iter().for_each(|s| go(s, v))
            }
            _ => {}
        }
    }
    go(s, &mut v);
    v
}

/// Follows `else` while it is directly an `if`: (ifs in the series, the tail's
/// `else`, the first `if`'s span).
fn series(s: &Stmt) -> (usize, Option<&Stmt>, Span) {
    let Stmt::If { span, .. } = s else {
        panic!("not an if: {s:?}")
    };
    let mut n = 1;
    let mut cur = s;
    loop {
        let Stmt::If { else_s, .. } = cur else {
            unreachable!()
        };
        match else_s.as_deref() {
            Some(e @ Stmt::If { .. }) => {
                n += 1;
                cur = e;
            }
            other => return (n, other, *span),
        }
    }
}

/// A three-`if` series armed at its tail with the first `if`'s span.
fn assert_armed_chain(body: &Stmt, src: &str, what: &str) {
    let (n, tail, first) = series(first_if(body));
    assert_eq!(n, 3, "{what}: three ifs in the series");
    let tail = tail.unwrap_or_else(|| panic!("{what}: the tail if has no else"));
    assert!(
        is_arm(tail),
        "{what}: tail else is the violation arm: {tail:?}"
    );
    let Stmt::SysTaskCall { span, .. } = tail else {
        unreachable!()
    };
    assert_eq!(*span, first, "{what}: the arm carries the first if's span");
    assert!(src[first.lo as usize..].starts_with("if (x)"), "{what}");
}

/// A three-`if` series with no arm anywhere (the first-`if` rule: its `else` is taken).
fn assert_unarmed_chain(body: &Stmt, what: &str) {
    let (n, tail, _) = series(first_if(body));
    assert_eq!(n, 3, "{what}: three ifs in the series");
    assert!(tail.is_none(), "{what}: no arm on the chain: {tail:?}");
}

/// A lone `unique if` armed with its own span.
fn assert_armed_lone(body: &Stmt, what: &str) {
    let (n, tail, first) = series(first_if(body));
    assert_eq!(n, 1, "{what}");
    assert!(tail.is_some_and(is_arm), "{what}: lone if armed: {tail:?}");
    assert_eq!(arms(body), [first], "{what}");
}

const CHAIN: &str = "unique if (x) v = 1; else if (z) v = 2; else if (w) v = 3;";
const LONE: &str = "unique if (x) v = 1;";

#[test]
fn a_chain_in_procedural_code_is_armed_at_its_tail() {
    let src = format!(
        "module m; logic x, z, w; logic [1:0] v;\n\
         initial begin {CHAIN} end\n\
         always @(x) begin {CHAIN} end\n\
         always_comb begin {CHAIN} end\n\
         final begin {CHAIN} end\n\
         initial fork begin {CHAIN} end join\n\
         endmodule"
    );
    let su = parse(&src);
    let bodies = proc_bodies(&su);
    assert_eq!(bodies.len(), 5);
    for (what, body) in ["initial", "always", "always_comb", "final", "fork branch"]
        .into_iter()
        .zip(bodies)
    {
        assert_armed_chain(body, &src, what);
    }
}

#[test]
fn chains_in_every_subroutine_body_keep_the_first_if_rule() {
    let src = format!(
        "class C; logic x, z, w; logic [1:0] v;\n\
         function int cf; {CHAIN} return 0; endfunction\n\
         function void cv; {CHAIN} endfunction\n\
         function new; {CHAIN} endfunction\n\
         task ct; {CHAIN} endtask\n\
         endclass\n\
         module m; logic x, z, w; logic [1:0] v;\n\
         function automatic logic [1:0] f; {CHAIN} return v; endfunction\n\
         function void fv; {CHAIN} endfunction\n\
         task t; begin {CHAIN} end endtask\n\
         task automatic ta; {CHAIN} endtask\n\
         endmodule"
    );
    let su = parse(&src);
    assert_unarmed_chain(func(&su, "f"), "item non-void function");
    // `parse_function_item` turns an item `function void` into a `ModuleItem::Task`.
    assert_unarmed_chain(task(&su, "fv"), "item function void");
    assert_unarmed_chain(task(&su, "t"), "task");
    assert_unarmed_chain(task(&su, "ta"), "automatic task");
    let c = class(&su);
    assert_unarmed_chain(method(c, "cf"), "class non-void function");
    assert_unarmed_chain(method(c, "cv"), "class void function");
    assert_unarmed_chain(method(c, "new"), "constructor");
    assert_unarmed_chain(method(c, "ct"), "class task");
}

#[test]
fn a_lone_unique_if_is_armed_in_every_body() {
    let src = format!(
        "class C; logic x; logic [1:0] v;\n\
         function int cf; {LONE} return 0; endfunction\n\
         function void cv; {LONE} endfunction\n\
         function new; {LONE} endfunction\n\
         endclass\n\
         module m; logic x; logic [1:0] v;\n\
         function automatic logic [1:0] f; {LONE} return v; endfunction\n\
         function void fv; {LONE} endfunction\n\
         task t; {LONE} endtask\n\
         initial begin {LONE} end\n\
         endmodule"
    );
    let su = parse(&src);
    let c = class(&su);
    assert_armed_lone(method(c, "cf"), "class non-void function");
    assert_armed_lone(method(c, "cv"), "class void function");
    assert_armed_lone(method(c, "new"), "constructor");
    assert_armed_lone(func(&su, "f"), "item non-void function");
    assert_armed_lone(task(&su, "fv"), "item function void");
    assert_armed_lone(task(&su, "t"), "task");
    assert_armed_lone(proc_body(&su), "initial");
}

#[test]
fn a_procedural_block_after_a_subroutine_or_a_class_is_armed() {
    let r_chain = CHAIN.replace('v', "r");
    let src = format!(
        "module m; logic x, z, w; logic [1:0] v;\n\
         function automatic logic [1:0] f; logic [1:0] r; r = 0; {r_chain} return r; endfunction\n\
         initial begin {CHAIN} end\n\
         endmodule"
    );
    let su = parse(&src);
    assert_armed_chain(proc_body(&su), &src, "initial after a function");
    let src = format!(
        "module m; logic x, z, w; logic [1:0] v;\n\
         task t; {CHAIN} endtask\n\
         initial begin {CHAIN} end\n\
         endmodule"
    );
    let su = parse(&src);
    assert_armed_chain(proc_body(&su), &src, "initial after a task");
    let src = format!(
        "class C; logic x, z, w; logic [1:0] v;\n\
         function void cv; {CHAIN} endfunction\n\
         endclass\n\
         module m; logic x, z, w; logic [1:0] v;\n\
         initial begin {CHAIN} end\n\
         endmodule"
    );
    let su = parse(&src);
    assert_armed_chain(proc_body(&su), &src, "initial after a class");
}

#[test]
fn an_assert_after_else_ends_the_series() {
    let src = "module m; logic a, b, c; logic [1:0] v;\n\
               initial begin unique if (a) v = 1; else assert (b) else if (c) v = 2; end\n\
               endmodule";
    let su = parse(src);
    let body = proc_body(&su);
    let Stmt::If { else_s, .. } = first_if(body) else {
        unreachable!()
    };
    // the assertion is an `if` in the tree, but it was not written as `else if`
    assert!(
        matches!(else_s.as_deref(), Some(Stmt::If { .. })),
        "{else_s:?}"
    );
    assert!(arms(body).is_empty(), "no arm: {body:?}");
}

#[test]
fn an_attribute_after_else_is_followed() {
    let src = "module m; logic x, z, w; logic [1:0] v;\n\
               initial begin unique if (x) v = 1; else (* a *) if (z) v = 2; else if (w) v = 3; end\n\
               endmodule";
    let su = parse(src);
    assert_armed_chain(proc_body(&su), src, "attribute");
}

/// IEEE 1800-2017 Syntax 12-2: `{ else if ( … ) … }` takes no qualifier, so a qualified
/// `if` after `else` is the series' final `else` statement. The outer series gets no
/// arm; a `unique0` / `priority0` series suppresses its own.
#[test]
fn a_zero_qualified_if_after_else_ends_the_series() {
    for (what, stmt) in [
        (
            "unique, else unique0",
            "unique if (x) v = 1; else unique0 if (z) v = 2;",
        ),
        (
            "unique, else unique0 chain",
            "unique if (x) v = 1; else unique0 if (z) v = 2; else if (w) v = 3;",
        ),
        (
            "priority, else unique0",
            "priority if (x) v = 1; else unique0 if (z) v = 2;",
        ),
        (
            "unique chain, else priority0",
            "unique if (x) v = 1; else if (z) v = 2; else priority0 if (w) v = 3;",
        ),
    ] {
        let src = format!(
            "module m; logic x, z, w; logic [1:0] v;\n\
             initial begin {stmt} end\n\
             endmodule"
        );
        let su = parse(&src);
        let body = proc_body(&su);
        assert!(arms(body).is_empty(), "{what}: no arm: {body:?}");
    }
}

#[test]
fn an_inner_unique_if_after_else_stops_the_walk() {
    let src = "module m; logic x, z, w; logic [1:0] v;\n\
               initial begin unique if (x) v = 1; else unique if (z) v = 2; end\n\
               endmodule";
    let su = parse(src);
    let body = proc_body(&su);
    let (n, tail, _) = series(first_if(body));
    assert_eq!(n, 2);
    assert!(tail.is_some_and(is_arm), "{tail:?}");
    // one arm: the inner `unique if`'s own, at its span; the outer adds none
    let inner = src.find("if (z)").unwrap() as u32;
    let a = arms(body);
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].lo, inner);
}

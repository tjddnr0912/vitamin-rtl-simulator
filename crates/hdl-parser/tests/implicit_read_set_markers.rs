//! The parse-tree marks elaborate reads for the implicit read set of `always_comb` /
//! `always @*` (IEEE 1800-2017 §9.2.2.2.1, §9.4.2.2; `elaborate/src/read_set.rs`):
//! - `TaskDef.is_void_fn` — a `function void` lowers through the task machinery, but
//!   its body adds its reads to an `always_comb` and a task's does not (:9056-9058);
//! - `Stmt::If.assertion` / `Stmt::DeferredAssert.directive` — an immediate or deferred
//!   assertion's action blocks add nothing to an `always_comb` (:9061-9068);
//! - `Stmt::is_compound_assign_desugar` — the lvalue copy `lv op= e` / `lv++` reads is
//!   not an identifier of the statement for an `@*` (:9594-9595). It is recognized by
//!   span identity, so a written `lv = lv op e` must not match.
//!
//! Runtime behaviour is pinned in `crates/cli/tests/implicit_read_set.rs`.
use hdl_ast::{AssertDefer, AssertDirective, ModuleDecl, ModuleItem, SourceUnit, Stmt, TopItem};

fn parse(src: &str) -> SourceUnit {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, errs) = hdl_parser::parse(&toks, src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    su.expect("a source unit")
}

fn module<'a>(su: &'a SourceUnit, name: &str) -> &'a ModuleDecl {
    su.items
        .iter()
        .find_map(|i| match i {
            TopItem::Module(m) | TopItem::Package(m) if m.name.name == name => Some(m),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no module/package {name}"))
}

/// `(name, is_void_fn)` of every task-machinery routine of `m`.
fn tasks(m: &ModuleDecl) -> Vec<(String, bool)> {
    m.body
        .iter()
        .filter_map(|i| match i {
            ModuleItem::Task(t) => Some((t.name.name.clone(), t.is_void_fn)),
            _ => None,
        })
        .collect()
}

/// Every statement of the first `initial` / `always*` body of `m`, depth-first.
fn stmts(m: &ModuleDecl) -> Vec<&Stmt> {
    fn walk<'a>(s: &'a Stmt, out: &mut Vec<&'a Stmt>) {
        out.push(s);
        match s {
            Stmt::Block { stmts, .. } => stmts.iter().for_each(|c| walk(c, out)),
            Stmt::For { body, .. } => walk(body, out),
            _ => {}
        }
    }
    let body = m
        .body
        .iter()
        .find_map(|i| match i {
            ModuleItem::Proc(p) => Some(&*p.body),
            _ => None,
        })
        .expect("a procedural block");
    let mut out = Vec::new();
    walk(body, &mut out);
    out
}

#[test]
fn a_void_function_is_a_task_with_the_void_mark() {
    let su = parse(
        "package p; function void pv(input logic a); endfunction endpackage\n\
         function void cuv(input logic a); endfunction\n\
         module m;\n\
           function void v(input logic a); endfunction\n\
           function automatic void av(input logic a); endfunction\n\
           task t(input logic a); endtask\n\
           task automatic at(input logic a); endtask\n\
           function logic f(input logic a); return a; endfunction\n\
         endmodule\n",
    );
    let mut got = tasks(module(&su, "m"));
    got.sort();
    let want: Vec<(String, bool)> = [
        ("at", false),
        ("av", true),
        ("cuv", true),
        ("t", false),
        ("v", true),
    ]
    .iter()
    .map(|(n, b)| (n.to_string(), *b))
    .collect();
    assert_eq!(
        got, want,
        "module body (the CU-scope `cuv` is replicated into it)"
    );
    assert_eq!(
        tasks(module(&su, "p")),
        vec![("pv".to_string(), true)],
        "package"
    );
    assert!(
        module(&su, "m")
            .body
            .iter()
            .any(|i| matches!(i, ModuleItem::Func(f) if f.name.name == "f")),
        "a value function stays a FunctionDef"
    );
}

#[test]
fn an_assertion_desugar_carries_its_directive() {
    let su = parse(
        "module m; logic a, b;\n\
           initial begin\n\
             assert (a) b = 1; else b = 0;\n\
             assume (a);\n\
             if (a) b = 1; else b = 0;\n\
             assert #0 (a);\n\
             assume final (a) else b = 0;\n\
           end\n\
         endmodule\n",
    );
    let got: Vec<String> = stmts(module(&su, "m"))
        .into_iter()
        .filter_map(|s| match s {
            Stmt::If { assertion, .. } => Some(format!("if {assertion:?}")),
            Stmt::DeferredAssert {
                region, directive, ..
            } => Some(format!("deferred {region:?} {directive:?}")),
            _ => None,
        })
        .collect();
    assert_eq!(
        got,
        vec![
            format!("if {:?}", Some(AssertDirective::Assert)),
            format!("if {:?}", Some(AssertDirective::Assume)),
            "if None".to_string(),
            format!(
                "deferred {:?} {:?}",
                AssertDefer::Observed,
                AssertDirective::Assert
            ),
            format!(
                "deferred {:?} {:?}",
                AssertDefer::Reactive,
                AssertDirective::Assume
            ),
        ]
    );
}

#[test]
fn only_the_compound_desugar_is_marked_compound() {
    let su = parse(
        "module m; logic [3:0] v; logic [1:0] w [0:1]; int n; logic x, m1;\n\
           initial begin\n\
             v |= 4'h1;\n\
             v[1] |= m1;\n\
             w[1][0] ^= x;\n\
             {v[3], x} += 2'd1;\n\
             n++;\n\
             n--;\n\
             ++n;\n\
             --n;\n\
             n <<= 1;\n\
             v = v | 4'h1;\n\
             v[1] = v[1] | m1;\n\
             n = n + 1;\n\
             n = n;\n\
             v <= v | 4'h1;\n\
             for (int i = 0; i < 2; i++) x = w[i][0];\n\
           end\n\
         endmodule\n",
    );
    let got: Vec<bool> = stmts(module(&su, "m"))
        .into_iter()
        .filter(|s| matches!(s, Stmt::Blocking { .. } | Stmt::NonBlocking { .. }))
        .map(Stmt::is_compound_assign_desugar)
        .collect();
    assert_eq!(
        got,
        vec![
            true, true, true, true, true, true, true, true, true, // op= ++ -- (prefix too)
            false, false, false, false, false, // written forms, nonblocking
            false, // the `for` body; its init and step are not top-level statements
        ]
    );
    // The `for` step `i++` is the desugar too.
    let step = stmts(module(&su, "m"))
        .into_iter()
        .find_map(|s| match s {
            Stmt::For { step, .. } => Some(step),
            _ => None,
        })
        .expect("a for loop");
    assert!(step.is_compound_assign_desugar(), "{step:?}");
}

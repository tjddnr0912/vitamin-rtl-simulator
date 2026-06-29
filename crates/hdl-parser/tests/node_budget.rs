//! PARSE-CONCAT-CAP (2026-06-22, user decision = global node budget 1<<21):
//! the expression comma-list loops (`{a,b,…}` concat, `{n{…}}` replication value,
//! `f(a,b,…)` arg list, case labels) had NO length guard — only nesting depth was
//! capped (256). A single flat list (`{a,a,…,4M}`) therefore builds millions of
//! `Expr` nodes (80 B each) from a few MB of source, exhausting memory with no
//! diagnostic. A global parsed-expression-node budget turns that into a loud,
//! bounded parse error.

/// A concat with more than the node budget (2^21) elements must be a clean,
/// bounded parse error — not an OOM and not a silent success.
#[test]
fn huge_concat_is_bounded_parse_error() {
    let n: usize = 2_200_000; // > MAX_AST_NODES = 1<<21 = 2,097,152
    let mut elems = String::with_capacity(n * 2);
    for _ in 0..n {
        elems.push_str("a,");
    }
    elems.pop(); // drop the trailing comma
    let src = format!("module t; wire a; wire [7:0] w; assign w = {{{elems}}}; endmodule");
    let (toks, le) = hdl_lexer::lex(&src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (_su, pe) = hdl_parser::parse(&toks, &src);
    assert!(
        !pe.is_empty(),
        "a >2M-element concat must be a parse error (node budget), not a silent success"
    );
}

/// A realistic concat parses cleanly — the budget never trips on normal code.
#[test]
fn normal_concat_still_parses() {
    let src = "module t; wire a, b, c; wire [2:0] w; assign w = {a, b, c}; endmodule";
    let (toks, _) = hdl_lexer::lex(src);
    let (su, pe) = hdl_parser::parse(&toks, src);
    assert!(pe.is_empty(), "a 3-element concat is legal: {pe:?}");
    assert!(su.is_some());
}

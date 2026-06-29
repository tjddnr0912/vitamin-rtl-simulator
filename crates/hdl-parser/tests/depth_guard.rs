//! P2-5: deep expression nesting must yield a PARSE ERROR, not a stack
//! overflow (SIGSEGV with no diagnostic). The recursive-descent expr path is
//! depth-capped (512) — far above any real RTL, far below the thread stack.

#[test]
fn deep_paren_nesting_errors_cleanly() {
    let depth = 20_000;
    let src = format!(
        "module t; wire y; assign y = {}1{}; endmodule",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let (toks, le) = hdl_lexer::lex(&src);
    assert!(le.is_empty());
    let (_su, pe) = hdl_parser::parse(&toks, &src);
    assert!(
        !pe.is_empty(),
        "deep nesting must produce a parse error (not a crash)"
    );
}

/// Realistic nesting depths stay accepted.
#[test]
fn shallow_nesting_still_parses() {
    let depth = 100;
    let src = format!(
        "module t; wire y; assign y = {}1{}; endmodule",
        "(".repeat(depth),
        ")".repeat(depth)
    );
    let (toks, _) = hdl_lexer::lex(&src);
    let (su, pe) = hdl_parser::parse(&toks, &src);
    assert!(pe.is_empty(), "100-deep parens are legal: {pe:?}");
    assert!(su.is_some());
}

// ── STMT-DEPTH (2026-06-22 hostile-input hardening) ─────────────────────────
// Expression recursion is depth-capped (above), but STATEMENT recursion was
// not: deeply nested `begin`/`if`/`for` recurse `parse_statement` → block →
// `parse_statement` … with no guard, so a pathological source SIGABRTs with no
// diagnostic. Mirror the expression guard at statement granularity.

/// Deeply nested statement blocks must be a clean parse error, not a crash.
#[test]
fn deep_stmt_nesting_errors_cleanly() {
    let depth = 20_000;
    let src = format!(
        "module t; initial {}begin end{} endmodule",
        "begin ".repeat(depth),
        " end".repeat(depth)
    );
    let (toks, le) = hdl_lexer::lex(&src);
    assert!(le.is_empty());
    let (_su, pe) = hdl_parser::parse(&toks, &src);
    assert!(
        !pe.is_empty(),
        "deep statement nesting must produce a parse error (not a crash)"
    );
}

/// Realistic statement nesting depths stay accepted.
#[test]
fn shallow_stmt_nesting_still_parses() {
    let depth = 100;
    let src = format!(
        "module t; initial {}begin end{} endmodule",
        "begin ".repeat(depth),
        " end".repeat(depth)
    );
    let (toks, _) = hdl_lexer::lex(&src);
    let (su, pe) = hdl_parser::parse(&toks, &src);
    assert!(pe.is_empty(), "100-deep begin/end is legal: {pe:?}");
    assert!(su.is_some());
}

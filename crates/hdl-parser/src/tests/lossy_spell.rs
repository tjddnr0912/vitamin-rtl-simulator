//! `hdl_ast::LossySpell` as the parser writes it (§5.2 Q-LOSSYSPELL, unit U1): one pin per
//! region opener, per closing rule, per fill shape, per `let` head and per signing form, with
//! the negatives each rule must keep. No oracle: the record is which spellings the token
//! vector holds, not a value of the language; each expected bit is read off the row's rule
//! (W/lossy/plan2/PLAN2.txt section C, option (c) by RULINGS-plan R14, premise judge 2's
//! literal lines L3 and L4). A pin whose bit is an over-report says so with its reason.
//! Every source parsed through `bits` / `bits_loose` also passes `superset` (home rule H2(d)).
use super::*;

const P: u8 = 1; // param_xz
const U: u8 = 2; // unsized_xz
const S: u8 = 4; // sign_kw_param
const L: u8 = 8; // let_any
const F: u8 = 16; // fill_xz

fn mask(ls: LossySpell) -> u8 {
    [
        (ls.param_xz, P),
        (ls.unsized_xz, U),
        (ls.sign_kw_param, S),
        (ls.let_any, L),
        (ls.fill_xz, F),
    ]
    .iter()
    .filter(|(b, _)| *b)
    .fold(0, |m, (_, k)| m | k)
}

/// The unit's `LossySpell` items (0 or 1), the parse errors, and the unit itself.
fn parsed(src: &str) -> (Option<SourceUnit>, Vec<LossySpell>, usize) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, errs, _) = parse_with_warnings(&toks, src);
    let items: Vec<LossySpell> = su
        .iter()
        .flat_map(|u| u.items.iter())
        .filter_map(|i| match i {
            TopItem::LossySpell(ls) => Some(*ls),
            _ => None,
        })
        .collect();
    assert!(items.len() <= 1, "at most one record per unit: {items:?}");
    if let Some(u) = &su {
        let n = u.items.len();
        if !items.is_empty() {
            // Pushed before the `inside` record, which stays last.
            let at = u
                .items
                .iter()
                .position(|i| matches!(i, TopItem::LossySpell(_)));
            let inside = u
                .items
                .iter()
                .any(|i| matches!(i, TopItem::InsideNameUse(_)));
            assert_eq!(at, Some(n - 1 - usize::from(inside)), "the record's place");
        }
        superset(u, items.first().copied().unwrap_or_default());
    }
    (su, items, errs.len())
}

/// The bits of a source the parser ACCEPTS (no parse error).
fn bits(src: &str) -> u8 {
    let (su, items, errs) = parsed(src);
    assert_eq!(errs, 0, "`{src}` must parse");
    assert!(su.is_some());
    items.first().map_or(0, |ls| mask(*ls))
}

/// The bits whatever the parse said, and whether it reported an error.
fn bits_loose(src: &str) -> (u8, bool) {
    let (_, items, errs) = parsed(src);
    (items.first().map_or(0, |ls| mask(*ls)), errs > 0)
}

/// `scan_lossy` alone over a token snippet that is not a design: the depth rule over the
/// lexer's opener and closer kinds (premise judge 2, L4).
fn scan(src: &str) -> u8 {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    mask(lossy::scan_lossy(&toks, src))
}

/// Home rule H2(d), the SUPERSET test: every parameter holder of the parsed unit — a
/// `ParamDecl` (a header's `params`, a body `Param`), a `param_overrides` list, a
/// `DefparamItem`, a class's `params` — read through the derived `Debug` text, which
/// prints every field, so no holder position is skipped; a `Source` literal with an
/// `x X z Z ?` digit after its base in any of them while `param_xz` is false fails. A new
/// parse path that puts a value outside the regions fails here on its first source. It
/// cannot see a class specialization's actuals (substituted into the monomorphized body
/// by name, premise judge 2 L6) nor a class's own `#( )` defaults (the parser keeps the
/// template out of the unit: `ClassDecl.params` is empty on 6bc9babc): the class pins
/// guard those paths alone.
fn superset(u: &SourceUnit, ls: LossySpell) {
    let n = holder_xz_literals(&format!("{u:?}"));
    assert!(
        ls.param_xz || n == 0,
        "{n} x / z literal(s) in a parameter holder, param_xz false"
    );
}

/// How many `Source` x / z literals sit inside a parameter holder of a unit's `Debug` text.
fn holder_xz_literals(dbg: &str) -> usize {
    const HOLDERS: [&str; 4] = [
        "params: [",
        "Param(ParamDecl {",
        "param_overrides: [",
        "Defparam(DefparamItem {",
    ];
    let mut n = 0;
    for h in HOLDERS {
        let mut from = 0;
        while let Some(at) = dbg[from..].find(h) {
            let open = from + at + h.len() - 1;
            let close = balanced_end(dbg, open);
            n += xz_source_literals(&dbg[open..close]);
            from = open + 1;
        }
    }
    n
}

/// The index just past the bracket matching the one at `open` (`[` / `{`), skipping the
/// insides of `"…"` strings.
fn balanced_end(s: &str, open: usize) -> usize {
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    for (i, c) in s[open..].char_indices() {
        if in_str {
            match (esc, c) {
                (true, _) => esc = false,
                (false, '\\') => esc = true,
                (false, '"') => in_str = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => in_str = true,
            '[' | '{' | '(' => depth += 1,
            ']' | '}' | ')' => {
                depth -= 1;
                if depth == 0 {
                    return open + i + 1;
                }
            }
            _ => {}
        }
    }
    s.len()
}

fn xz_source_literals(s: &str) -> usize {
    s.match_indices("IntLit { kind: ")
        .filter(|(at, _)| {
            let rest = &s[*at..];
            let end = rest.find('}').unwrap_or(rest.len());
            let lit = &rest[..end];
            let raw = lit
                .split("raw: \"")
                .nth(1)
                .and_then(|r| r.split('"').next());
            lit.contains("origin: Source")
                && raw.is_some_and(|r| {
                    let after = r.split_once('\'').map_or("", |(_, a)| a);
                    let after = after.strip_prefix(['s', 'S']).unwrap_or(after);
                    let digits = match after.chars().next() {
                        Some('d' | 'D' | 'b' | 'B' | 'o' | 'O' | 'h' | 'H') => &after[1..],
                        _ => after,
                    };
                    digits.contains(['x', 'X', 'z', 'Z', '?'])
                })
        })
        .count()
}

// ───────────────────────────── the superset test's own controls ─────────────────────────────

#[test]
fn superset_walk_finds_a_holder_literal_and_fails_without_the_bit() {
    let src = "module m; localparam P = 4'b1x01; endmodule";
    let (toks, _) = hdl_lexer::lex(src);
    let (su, _, _) = parse_with_warnings(&toks, src);
    let u = su.unwrap();
    assert_eq!(holder_xz_literals(&format!("{u:?}")), 1);
    // A record with the bit cleared must fail the check (planted control).
    let r = std::panic::catch_unwind(|| superset(&u, LossySpell::default()));
    assert!(
        r.is_err(),
        "the superset check must fail on a cleared param_xz"
    );
    // Each holder kind is found: header, ordered and named override, defparam. (A class's
    // `#( )` parameters reach no holder either: the parser keeps the template out of the
    // unit, `ClassDecl.params` is empty — `param_xz_class_default_and_specialization`
    // guards that path.)
    for src in [
        "module m #(parameter P = 4'b1x01) (); endmodule",
        "module s #(parameter P = 0) (); endmodule module m; s #(.P(4'b1x01)) u(); endmodule",
        "module s #(parameter P = 0) (); endmodule module m; s #(4'b1x01) u(); endmodule",
        "module s; parameter P = 0; endmodule module m; s u(); defparam u.P = 4'b1x01; endmodule",
    ] {
        let (toks, _) = hdl_lexer::lex(src);
        let (su, _, _) = parse_with_warnings(&toks, src);
        let n = holder_xz_literals(&format!("{:?}", su.unwrap()));
        assert_eq!(n, 1, "`{src}`");
    }
}

#[test]
fn superset_walk_ignores_a_literal_outside_every_holder() {
    for src in [
        "module m; logic [3:0] w = 4'b1x01; endmodule",
        "module s (input [3:0] a); endmodule module m; s u(4'b1x01); endmodule",
    ] {
        let (toks, _) = hdl_lexer::lex(src);
        let (su, _, _) = parse_with_warnings(&toks, src);
        assert_eq!(
            holder_xz_literals(&format!("{:?}", su.unwrap())),
            0,
            "`{src}`"
        );
    }
}

// ───────────────────────────── param_xz: every region opener ─────────────────────────────

#[test]
fn param_xz_each_region_opener() {
    let sub = "module s #(parameter P = 0) (); endmodule ";
    for src in [
        // r1 `#(`: a header with and without the keyword
        "module m #(parameter P = 4'b1x01) (); endmodule".to_string(),
        "module m #(P = 4'b1x01) (); endmodule".to_string(),
        // r2 the four keywords in a body
        "module m; parameter P = 4'b1x01; endmodule".to_string(),
        "module m; localparam P = 4'b1x01; endmodule".to_string(),
        "module m; specparam P = 4'b1x01; endmodule".to_string(),
        format!("{sub}module m; s u(); defparam u.P = 4'b1x01; endmodule"),
        // r1: an ordered and a named override, a bind
        format!("{sub}module m; s #(4'b1x01) u(); endmodule"),
        format!("{sub}module m; s #(.P(4'b1x01)) u(); endmodule"),
        format!("{sub}module m; endmodule bind m s #(.P(4'b1x01)) u();"),
        // a package, an interface, a generate block, `$unit`
        "package p; localparam P = 4'b1x01; endpackage".to_string(),
        "interface i; parameter P = 4'b1x01; endinterface".to_string(),
        "module m; if (1) begin : g localparam P = 4'b1x01; end endmodule".to_string(),
        "localparam P = 4'b1x01; module m; endmodule".to_string(),
        // an array parameter's `'{…}` value (`'{` is `'` + `{`)
        "module m; localparam logic [3:0] A [2] = '{4'b0, 4'b1x01}; endmodule".to_string(),
        // split across lines
        "module m #(\n  parameter\n  P\n  =\n  4'b1x01\n) ();\nendmodule".to_string(),
        // a defparam in a generate block
        format!("{sub}module m; if (1) begin : g s u(); defparam u.P = 4'b1x01; end endmodule"),
    ] {
        assert_eq!(bits(&src), P, "`{src}`");
    }
}

#[test]
fn param_xz_class_default_and_specialization() {
    // Premise judge 1 L1: a class `#( )` default, and b4_classcollide's spelling (the class
    // specialization's actuals reach no AST holder the superset test sees: this pin is that
    // path's only guard, premise judge 2 L6).
    assert_eq!(
        bits("class C #(parameter logic [7:0] R = 8'b1x01_0000); endclass"),
        P
    );
    let collide = "module t;
  class C #(parameter logic [7:0] P = 0); function logic [7:0] v(); return P; endfunction endclass
  C #(8'b0000101) a; C #(8'b0000_1?01) b;
  initial begin a = new; b = new; #1 $display(\"A a=%b b=%b\", a.v(), b.v()); end
  initial #100 $finish;
endmodule";
    assert_eq!(bits(collide), P);
}

#[test]
fn param_xz_type_parameter_ranges_over_report() {
    // A type's range inside a region is a region literal: true, an over-report (PLAN2
    // section C, "WHAT EACH BIT DOES NOT CLAIM").
    let (b, _) = bits_loose("module m #(parameter type T = logic [4'b1x01:0]) (); endmodule");
    assert_eq!(b & P, P);
    let (b, _) = bits_loose(
        "module s #(parameter type T = logic) (); endmodule \
         module m; s #(.T(logic [4'b1x01:0])) u(); endmodule",
    );
    assert_eq!(b & P, P);
}

#[test]
fn param_xz_nested_brackets_stay_inside() {
    let sub = "module s #(parameter A = 0, parameter B = 0) (); endmodule ";
    for src in [
        "module m; localparam P = {4'b0, {2{1'b0}}, $clog2(4), 4'b1x01}; endmodule".to_string(),
        format!("{sub}module m; s #(.A({{1'b0, (2)}}), .B(4'b1x01)) u(); endmodule"),
        "module m #(parameter A = {1'b0}, parameter B = 4'b1x01) (); endmodule".to_string(),
        "module m #(parameter A = (1), B = [1:0]'(2), C = 4'b1x01) (); endmodule".to_string(),
    ] {
        let (b, _) = bits_loose(&src);
        assert_eq!(b & P, P, "`{src}`");
    }
}

#[test]
fn param_xz_opener_kinds_scan_level() {
    // The depth rule counts `(` `[` `[*` `[->` `[=` `{` up and `)` `]` `}` down (premise
    // judge 2 L4). The SVA repetition openers never sit inside a region in an accepted design
    // (none found), so they are pinned on token snippets: a missing opener would let its `]`
    // close the `#(` region before the literal.
    for src in [
        "#( a [*2] , 4'b1x01 )",
        "#( a [->2] , 4'b1x01 )",
        "#( a [=2] , 4'b1x01 )",
        "#( a [2] , 4'b1x01 )",
        "#( a {2} , 4'b1x01 )",
        "#( (a) , 4'b1x01 )",
        "parameter P = ( a ; 4'b1x01 ) ;",
        "parameter P = { a ; 4'b1x01 } ;",
    ] {
        assert_eq!(scan(src) & P, P, "`{src}`");
    }
    // `[+]` is one self-closed token, `##` no opener.
    assert_eq!(scan("#( a [+] , 4'b1x01 )") & P, P);
    assert_eq!(scan("a ##( 1 ) 4'b1x01 ;") & P, 0);
}

#[test]
fn param_xz_regions_close() {
    let sub = "module s #(parameter P = 0) (input [3:0] a); endmodule ";
    for src in [
        // the `;` closes a keyword region
        "module m; parameter P = 1; logic [3:0] w = 4'b1x01; endmodule".to_string(),
        "module m; localparam A = 1, B = 2; assign w = 4'b1x01; endmodule".to_string(),
        // the `)` closes `#(`: an instance override, then a literal outside
        format!("{sub}module m; wire [3:0] w; s #(.P(1)) u(w); assign w = 4'b1x01; endmodule"),
        format!("{sub}module m; s #(1) u(4'b1x01); endmodule"),
        // a header's `)` closes the keyword inside it: a port range after the header
        // (ansi.KZ.m's shape, PLAN2 B2) is not a parameter position
        "module m #(parameter P = 1) (input logic [3:0] p); assign p = 4'b1x01; endmodule"
            .to_string(),
        // `#` with no `(` opens nothing: a delay `#1`
        "module m; logic [3:0] a; initial #1 a = 4'b1x01; endmodule".to_string(),
    ] {
        let (b, _) = bits_loose(&src);
        assert_eq!(b & P, 0, "`{src}`");
    }
    let (b, _) = bits_loose("module m #(parameter P = 1) (input logic [4'b1x01:0] p); endmodule");
    assert_eq!(b & P, 0, "a port range after the header");
}

#[test]
fn param_xz_delay_paren_is_an_over_report() {
    // `#( … )` as a delay is a region: true, pinned with the reason (the scan reads tokens,
    // not the grammar; PLAN2 B2 measured 0 such units in every population).
    assert_eq!(
        bits("module m; logic a; initial #(1'bx) a = 1; endmodule"),
        P
    );
}

#[test]
fn param_xz_question_digit_not_conditional() {
    assert_eq!(bits("module m; localparam P = 4'b1?01; endmodule"), P);
    assert_eq!(bits("module m; localparam Q = 1 ? 2 : 3; endmodule"), 0);
    assert_eq!(bits("module m; localparam R = 'd?; endmodule"), P | U);
    // a sized `?` outside every region sets nothing
    assert_eq!(bits("module m; wire [3:0] w = 4'b1?01; endmodule"), 0);
}

#[test]
fn param_xz_spelling_variants() {
    for (src, want) in [
        ("module m; localparam A = 4 'b 1X01; endmodule", P),
        ("module m; localparam B = 8'HZ_Z; endmodule", P),
        ("module m; localparam C = 8'b1_x_0_1_0000; endmodule", P),
        ("module m; localparam D = 'B X; endmodule", P | U),
        ("module m; localparam E = 'h_z; endmodule", P | U),
        ("module m; localparam G = 8'sbx; endmodule", P),
        (
            "module m; localparam H = 4'b1010, I = 8'hFF, J = 'd9; endmodule",
            0,
        ),
        ("module m; localparam K = 'x; endmodule", P),
    ] {
        assert_eq!(bits(src), want, "`{src}`");
    }
}

// ───────────────────────────── unsized_xz ─────────────────────────────

#[test]
fn unsized_xz_based_not_fill() {
    for (src, want) in [
        ("module m; wire [127:0] w = 'bx; endmodule", U),
        ("module m; wire [127:0] w = 'hz; endmodule", U),
        ("module m; wire [127:0] w = 'sb1x; endmodule", U),
        // `'sx` / `'sz` are not fills (M12): unsized x / z literals
        ("module m; logic [127:0] w; initial w = 'sx; endmodule", U),
        ("module m; logic [127:0] w; initial w = 'sz; endmodule", U),
        ("module m; wire [127:0] w = 'b1010; endmodule", 0),
        ("module m; wire [127:0] w = 128'bx; endmodule", 0),
        ("module m; logic [127:0] w; initial w = '1; endmodule", 0),
        ("module m; logic [127:0] w; initial w = '0; endmodule", 0),
    ] {
        assert_eq!(bits(src), want, "`{src}`");
    }
}

// ───────────────────────────── fill_xz under option (c) ─────────────────────────────

#[test]
fn fill_xz_plain_shape_is_not_counted() {
    // `=` / `<=` FILL `;`: PR16, measured right or loud on every left side that takes it.
    for src in [
        "module m; logic [69:0] q; initial q = 'x; endmodule",
        "module m; logic [69:0] q; logic c; always @(posedge c) q <= 'z; endmodule",
        "module m; logic [69:0] v = 'x; endmodule",
        "module m; wire [69:0] w; assign w = 'z; endmodule",
        // EH1's one form, `Q <= 'x;` (41 of 41 sites)
        "module m; logic [31:0] Q; logic c; always @(posedge c) begin Q <= 'x; end endmodule",
        // the relational `<=` operand ends with the same tokens: measured right (p9)
        "module m; logic b1; initial b1 = b1 <= 'x; endmodule",
    ] {
        assert_eq!(bits(src), 0, "`{src}`");
    }
}

#[test]
fn fill_xz_every_other_position_counts() {
    for src in [
        "module m; function automatic logic [69:0] f(); return 'x; endfunction endmodule",
        "module m; function automatic logic [69:0] f(input logic [69:0] a); return a; \
         endfunction logic [69:0] r; initial r = f('x); endmodule",
        "module m; logic [69:0] r; logic [3:0] a; initial r = {'x, a}; endmodule",
        "module m; logic [69:0] r, w; logic c; initial r = c ? 'x : w; endmodule",
        "module m; logic [69:0] r; logic [1:0] s; initial case (s) 'x: r = 0; default: r = 1; \
         endcase endmodule",
        "module m; logic [69:0] r; logic b; initial b = r inside {'z}; endmodule",
        // k9_qxz / k10b_class's families (silently wrong today, PLAN2 section A)
        "module m; logic [69:0] q[$]; initial q.push_back('z); endmodule",
        // a fill inside a non-plain `let` body
        "module m; logic c; let T(c) = c ? 'x : 'z; endmodule",
    ] {
        let b = bits(src);
        assert_eq!(b & F, F, "`{src}`");
        assert_eq!(b & (P | U | S), 0, "`{src}`");
    }
}

#[test]
fn fill_xz_let_body_is_held_by_let_any() {
    // `let LX = 'x;` has the plain shape, so the fill is not counted; the `let` is
    // (b3_letfill: silently wrong today, PLAN2 section A's table, (c) needs let_any).
    assert_eq!(bits("module m; let LX = 'x; endmodule"), L);
    let letfill = "module t; logic [69:0] w1, w2, w3, w4; logic c;
  let LX = 'x; let ID(q) = q; let A(q) = q | 70'h0; let T(q) = c ? q : 70'h0;
  initial begin c = 1; w1 = LX; w2 = ID('z); w3 = A('z); w4 = T('x); end
endmodule";
    assert_eq!(bits(letfill), L | F);
}

#[test]
fn zero_one_fills_set_no_bit() {
    // The `'0` / `'1` twin is silently wrong in the same positions and is not an x / z
    // spelling: no bit under any option; `false` says nothing about it (filed).
    assert_eq!(
        bits("module m; logic [69:0] q[$]; initial begin q.push_back('1); q.push_front('0); end endmodule"),
        0
    );
}

#[test]
fn fill_rule_is_the_token_shape() {
    // fill_counts reads the previous and next token only.
    assert_eq!(scan("a = 'x ;") & F, 0);
    assert_eq!(scan("a <= 'z ;") & F, 0);
    assert_eq!(scan("a = 'x , b = 1 ;") & F, F);
    assert_eq!(scan("a = ( 'x ) ;") & F, F);
    assert_eq!(scan("'x ;") & F, F);
    assert_eq!(scan("a = 'x") & F, F);
    assert_eq!(scan("a == 'x ;") & F, F);
}

// ───────────────────────────── let_any ─────────────────────────────

#[test]
fn let_any_every_declaration_head() {
    for src in [
        "module m; logic a, b; let L = a + b; endmodule",
        "module m; let L(int x) = x; endmodule",
        "module m; let L(x) = x; endmodule",
        "module m; let L = 1; endmodule",
        "module m; let \\L$x = 1; endmodule",
        "module m;\n  let\n    L\n    = 1;\nendmodule",
    ] {
        assert_eq!(bits(src), L, "`{src}`");
    }
    // A `let` in a function body is E2002 today, still a head: over-report.
    let (b, _) = bits_loose("module m; function f(); let L = 1; endfunction endmodule");
    assert_eq!(b & L, L);
}

#[test]
fn let_any_negatives() {
    for src in [
        "module m; logic outlet; logic \\let ; endmodule",
        "module m; initial $display(\"let x = 1\"); endmodule",
        "module m; // let x = 1;\n /* let y = 2; */ endmodule",
        "module m; typedef logic letter_t; letter_t x; endmodule",
    ] {
        assert_eq!(bits(src), 0, "`{src}`");
    }
    // the word `let` with no name after it is not a head
    let (b, _) = bits_loose("module m; wire let; endmodule");
    assert_eq!(b & L, 0);
    assert_eq!(scan("let ;") & L, 0);
    assert_eq!(scan("let 1") & L, 0);
    assert_eq!(scan("let x") & L, L);
    assert_eq!(scan("let \\x ") & L, L);
}

#[test]
fn is_let_head_is_the_dispatch_guard() {
    // The guard and the scan read one fn; the guard's `let` still parses a declaration.
    let src = "module m; let L = 1; endmodule";
    let (toks, _) = hdl_lexer::lex(src);
    let i = toks
        .iter()
        .position(|t| &src[t.span.clone()] == "let")
        .unwrap();
    assert!(lossy::is_let_head(&toks, src, i));
    assert!(!lossy::is_let_head(&toks, src, i + 1));
    let (su, errs) = p(src);
    assert!(errs.is_empty());
    assert!(matches!(
        first_module(&su.unwrap()).body[0],
        ModuleItem::LetDecl(_)
    ));
}

// ───────────────────────────── sign_kw_param ─────────────────────────────

#[test]
fn sign_kw_param_unranged_only() {
    for (src, want) in [
        ("module m; parameter signed N = 5; endmodule", S),
        ("module m; localparam unsigned U = 4'sd8; endmodule", S),
        ("module m #(parameter signed N = 1) (); endmodule", S),
        ("package p; localparam signed N = -1; endpackage", S),
        ("module m; parameter signed A = 1, B = 2; endmodule", S),
        // ranged and typed forms: out (f7_sgnrange, b4_sgnrange right)
        ("module m; parameter signed [3:0] N = 1; endmodule", 0),
        (
            "module m; localparam unsigned [3:0] U = 4'sd8; endmodule",
            0,
        ),
        ("module m; parameter int unsigned W = 1; endmodule", 0),
        ("module m; parameter integer signed I = 1; endmodule", 0),
        ("module m; parameter logic signed L = 1; endmodule", 0),
        ("module m; parameter N = 1; endmodule", 0),
    ] {
        assert_eq!(bits(src), want, "`{src}`");
    }
}

#[test]
fn sign_kw_param_survives_later_units() {
    // Written by `|=` only, in no scope snapshot: a later module does not clear it.
    assert_eq!(
        bits("module a; parameter signed N = 1; endmodule module b; parameter M = 2; endmodule"),
        S
    );
}

// ───────────────────────────── the record's carrier ─────────────────────────────

#[test]
fn all_false_unit_carries_no_record() {
    // A unit that spells none keeps PRE's items exactly (its `.vu` body is PRE's).
    let src = "module m; logic [31:0] Q; logic c; always @(posedge c) Q <= 'x; endmodule";
    let (su, items, errs) = parsed(src);
    assert_eq!(errs, 0);
    assert!(items.is_empty());
    let u = su.unwrap();
    assert_eq!(u.items.len(), 1);
    assert!(matches!(u.items[0], TopItem::Module(_)));
}

#[test]
fn no_record_without_a_unit() {
    // Only into a unit that exists: a source with no design unit stays `None`.
    let src = "function automatic logic [3:0] f(); return 'x; endfunction";
    let (su, items, errs) = parsed(src);
    assert_eq!(errs, 0);
    assert!(su.is_none());
    assert!(items.is_empty());
}

#[test]
fn record_sits_before_the_inside_record() {
    let src = "module m; logic inside; localparam P = 4'b1x01; endmodule";
    let (su, items, _) = parsed(src);
    assert_eq!(items.len(), 1);
    let u = su.unwrap();
    assert!(matches!(u.items.last(), Some(TopItem::InsideNameUse(_))));
}

#[test]
fn config_use_clause_is_refused() {
    // Annex A's one parameter value outside both regions, a config's `use .P(v)`
    // (ieee1800-2017.txt:41388-41391): vita refuses `config` (E2002). A row that makes
    // `config` parse adds its use clause to scan_lossy's regions in the same commit
    // (premise judge 2 L3, W/lossy/premise2/cells/t2_config).
    let sub =
        "module sub #(parameter logic [7:0] W = 8'd1); endmodule module t; sub u(); endmodule\n";
    for cfg in [
        "config cfg; design t; instance t.u use #(.W(8'b1x01_0000)); endconfig",
        "config cfg; design t; instance t.u use .W(8'b1x01_0000); endconfig",
    ] {
        let (_, refused) = bits_loose(&format!("{sub}{cfg}"));
        assert!(refused, "`{cfg}` must stay refused");
    }
}

#[test]
fn plain_fill_on_a_refused_left_side_stays_refused() {
    // Premise judge 2 L2: PR16 holds on these left sides only while they are refused;
    // opening one re-measures the plain fill there (T7). Each is a parse error today (the
    // tri1 / wor initializer is elaborate's E3009: a cli pin, not here).
    for src in [
        "module m; logic [69:0] a, b; initial {>>{a, b}} = 'x; endmodule",
        "module m; struct { logic [69:0] a = 'x; } s; endmodule",
        "module m; class C; static logic [69:0] f = 'x; endclass endmodule",
        "module m; var logic [69:0] v = 'x; endmodule",
        "module m; const logic [69:0] c = 'x; endmodule",
    ] {
        let (_, refused) = bits_loose(src);
        assert!(refused, "`{src}` must stay refused");
    }
}

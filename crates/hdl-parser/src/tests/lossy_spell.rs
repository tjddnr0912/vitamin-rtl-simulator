//! `hdl_ast::LossySpell` as the parser writes it (§5.2 Q-LOSSYSPELL, unit U1): one pin per
//! region opener, per closing rule, per fill shape, per `let` head and per signing form, with
//! the negatives each rule must keep. No oracle: the record is which spellings the token
//! vector holds, not a value of the language; each expected bit is read off the row's rule
//! (W/lossy/plan2/PLAN2.txt section C, option (c) by RULINGS-plan R14, premise judge 2's
//! literal lines L3 and L4). A pin whose bit is an over-report says so with its reason.
//! Every source parsed through `bits` / `bits_loose` also passes `superset` (home rule H2(d)).
//! Unit U3 (the lexer's x / z literal kinds gate the pass ; `let_any` is the parser's note at
//! `parse_let_decl`, RULINGS-plan R23) moved the function-body `let` and the head snippets
//! and adds the last section: the production pair (`lex_facts`, `parse_lexed`), the early
//! return, `let` as a type name (premise judge 3 L7), the attribute-only literal.
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
    scan_marked(src, &[])
}

/// `scan_lossy` over a token snippet with the given right-side marks (token indices).
fn scan_marked(src: &str, marks: &[usize]) -> u8 {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    mask(lossy::scan_lossy(&toks, src, marks))
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
        // a localparam value is no mark site (main's K1, fix N4): its plain fill counts
        ("module m; localparam K = 'x; endmodule", P | F),
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
    // `=` / `<=` FILL `;` where the parser took the fill as the whole right side of an
    // assignment or initializer (fix N4: the shape AND the parser's mark): PR16, measured
    // right or loud on every left side that takes it. The relational tail `b1 = b1 <= 'x;`
    // (p9) has the same tokens and is counted since fix N4 (`fill_xz_relational_matrix`).
    for src in [
        "module m; logic [69:0] q; initial q = 'x; endmodule",
        "module m; logic [69:0] q; logic c; always @(posedge c) q <= 'z; endmodule",
        "module m; logic [69:0] v = 'x; endmodule",
        "module m; wire [69:0] w; assign w = 'z; endmodule",
        // EH1's one form, `Q <= 'x;` (41 of 41 sites)
        "module m; logic [31:0] Q; logic c; always @(posedge c) begin Q <= 'x; end endmodule",
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
fn fill_xz_let_body_counts_and_let_any_is_set() {
    // `let LX = 'x;` has the plain shape but a `let` body is no mark site (main's K1,
    // RULINGS L1-10): the fill counts, and the `let` is noted too (b3_letfill: silently
    // wrong today, PLAN2 section A's table).
    assert_eq!(bits("module m; let LX = 'x; endmodule"), L | F);
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
fn fill_rule_shape_alone_exempts_nothing() {
    // Pin (e): `scan_lossy` with an EMPTY mark list — the token shape alone exempts no fill
    // (fix N4: the exemption is the shape AND the parser's mark at the fill's index).
    assert_eq!(scan("a = 'x ;") & F, F);
    assert_eq!(scan("a <= 'z ;") & F, F);
    assert_eq!(scan("a = 'x , b = 1 ;") & F, F);
    assert_eq!(scan("a = ( 'x ) ;") & F, F);
    assert_eq!(scan("'x ;") & F, F);
    assert_eq!(scan("a = 'x") & F, F);
    assert_eq!(scan("a == 'x ;") & F, F);
    // ... and with the parser's mark at the fill's index (2) the shape exempts it ; a mark
    // elsewhere, or a mark where the shape fails, exempts nothing.
    assert_eq!(scan_marked("a = 'x ;", &[2]) & F, 0);
    assert_eq!(scan_marked("a <= 'z ;", &[2]) & F, 0);
    assert_eq!(scan_marked("a = 'x ;", &[0, 1, 3]) & F, F);
    assert_eq!(scan_marked("a = 'x , b = 1 ;", &[2]) & F, F);
    assert_eq!(scan_marked("a == 'x ;", &[2]) & F, F);
    assert_eq!(scan_marked("a = # 1 'x ;", &[4]) & F, F);
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
    // A `let` in a function body is E2002 today and never reaches `parse_let_decl` (the
    // module-item dispatch is its one door): false since U3 (U1's token scan said true, an
    // over-report). The refused unit reaches no reader (the front end returns `None` on a
    // parse error ; W/lossy/plan3/PLAN3.txt section C (ii)).
    let (b, refused) = bits_loose("module m; function f(); let L = 1; endfunction endmodule");
    assert!(refused);
    assert_eq!(b & L, 0);
    // A head the dispatch took and whose body the parse then refuses stays noted
    // (`note_let_decl` is `parse_let_decl`'s first line): over-report, loud either way.
    let (b, refused) = bits_loose("module m; let L = ; endmodule");
    assert!(refused);
    assert_eq!(b & L, L);
}

#[test]
fn let_any_negatives() {
    for src in [
        "module m; logic outlet; logic \\let ; endmodule",
        "module m; initial $display(\"let x = 1\"); endmodule",
        "module m; // let x = 1;\n /* let y = 2; */ endmodule",
        "module m; typedef logic letter_t; letter_t x; endmodule",
        // A module whose name starts with `let`, instantiated: the dispatch tests the
        // `let` head before the bare-identifier instance arm, so only the whole word
        // `let` may take it (the U3 mutant `starts_with("let")` survived without it).
        "module letter; endmodule module m; letter u (); endmodule",
    ] {
        assert_eq!(bits(src), 0, "`{src}`");
    }
    // the word `let` with no name after it is not a head
    let (b, _) = bits_loose("module m; wire let; endmodule");
    assert_eq!(b & L, 0);
    // The head predicate on token snippets (the dispatch guard ; the scan has no `let`
    // arm since U3, so it sets no L on any of them).
    assert!(!head("let ;"));
    assert!(!head("let 1"));
    assert!(head("let x"));
    assert!(head("let \\x "));
    for src in ["let ;", "let 1", "let x", "let \\x "] {
        assert_eq!(scan(src) & L, 0, "`{src}`");
    }
}

/// `is_let_head` at the first token of a snippet.
fn head(src: &str) -> bool {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    lossy::is_let_head(&toks, src, 0)
}

#[test]
fn is_let_head_is_the_dispatch_guard() {
    // The dispatch guard; the guard's `let` still parses a declaration.
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

// ───────────────── U3: the gate, the early return, the parser's `let` note ─────────────────

/// The bits through the PRODUCTION pair: `hdl_lexer::lex_facts` then `parse_lexed` with the
/// fact the lexer returned (cli `frontend.rs`), and the parse error count.
fn bits_lexed(src: &str) -> (u8, usize) {
    let (toks, le, facts) = hdl_lexer::lex_facts(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    let (su, errs, _) = parse_lexed(&toks, src, facts);
    let found: Vec<LossySpell> = su
        .iter()
        .flat_map(|u| u.items.iter())
        .filter_map(|i| match i {
            TopItem::LossySpell(ls) => Some(*ls),
            _ => None,
        })
        .collect();
    (found.first().map_or(0, |ls| mask(*ls)), errs.len())
}

#[test]
fn let_as_a_type_name_builds_no_let_declaration() {
    // Premise judge 3 L7 (RULINGS-plan R23): vita takes the reserved word `let` as a type
    // name, these units build no `LetDecl`, so `let_any` is false. U2's token scan said, on
    // each (W/lossy/premise3/out/k*.lb_base.out): "BITS param_xz=0 unsized_xz=0
    // sign_kw_param=0 let_any=1 fill_xz=0 items=1 parse_errors=0".
    for (id, src) in [
        (
            "k1_fnlocal",
            include_str!("../../tests/testdata/lossy_u3/k1_fnlocal.sv"),
        ),
        (
            "k2_port",
            include_str!("../../tests/testdata/lossy_u3/k2_port.sv"),
        ),
        (
            "k3_block",
            include_str!("../../tests/testdata/lossy_u3/k3_block.sv"),
        ),
    ] {
        assert_eq!(bits(src), 0, "{id}");
        assert_eq!(bits_lexed(src), (0, 0), "{id}");
    }
}

#[test]
fn let_note_survives_later_units() {
    // The second parser bool, like the first: `|=` only, in no scope snapshot.
    assert_eq!(
        bits("module a; let L = 1; endmodule module b; logic x; endmodule"),
        L
    );
    assert_eq!(
        bits("package p; let L = 1; endpackage module b; endmodule"),
        L
    );
}

#[test]
fn scan_runs_past_two_bits_until_all_three() {
    // The pass returns only when param_xz, unsized_xz and fill_xz are all set: each pair
    // first, the third after it.
    for (src, want) in [
        // param_xz and unsized_xz by one token, the counted fill later
        (
            "module m; localparam P = 'bx; logic [69:0] r; logic [3:0] a; initial r = {'x, a}; endmodule",
            P | U | F,
        ),
        // param_xz and fill_xz first (a fill in a region counts), unsized_xz later
        (
            "module m; localparam Q = {'z, 4'b0}; wire [127:0] w = 'hz; endmodule",
            P | U | F,
        ),
        // unsized_xz and fill_xz outside every region, param_xz later
        (
            "module m; wire [127:0] w = 'hz; logic [69:0] r; initial r = {'x, 70'h0}; localparam R = 4'b1x01; endmodule",
            P | U | F,
        ),
        // the same three, the sized one last
        (
            "module m; wire [127:0] w = 'hz; logic [69:0] r; initial r = {'x, 70'h0}; endmodule module n; localparam R = 4'b10?1; endmodule",
            P | U | F,
        ),
    ] {
        assert_eq!(bits(src), want, "`{src}`");
        assert_eq!(bits_lexed(src), (want, 0), "`{src}`");
    }
}

#[test]
fn attribute_only_literal_runs_the_pass_and_finds_none() {
    // b1_attr (premise judge 3 section B): every x / z literal sits in an attribute
    // instance, which the lexer strips. Its fact is true, the token vector holds none, the
    // record is all-false either way (U2: "BITS param_xz=0 unsized_xz=0 sign_kw_param=0
    // let_any=0 fill_xz=0 items=0 parse_errors=0").
    let src = include_str!("../../tests/testdata/lossy_u3/b1_attr.sv");
    let (toks, _, facts) = hdl_lexer::lex_facts(src);
    assert!(facts.xz_literal());
    assert!(!hdl_lexer::LexFacts::of_tokens(&toks).xz_literal());
    assert_eq!(bits_lexed(src), (0, 0));
    assert_eq!(bits(src), 0);
}

#[test]
fn a_clean_unit_skips_the_pass() {
    // No x / z literal: the fact is false and the record holds only the parser's notes.
    let src = "module m; parameter signed N = 4'b1010; let L = 8'hFF; logic [7:0] q; initial q = '1; endmodule";
    let (toks, _, facts) = hdl_lexer::lex_facts(src);
    assert!(!facts.xz_literal());
    assert_eq!(
        parse_lexed(&toks, src, facts),
        parse_with_warnings(&toks, src)
    );
    assert_eq!(bits_lexed(src), (S | L, 0));
}

// ──────────── fix N4: the exemption is the parser's mark (§5.2 Q-LOSSYSPELL U4) ────────────
//
// The boundary's soundness lens (N4) found the token shape `=` / `<=` FILL `;` exempting a
// fill that is a relational operand (`b = a <= 'z;`), a parameter value, a defparam value, a
// `let` body: positions PR16 never measured. Since U4 a fill is exempt only when the shape
// holds AND the parser pushed the fill's token index when it took the right side of an
// assignment or initializer at one of its 8 sites (`lossy.rs` `assign_rhs`; W/lossy/fixn4/
// plan/PLAN.txt section 1, premise judge W/lossy/fixn4/premise L1-L9, main's K1). Each pin's
// "before" bit is the frozen printer's (lossy_bits 1228f9dc, W/lossy/fixn4/{plan,premise}/cells).

/// Every source of a matrix: (label, source, want, parse accepted), the mismatches listed.
fn mismatches(cases: &[(String, String, u8)]) -> Vec<String> {
    cases
        .iter()
        .filter_map(|(label, src, want)| {
            let (got, refused) = bits_loose(src);
            (refused || got != *want)
                .then(|| format!("{label}: got {got}, want {want}, refused {refused}: `{src}`"))
        })
        .collect()
}

/// Pin (a)'s four spellings of the relational operand.
const REL_FORMS: [(&str, &str); 4] = [
    ("bare", "a <= 'z"),
    ("whole paren", "(a <= 'z)"),
    ("fill paren", "a <= ('z)"),
    ("operand paren", "((a) <= 'z)"),
];

/// Pin (a)'s positions, `@R` the relational operand. The first eighteen read 0 before U4 in
/// the bare spelling (the plan's cells r1-r8, r11 ; premise judge's d05-d08, d11 ; L5) ; a
/// parenthesized spelling never had the shape.
const REL_POSITIONS: [(&str, &str); 19] = [
    (
        "S1 blocking",
        "module m; logic [99:0] a; logic b; initial b = @R; endmodule",
    ),
    (
        "S2 nonblocking",
        "module m; logic [99:0] a; logic q; initial q <= @R; endmodule",
    ),
    (
        "S5 continuous",
        "module m; logic [99:0] a; wire y; assign y = @R; endmodule",
    ),
    (
        "S6 initializer",
        "module m; logic [99:0] a; logic b = @R; endmodule",
    ),
    (
        "return",
        "module m; logic [99:0] a; function automatic logic f(); return @R; endfunction endmodule",
    ),
    (
        "for condition",
        "module m; logic [99:0] a; int i; initial for (i = 0; @R; i = i + 1) ; endmodule",
    ),
    (
        "generate-for condition",
        "module m; logic [99:0] a; for (genvar g = 0; @R; g++) begin : b end endmodule",
    ),
    (
        "wait statement",
        "module m; logic [99:0] a; logic b, c; initial wait (c) b = @R; endmodule",
    ),
    (
        "property body",
        "module m; logic c; logic [99:0] a; property p; @(posedge c) @R; endproperty \
         assert property (p); endmodule",
    ),
    (
        "sequence body",
        "module m; logic c; logic [99:0] a; sequence s; @R; endsequence \
         assert property (@(posedge c) s); endmodule",
    ),
    (
        "constraint body",
        "class C; rand logic [99:0] a; constraint k { @R; } endclass module m; endmodule",
    ),
    (
        "compound",
        "module m; logic [99:0] a; logic b; initial b |= @R; endmodule",
    ),
    (
        "ternary tail",
        "module m; logic [99:0] a; logic b, c, d; initial b = c ? d : @R; endmodule",
    ),
    // premise judge L5: the other four sites' tails
    (
        "S3 tail",
        "module m; logic [99:0] a; logic b; initial assign b = @R; endmodule",
    ),
    (
        "S4 tail",
        "module m; logic [99:0] a; logic b; initial force b = @R; endmodule",
    ),
    (
        "S7 tail",
        "module m; logic [99:0] a; initial for (logic i = @R; i; i = 0) ; endmodule",
    ),
    (
        "S7 typedef tail",
        "typedef logic w1_t; module m; logic [99:0] a; initial for (w1_t j = @R; j; j = 0) ; \
         endmodule",
    ),
    (
        "S8 tail",
        "module m; logic [99:0] a; logic b; initial for (b = @R; b; b = 0) ; endmodule",
    ),
    // premise judge L6: an inline constraint's `with { … }` (cells2/d09b)
    (
        "inline constraint",
        "class C; rand logic [99:0] a; endclass module m; C o; initial begin o = new; \
         o.randomize() with { @R; }; end endmodule",
    ),
];

#[test]
fn fill_xz_relational_matrix() {
    // Pin (a): the relational `a <= 'z` counts in every spelling at every position — the
    // fill is no assignment's whole right side whatever tokens surround it.
    let mut cases = Vec::new();
    for (pos, tpl) in REL_POSITIONS {
        for (form, rel) in REL_FORMS {
            cases.push((format!("{pos} / {form}"), tpl.replace("@R", rel), F));
        }
    }
    // the row's own cell (premise2_p9_stmt's shape) and the nonblocking tail of r8
    cases.push((
        "p9".into(),
        "module m; logic b1; initial b1 = b1 <= 'x; endmodule".into(),
        F,
    ));
    assert_eq!(cases.len(), 77);
    let wrong = mismatches(&cases);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn fill_xz_whole_right_side_per_site() {
    // Pin (b): one cell per mark site holding only that site's whole-right-side fill: 0.
    // A site whose mark were lost would read 1 (trip-wire W4).
    let cases: Vec<(String, String, u8)> = [
        ("S1", "module m; logic [69:0] q; initial q = 'x; endmodule"),
        (
            "S2",
            "module m; logic [69:0] q; logic c; always @(posedge c) q <= 'z; endmodule",
        ),
        (
            "S3",
            "module m; logic [99:0] b; initial assign b = 'z; endmodule",
        ),
        (
            "S4",
            "module m; logic [69:0] v; initial force v = 'x; endmodule",
        ),
        ("S5", "module m; wire [69:0] w; assign w = 'z; endmodule"),
        (
            "S6 module variable",
            "module m; logic [69:0] v = 'x; endmodule",
        ),
        ("S6 net", "module m; wire [69:0] w = 'z; endmodule"),
        (
            "S6 class property",
            "class C; logic [69:0] p = 'x; endclass module m; endmodule",
        ),
        (
            "S6 rand class property",
            "class C; rand logic [69:0] p = 'x; endclass module m; endmodule",
        ),
        (
            "S6 function local",
            "module m; function automatic void f(); logic [69:0] l = 'x; endfunction endmodule",
        ),
        (
            "S6 block local",
            "module m; initial begin logic [69:0] l = 'z; end endmodule",
        ),
        (
            "S6 automatic block local",
            "module m; typedef logic [69:0] t70; initial begin automatic t70 s = 'z; end \
             endmodule",
        ),
        (
            "S6 typed name",
            "module m; typedef logic [69:0] t70; t70 v = 'x; endmodule",
        ),
        (
            "S6 struct typed",
            "module m; typedef struct packed { logic [69:0] a; } ps_t; ps_t s = 'x; endmodule",
        ),
        (
            "S7",
            "module m; initial for (logic [69:0] lv = 'z; lv != 0; lv = 0) ; endmodule",
        ),
        (
            "S7 typedef",
            "typedef logic [99:0] w_t; module m; int k; initial for (w_t j = 'z; k < 1; \
             k = k + 1) ; endmodule",
        ),
        (
            "S8",
            "module m; logic [69:0] a; initial for (a = 'x; a != 0; a = 0) ; endmodule",
        ),
        // the plan's r13_ctl with the procedural `assign` premise judge L7 adds: 8 sites
        (
            "r13_ctl + S3",
            "module top; logic [99:0] a, b; wire [99:0] w = 'z; logic [99:0] v = 'x; \
             assign b = 'z; initial begin a = 'x; a <= 'z; force a = 'x; assign a = 'x; \
             for (a = 'x; a != 0; a = 0) ; for (logic [69:0] lv = 'z; lv != 0; lv = 0) ; end \
             endmodule",
        ),
        // EH1's one form (mem_lib.sv)
        (
            "EH1",
            "module m; logic [31:0] Q; logic CLK, WE; always @(posedge CLK) begin if (WE) \
             begin end else Q <= 'x; end endmodule",
        ),
    ]
    .into_iter()
    .map(|(l, s)| (l.to_string(), s.to_string(), 0))
    .collect();
    let wrong = mismatches(&cases);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn fill_xz_k1_and_unmarked_sites_count() {
    // Pin (c): main's K1 (RULINGS L1-10) — a parameter / localparam / specparam / defparam
    // value and a `let` body are no mark site, so their plain fill counts (param_xz /
    // let_any carry it too) ; a genvar initialization, a covergroup option, a clocking
    // signal's `= expr` and a UDP's `initial` (premise judge L6) are no mark site either.
    let sub = "module s #(parameter W = 0) (); endmodule ";
    let cases: Vec<(String, String, u8)> = [
        (
            "localparam",
            "module m; localparam L = 'x; endmodule".to_string(),
            P | F,
        ),
        (
            "parameter",
            "module m; parameter Q = 'z; endmodule".to_string(),
            P | F,
        ),
        (
            "specparam",
            "module top; specparam S = 'x; endmodule".to_string(),
            P | F,
        ),
        (
            "defparam",
            format!("{sub}module m; s u(); defparam u.W = 'z; endmodule"),
            P | F,
        ),
        (
            "let body",
            "module m; let LX = 'x; endmodule".to_string(),
            L | F,
        ),
        (
            "genvar initialization",
            "module m; for (genvar g = 'x; g < 2; g = g + 1) begin : b end endmodule".to_string(),
            F,
        ),
        (
            "covergroup option",
            "module top; logic c; covergroup cg @(posedge c); option.per_instance = 'x; \
             endgroup endmodule"
                .to_string(),
            F,
        ),
        (
            "clocking signal",
            "module top; logic c; logic [3:0] x; clocking cb @(posedge c); output o = 'x; \
             endclocking endmodule"
                .to_string(),
            F,
        ),
        (
            "UDP initial",
            "primitive p(q, a, b); output q; reg q; input a, b; initial q = 'x; table \
             0 ? : ? : 0; 1 ? : ? : 1; endtable endprimitive\nmodule top; wire y; reg a, b; \
             p u(y, a, b); endmodule"
                .to_string(),
            F,
        ),
        (
            "inline constraint",
            "class C; rand logic [99:0] a; endclass module top; C o; initial begin o = new; \
             o.randomize() with { a <= 'z; }; end endmodule"
                .to_string(),
            F,
        ),
    ]
    .into_iter()
    .map(|(l, s, w)| (l.to_string(), s, w))
    .collect();
    let wrong = mismatches(&cases);
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn fill_xz_unchanged_over_reports() {
    // Pin (d): marked, but the shape fails, so the fill counts as it did before U4 — the
    // intra-assignment delay (`#1` before the fill), a parenthesized fill, a declarator
    // that ends at `,` (an over-report each, stated).
    for src in [
        "module m; logic [99:0] a; initial a = #1 'x; endmodule",
        "module m; logic [99:0] a; initial a <= ('z); endmodule",
        "module m; logic [99:0] a = 'x, b; endmodule",
    ] {
        assert_eq!(bits(src), F, "`{src}`");
    }
}

/// A parser that has parsed `src` (a design), and the token index of `src`'s first fill.
fn parsed_parser<'t, 's>(toks: &'t [Spanned], src: &'s str) -> Parser<'t, 's> {
    let mut p = Parser::new(toks, src);
    let _ = p.parse_source_unit();
    assert!(p.errors.is_empty(), "`{src}` must parse: {:?}", p.errors);
    p
}

#[test]
fn rewind_pops_the_marks_at_and_past_its_position() {
    // Pin (f) with premise judge L3: the mark sits at the fill's index m ; `rewind(m)` pops
    // it, `rewind(m + 1)` keeps it (an off-by-one `> save` keeps it at m: fx_m11), and
    // `rewind(0)` pops everything (a rewind that pops nothing: fx_m10).
    let src = "module m; logic a; initial a = 'x; endmodule";
    let (toks, _) = hdl_lexer::lex(src);
    let m = toks
        .iter()
        .position(|t| t.kind == TokenKind::IntUnsizedBasedXz)
        .unwrap();
    let mut p = parsed_parser(&toks, src);
    assert_eq!(p.lossy_whole_rhs, [m]);
    p.rewind(m + 1);
    assert_eq!(p.lossy_whole_rhs, [m], "rewind(m + 1) keeps the mark at m");
    assert_eq!(p.pos, m + 1);
    p.rewind(m);
    assert!(p.lossy_whole_rhs.is_empty(), "rewind(m) pops the mark at m");
    let mut p = parsed_parser(&toks, src);
    p.rewind(0);
    assert!(p.lossy_whole_rhs.is_empty(), "rewind(0) pops every mark");
    assert_eq!(p.pos, 0);
}

#[test]
fn rewind_pops_the_inside_keywords_too() {
    // RULINGS L1-19: the one funnel truncates `inside_kw_at` the same way.
    let src = "module m; logic [3:0] a; logic b; initial b = a inside {1, 2}; endmodule";
    let (toks, _) = hdl_lexer::lex(src);
    let k = toks
        .iter()
        .position(|t| &src[t.span.clone()] == "inside")
        .unwrap();
    let mut p = parsed_parser(&toks, src);
    assert_eq!(p.inside_kw_at, [k]);
    p.rewind(k + 1);
    assert_eq!(p.inside_kw_at, [k]);
    p.rewind(k);
    assert!(p.inside_kw_at.is_empty());
}

#[test]
fn marks_are_only_unsized_xz_right_side_starts() {
    // Pin (f), second half: a right side that starts at any other token is not marked
    // (the helper's kind test: fx_m12) ; every mark is an `IntUnsizedBasedXz` token, at a
    // site's right side, in ascending order. Plain assignments with no fill leave it empty.
    let plain = "module m; logic [7:0] a, b; wire [7:0] w = 8'd1; assign b = a; initial begin                  a = 1; a <= b; a = 4'bx; a = '1; force a = b; assign a = b;                  for (a = 0; a < 2; a = a + 1) ; end endmodule";
    let (toks, _) = hdl_lexer::lex(plain);
    assert!(parsed_parser(&toks, plain).lossy_whole_rhs.is_empty());
    let src = "module m; logic [69:0] a, b; logic [69:0] v = 'x; initial begin a = 'z; \
               b = 'bx; a = b | 'x; a = {'x}; a <= 'Z; end endmodule";
    let (toks, _) = hdl_lexer::lex(src);
    let p = parsed_parser(&toks, src);
    let marked: Vec<&str> = p
        .lossy_whole_rhs
        .iter()
        .map(|&i| {
            assert_eq!(toks[i].kind, TokenKind::IntUnsizedBasedXz);
            &src[toks[i].span.clone()]
        })
        .collect();
    // the initializer's `'x`, `'z`, `'bx` (an unsized based literal: marked, never read as
    // a fill), `'Z` ; `b | 'x` starts at `b` and `{'x}` at `{`: no mark.
    assert_eq!(marked, ["'x", "'z", "'bx", "'Z"]);
    assert!(p.lossy_whole_rhs.windows(2).all(|w| w[0] < w[1]));
}

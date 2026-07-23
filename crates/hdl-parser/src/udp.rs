//! UDP declarations — split out of the original `hdl-parser` lib.rs (mechanical move).

use super::*;

impl Parser<'_, '_> {
    /// Combinational User-Defined Primitive (IEEE 1364 §29), DESUGARED in the parser
    /// (mirroring [`parse_gate_primitive`]) into a synthetic ordinary [`ModuleDecl`].
    /// The module body is one `always @(*)` whose if/else-if cascade realizes the
    /// truth table with iverilog-faithful semantics. (1) Each input column is matched
    /// 4-state-EXACT (`===`), NOT `casez` — a `0`/`1`/`x` column matches only that
    /// value; `?` matches anything (incl. z); `b` matches 0 or 1 only (never x/z).
    /// casez would wildcard the SCRUTINEE's x/z and silently mis-match (the cardinal
    /// trap). (2) Conflicting rows resolve order-INDEPENDENTLY by priority 0 > 1 > x:
    /// `if (any-0-row) o=0; else if (any-1-row) o=1; else o=x;`. (3) Any combination
    /// matched by no row (and every `x`-output row) → x (the trailing `else`).
    ///
    /// Honest-loud rejects (combinational only — sequential UDPs are slice #9):
    /// `reg` output / a second `:` (sequential current-state form) / edge & `z`
    /// symbols / `z`/`-` outputs / multi-output / wrong column count.
    pub(crate) fn parse_udp_decl(&mut self) -> Option<ModuleDecl> {
        // The UDP table symbol kinds live at module scope (so the row-scanner helper
        // methods can name them); alias to short local names for the body.
        use UdpEnd as EndSym;
        use UdpInCol as InCol;
        use UdpLevSym as LevSym;
        use UdpNextSym as NextSym;
        use UdpOutSym as OutSym;
        use UdpStateSym as StateSym;
        let start = self.cur_span();
        self.bump(); // `primitive`
        let name = self.ident()?;
        // ── header port list: ( out, in0, in1, … ) ;  (positional names only) ──
        self.expect(TokenKind::LParen, "'(' after primitive name");
        let mut port_names: Vec<Ident> = Vec::new();
        loop {
            let before = self.pos;
            if let Some(id) = self.ident() {
                port_names.push(id);
            }
            if self.pos == before {
                self.bump(); // forward-progress guard
            }
            if !self.eat(TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RParen, "')' after primitive ports");
        self.expect(TokenKind::Semi, "';' after primitive header");
        if port_names.len() < 2 {
            self.error_at(
                name.span,
                "a UDP with one output and at least one input port",
            );
            return None;
        }
        // ── direction declarations: `output [reg] OUT;`, `input a, b;`, plus the
        //    sequential-only `initial OUT = 1'bN;` — in any order before `table`. ──
        let mut out_name: Option<Ident> = None;
        let mut in_names: Vec<Ident> = Vec::new();
        // `output reg` ⇒ sequential UDP.
        let mut seq = false;
        // Whether the output was actually declared `reg` (via `output reg` or a
        // standalone `reg OUT;`). A sequential UDP table REQUIRES a reg output
        // (IEEE §29.7); a plain `wire` output with sequential rows is internally
        // inconsistent and must be loud-rejected (iverilog asserts/aborts).
        let mut out_is_reg = false;
        // sequential power-on value from `initial OUT = …;` (None ⇒ x).
        let mut initial_val: Option<char> = None;
        let mut saw_initial = false;
        while !self.at_kw(Kw::Table) && !self.at_eof() {
            if self.at_kw(Kw::Output) {
                self.bump();
                if self.at_kw(Kw::Reg) {
                    // `output reg` is the canonical sequential-UDP marker.
                    self.bump();
                    seq = true;
                    out_is_reg = true;
                }
                // Honest-loud: a VECTOR output (`output reg [N:0] o;`) is out of
                // scope — UDP outputs are scalar 1-bit per IEEE §29.
                if matches!(self.peek(), Some(TokenKind::LBracket)) {
                    self.error("a scalar (1-bit) UDP output (vector output is unsupported)");
                    return None;
                }
                loop {
                    let before = self.pos;
                    if let Some(id) = self.ident() {
                        if out_name.is_some() {
                            self.error_at(id.span, "exactly one UDP output port");
                            return None;
                        }
                        out_name = Some(id);
                    }
                    if self.pos == before {
                        self.bump();
                    }
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Semi, "';' after UDP output declaration");
            } else if self.at_kw(Kw::Input) {
                self.bump();
                loop {
                    let before = self.pos;
                    if let Some(id) = self.ident() {
                        in_names.push(id);
                    }
                    if self.pos == before {
                        self.bump();
                    }
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Semi, "';' after UDP input declaration");
            } else if self.at_kw(Kw::Reg) {
                // `reg OUT;` separate from the `output` decl (V2005 UDP form) ⇒
                // sequential. The named reg must be the output; we just note `seq`.
                self.bump();
                seq = true;
                out_is_reg = true;
                loop {
                    let before = self.pos;
                    let _ = self.ident();
                    if self.pos == before {
                        self.bump();
                    }
                    if !self.eat(TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::Semi, "';' after UDP reg declaration");
            } else if self.at_kw(Kw::Initial) {
                // `initial OUT = 1'bN;` power-on state (sequential-only ⇒ seq).
                self.bump();
                seq = true;
                saw_initial = true;
                let _ = self.ident(); // OUT (positional; value goes to the state reg)
                self.expect(TokenKind::Eq, "'=' in a UDP initial statement");
                // Collect the full literal text up to ';' (handles multi-token forms
                // like `1'b0`/`1 'b z`), then the bit VALUE is its LAST non-space char.
                // A 1-bit power-on must be exactly 0/1/x — `z` (or anything else) is an
                // illegal power-on value → loud-reject (NOT silently coerced to 1).
                let mut lit = String::new();
                while !matches!(self.peek(), Some(TokenKind::Semi)) && !self.at_eof() {
                    lit.push_str(self.cur_text());
                    self.bump();
                }
                self.expect(TokenKind::Semi, "';' after UDP initial statement");
                let v = lit
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .next_back()
                    .map(|c| c.to_ascii_lowercase());
                match v {
                    Some(c @ ('0' | '1' | 'x')) => initial_val = Some(c),
                    _ => {
                        self.error_at(
                            name.span,
                            "a UDP initial value of 0, 1, or x (1'b0 / 1'b1 / 1'bx)",
                        );
                        return None;
                    }
                }
            } else {
                self.error("'output', 'input', 'reg', 'initial', or 'table' in a UDP body");
                return None;
            }
        }
        let out_name = match out_name {
            Some(o) => o,
            None => {
                self.error_at(name.span, "a UDP 'output' declaration");
                return None;
            }
        };
        // output must be the FIRST port; the rest (in port-list order) are inputs.
        if out_name.name != port_names[0].name {
            self.error_at(out_name.span, "the UDP output to be the first port");
            return None;
        }
        let ordered_inputs: Vec<Ident> = port_names[1..].to_vec();
        {
            // every non-first port must be declared `input`, and vice versa.
            let decl_in: std::collections::BTreeSet<&str> =
                in_names.iter().map(|i| i.name.as_str()).collect();
            let port_in: std::collections::BTreeSet<&str> =
                ordered_inputs.iter().map(|i| i.name.as_str()).collect();
            if decl_in != port_in {
                self.error_at(
                    name.span,
                    "every UDP input port to be declared `input` (and vice versa)",
                );
                return None;
            }
        }
        let n_in = ordered_inputs.len();
        // ── table … endtable ──
        if !self.at_kw(Kw::Table) {
            self.error("'table' in a UDP body");
            return None;
        }
        self.bump(); // `table`
                     // A combinational row: (level cols, out). A sequential row carries the
                     // current-state column and a next-state symbol, and each input column may be
                     // a level or an edge.
        struct SeqRow {
            cols: Vec<InCol>,
            state: StateSym,
            next: NextSym,
            is_edge: bool, // exactly one column is an edge
        }
        let mut comb_rows: Vec<(Vec<LevSym>, OutSym)> = Vec::new();
        let mut seq_rows: Vec<SeqRow> = Vec::new();
        // A two-colon row (`inputs : state : next`) is itself a sequential signal even
        // when the state column is `?` and there is no edge/`-`.
        let mut has_two_colon = false;
        // `?` endpoint inside an edge is legal; the level-table `b` is NOT a legal
        // edge endpoint and must loud-reject.
        while !self.at_kw(Kw::Endtable) && !self.at_eof() {
            let row_start = self.cur_span();
            // Collect the three fields as raw text. Each table symbol is a single
            // char (`0 1 x ? b - r f p n *`) or a self-delimiting paren group
            // `(vw)`; joining each token's source text and char-scanning is therefore
            // unambiguous (whitespace is irrelevant). We split on the literal `:`
            // tokens so a parenthesised pair is never mistaken for a colon.
            let mut fields: Vec<String> = vec![String::new()];
            loop {
                if self.at_eof() || self.at_kw(Kw::Endtable) {
                    self.error("';' to end a UDP table row");
                    return None;
                }
                match self.peek() {
                    Some(TokenKind::Semi) => {
                        self.bump();
                        break;
                    }
                    Some(TokenKind::Colon) => {
                        self.bump();
                        if fields.len() >= 3 {
                            self.error_at(row_start, "at most two colons in a UDP table row");
                            return None;
                        }
                        fields.push(String::new());
                    }
                    _ => {
                        let last = fields.last_mut().unwrap();
                        last.push_str(self.cur_text());
                        self.bump();
                    }
                }
            }
            if fields.len() == 2 {
                // ── single-colon row: combinational OR sequential level/edge row
                //    (output is the next-state column; no separate state column). ──
                let cols = match Self::scan_udp_input_cols(&fields[0], n_in) {
                    Ok(c) => c,
                    Err(msg) => {
                        self.error_at(row_start, msg);
                        return None;
                    }
                };
                let has_edge = cols.iter().any(|c| matches!(c, InCol::Edge(_)));
                if has_edge {
                    // an edge column makes this a sequential row (single-colon form:
                    // state column omitted ⇒ `?`).
                    seq = true;
                    let n_edge = cols.iter().filter(|c| matches!(c, InCol::Edge(_))).count();
                    if n_edge > 1 {
                        // Honest-loud (correct-or-loud-stricter): IEEE §29 forbids >1
                        // edge per row; iverilog accepts-but-never-fires it.
                        self.error_at(row_start, "at most one edge column per UDP table row");
                        return None;
                    }
                    let next = match Self::scan_udp_next(&fields[1]) {
                        Ok(n) => n,
                        Err(msg) => {
                            self.error_at(row_start, msg);
                            return None;
                        }
                    };
                    seq_rows.push(SeqRow {
                        cols,
                        state: StateSym::Q,
                        next,
                        is_edge: true,
                    });
                } else {
                    // No edge → could be a combinational row OR a sequential LEVEL row
                    // (single-colon level form). Stash as a level SeqRow; also keep a
                    // comb projection for the combinational fallback (used only if the
                    // whole table turns out to be combinational). A `-` next is
                    // sequential-only ⇒ it forces `seq` and has NO comb projection.
                    let next = match Self::scan_udp_next(&fields[1]) {
                        Ok(n) => n,
                        Err(msg) => {
                            self.error_at(row_start, msg);
                            return None;
                        }
                    };
                    let levs: Vec<LevSym> = cols
                        .into_iter()
                        .map(|c| match c {
                            InCol::Lev(l) => l,
                            InCol::Edge(_) => unreachable!(),
                        })
                        .collect();
                    match next {
                        NextSym::Hold => {
                            // `-` ⇒ sequential; no comb projection.
                            seq = true;
                        }
                        NextSym::Zero => comb_rows.push((levs.clone(), OutSym::Zero)),
                        NextSym::One => comb_rows.push((levs.clone(), OutSym::One)),
                        NextSym::X => comb_rows.push((levs.clone(), OutSym::X)),
                    }
                    seq_rows.push(SeqRow {
                        cols: levs.into_iter().map(InCol::Lev).collect(),
                        state: StateSym::Q,
                        next,
                        is_edge: false,
                    });
                }
            } else {
                // ── two-colon row: SEQUENTIAL  inputs : state : next ──
                seq = true;
                has_two_colon = true;
                let cols = match Self::scan_udp_input_cols(&fields[0], n_in) {
                    Ok(c) => c,
                    Err(msg) => {
                        self.error_at(row_start, msg);
                        return None;
                    }
                };
                let n_edge = cols.iter().filter(|c| matches!(c, InCol::Edge(_))).count();
                if n_edge > 1 {
                    self.error_at(row_start, "at most one edge column per UDP table row");
                    return None;
                }
                let state = match fields[1]
                    .chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<Vec<_>>()
                    .as_slice()
                {
                    ['0'] => StateSym::Zero,
                    ['1'] => StateSym::One,
                    ['x'] | ['X'] => StateSym::X,
                    ['?'] => StateSym::Q,
                    _ => {
                        self.error_at(row_start, "a UDP state symbol (0 1 x ?)");
                        return None;
                    }
                };
                let next = match Self::scan_udp_next(&fields[2]) {
                    Ok(n) => n,
                    Err(msg) => {
                        self.error_at(row_start, msg);
                        return None;
                    }
                };
                seq_rows.push(SeqRow {
                    cols,
                    state,
                    next,
                    is_edge: n_edge == 1,
                });
            }
        }
        if !self.at_kw(Kw::Endtable) {
            self.error("'endtable'");
            return None;
        }
        self.bump(); // `endtable`
        if !self.at_kw(Kw::Endprimitive) {
            self.error("'endprimitive'");
            return None;
        }
        self.bump(); // `endprimitive`
                     // Optional `: name` end-label (IEEE 1800 §29.3), same accept-and-ignore
                     // policy as the container ends in `parse_module_like`.
        self.opt_block_label();
        if seq_rows.is_empty() {
            // An empty `table … endtable` is an illegal UDP form (iverilog: "Empty
            // UDP table") — loud-reject rather than silently synthesize an always-x
            // primitive.
            self.error_at(name.span, "a non-empty UDP table");
            return None;
        }
        // ── internal-consistency: a sequential marker (`output reg` / two-colon /
        //    edge / `initial`) must be matched by a sequential table, and a purely
        //    combinational table must NOT carry sequential-only markers. ──
        if !seq {
            // Combinational UDP: every row must be single-colon, no edge, next ∈ 0/1/x.
            let span = start.to(self.prev_span());
            return self.build_comb_udp(span, name, port_names, &ordered_inputs, &comb_rows);
        }
        // Sequential consistency: every state column drawn from a single-colon row is
        // `?`; that is fine. But a `wire` (non-reg) output mixed with a two-colon row,
        // or a `reg`/`initial` marker with an all-combinational single-colon table
        // (no edges, no `-`, no state column), is internally inconsistent.
        let has_edge = seq_rows.iter().any(|r| r.is_edge);
        let has_hold = seq_rows.iter().any(|r| matches!(r.next, NextSym::Hold));
        let has_state = seq_rows.iter().any(|r| !matches!(r.state, StateSym::Q));
        if !out_is_reg {
            // A `wire` (non-reg) output with a SEQUENTIAL table (two-colon rows /
            // edge column / `-` next-state / state column) is internally
            // inconsistent: a sequential UDP requires a `reg` output (IEEE §29.7).
            // iverilog aborts on this; vita loud-rejects (correct-or-loud). NB
            // `initial OUT=…;` alone is NOT a `reg` declaration — it also requires
            // the output be declared `reg`, so this guard covers it too.
            self.error_at(
                name.span,
                "a `reg` output (`output reg …;`) for a sequential UDP table",
            );
            return None;
        }
        if !has_edge && !has_hold && !has_state && !has_two_colon {
            // A `reg`/`initial` marker but a table with no sequential content at all.
            self.error_at(
                name.span,
                "a sequential UDP table (with an edge column, a '-' next-state, or a state column) to match the 'reg'/'initial' marker",
            );
            return None;
        }
        let _ = saw_initial; // (only sets the t0 value; presence already set `seq`)

        // ── desugar: build the synthetic SEQUENTIAL module (literal §29 state-table
        //    evaluator: level rows first, then edge rows, no-match→x, '-'=hold). ──
        let span = start.to(self.prev_span());
        let mk_lit = |raw: &str| Expr {
            span,
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: raw.to_string(),
            },
        };
        let mk_ident = |id: &Ident| Expr {
            span,
            kind: ExprKind::Ident(HierPath {
                segments: vec![id.clone()],
                span,
            }),
        };
        let mk_bin = |op: BinOp, a: Expr, b: Expr| Expr {
            span,
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(a),
                rhs: Box::new(b),
            },
        };
        // case-equality of an expr against a 4-state literal raw (`1'b0/1/x/z`).
        let case_eq = |raw: &str, lhs: Expr| Expr {
            span,
            kind: ExprKind::Binary {
                op: BinOp::CaseEq,
                lhs: Box::new(lhs),
                rhs: Box::new(Expr {
                    span,
                    kind: ExprKind::IntLit {
                        kind: IntLitKind::Sized,
                        raw: raw.to_string(),
                    },
                }),
            },
        };
        // 4-state-exact level match of expr `e` against a LevSym/EndSym kind, with
        // z→x folding for the `x` symbol (matches BOTH x and z — IEEE §29.3.4).
        let match_zero = |e: Expr| case_eq("1'b0", e);
        let match_one = |e: Expr| case_eq("1'b1", e);
        let match_x = |e: Expr| {
            // x OR z
            mk_bin(BinOp::LogOr, case_eq("1'bx", e.clone()), case_eq("1'bz", e))
        };
        let match_b =
            |e: Expr| mk_bin(BinOp::LogOr, case_eq("1'b0", e.clone()), case_eq("1'b1", e));
        // state name + shadow regs.
        let out_id = &port_names[0];
        let shadow_name = |k: usize| Ident {
            name: format!("__p_udp_{}_{}", out_id.name, k),
            span,
        };
        // condition that input `k` LEVEL-matches symbol `s` (against CURRENT value).
        let lev_cond = |k: usize, s: LevSym| -> Option<Expr> {
            let e = mk_ident(&ordered_inputs[k]);
            match s {
                LevSym::Zero => Some(match_zero(e)),
                LevSym::One => Some(match_one(e)),
                LevSym::X => Some(match_x(e)),
                LevSym::Q => None, // `?` ⇒ no constraint
                LevSym::B => Some(match_b(e)),
            }
        };
        // condition for the state column matching stored `out`.
        let state_cond = |s: StateSym| -> Option<Expr> {
            let e = mk_ident(out_id);
            match s {
                StateSym::Zero => Some(match_zero(e)),
                StateSym::One => Some(match_one(e)),
                StateSym::X => Some(match_x(e)),
                StateSym::Q => None,
            }
        };
        // endpoint case-eq against current value `e` (z→x for `x`; `?` ⇒ None).
        let end_cond = |e: Expr, ep: EndSym| -> Option<Expr> {
            match ep {
                EndSym::Zero => Some(match_zero(e)),
                EndSym::One => Some(match_one(e)),
                EndSym::X => Some(match_x(e)),
                EndSym::Q => None,
            }
        };
        // FOLDED-change guard for input `k`: an edge exists iff prev/cur differ AFTER
        // z→x folding (IEEE §29.3.4). A pure z↔x swap (both fold to x) is NOT a change,
        // so it must neither fire an edge NOR re-evaluate the table (the output holds).
        // Encoded without a NOT operator:
        //   (prev !== cur) && ( is012(prev) || is012(cur) )
        // where is012(e) = (e===1'b0) || (e===1'b1).
        let folded_changed = |k: usize| -> Expr {
            let cur = mk_ident(&ordered_inputs[k]);
            let prev = mk_ident(&shadow_name(k));
            let is012 =
                |e: Expr| mk_bin(BinOp::LogOr, case_eq("1'b0", e.clone()), case_eq("1'b1", e));
            mk_bin(
                BinOp::LogAnd,
                mk_bin(BinOp::CaseNe, prev.clone(), cur.clone()),
                mk_bin(BinOp::LogOr, is012(prev), is012(cur)),
            )
        };
        // Build the full match condition for one row. `current` reads inputs;
        // edges also read the shadow regs.
        let and_fold = |conds: Vec<Expr>| -> Option<Expr> {
            conds.into_iter().reduce(|a, b| mk_bin(BinOp::LogAnd, a, b))
        };
        let or_fold = |conds: Vec<Expr>| -> Option<Expr> {
            conds.into_iter().reduce(|a, b| mk_bin(BinOp::LogOr, a, b))
        };
        let row_match = |row: &SeqRow| -> Expr {
            let mut conds: Vec<Expr> = Vec::new();
            for (k, col) in row.cols.iter().enumerate() {
                match col {
                    InCol::Lev(l) => {
                        if let Some(c) = lev_cond(k, *l) {
                            conds.push(c);
                        }
                    }
                    InCol::Edge(pairs) => {
                        // edge column k: folded_changed(k) AND OR_over_pairs[
                        //   CaseEq(__p_ik,from) && CaseEq(ik,to) ]  (?-endpoint drops).
                        let cur = mk_ident(&ordered_inputs[k]);
                        let prev = mk_ident(&shadow_name(k));
                        let changed = folded_changed(k);
                        let mut pair_terms: Vec<Expr> = Vec::new();
                        for (from, to) in pairs {
                            let mut t: Vec<Expr> = Vec::new();
                            if let Some(c) = end_cond(prev.clone(), *from) {
                                t.push(c);
                            }
                            if let Some(c) = end_cond(cur.clone(), *to) {
                                t.push(c);
                            }
                            // both endpoints `?` ⇒ any-change (just the guard).
                            match and_fold(t) {
                                Some(e) => pair_terms.push(e),
                                None => pair_terms.push(mk_lit("1'b1")),
                            }
                        }
                        let any_pair = or_fold(pair_terms).unwrap_or(mk_lit("1'b1"));
                        conds.push(mk_bin(BinOp::LogAnd, changed, any_pair));
                    }
                }
            }
            if let Some(c) = state_cond(row.state) {
                conds.push(c);
            }
            and_fold(conds).unwrap_or(mk_lit("1'b1"))
        };
        // Partition rows into LEVEL group and EDGE group, then bucket by next-value
        // (0/1/x/hold) — within a group, all rows producing the same next are
        // OR-folded; cross-group priority is fixed (level beats edge).
        let out_lval = Lvalue::Ident(HierPath {
            segments: vec![out_id.clone()],
            span,
        });
        let assign_lit = |raw: &str| Stmt::Blocking {
            lhs: out_lval.clone(),
            delay: None,
            event: None,
            rhs: mk_lit(raw),
            span,
        };
        let hold_stmt = || Stmt::Block {
            label: None,
            decls: Vec::new(),
            stmts: Vec::new(), // EMPTY then-branch that OWNS its else-slot (holds out)
            span,
        };
        // Build one priority cascade for a row group. Returns the innermost-else
        // chain WITHOUT the final no-match else (caller appends it).
        let build_group = |rows: &[&SeqRow], inner: Stmt| -> Stmt {
            let mut zero: Vec<Expr> = Vec::new();
            let mut one: Vec<Expr> = Vec::new();
            let mut xv: Vec<Expr> = Vec::new();
            let mut hold: Vec<Expr> = Vec::new();
            for r in rows {
                let m = row_match(r);
                match r.next {
                    NextSym::Zero => zero.push(m),
                    NextSym::One => one.push(m),
                    NextSym::X => xv.push(m),
                    NextSym::Hold => hold.push(m),
                }
            }
            let mut node = inner;
            // Build from lowest to highest priority so the final wrapping gives
            // 0 > 1 > x > hold.
            if let Some(c) = or_fold(hold) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(hold_stmt()),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            if let Some(c) = or_fold(xv) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(assign_lit("1'bx")),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            if let Some(c) = or_fold(one) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(assign_lit("1'b1")),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            if let Some(c) = or_fold(zero) {
                node = Stmt::If {
                    cond: c,
                    then_s: Box::new(assign_lit("1'b0")),
                    else_s: Some(Box::new(node)),
                    span,
                };
            }
            node
        };
        let level_rows: Vec<&SeqRow> = seq_rows.iter().filter(|r| !r.is_edge).collect();
        let edge_rows: Vec<&SeqRow> = seq_rows.iter().filter(|r| r.is_edge).collect();
        // No-match → x (NEVER hold). Then edge group reached only when no level row
        // matched, then level group on top.
        let nomatch = assign_lit("1'bx");
        let edge_cascade = build_group(&edge_rows, nomatch);
        let full_cascade = build_group(&level_rows, edge_cascade);
        // GLOBAL folded-change guard: the table is RE-EVALUATED only when at least one
        // input's FOLDED (z→x) value changed. A wake caused solely by a z↔x swap on
        // some input (no folded change) holds the output — matching iverilog, which
        // does not re-evaluate a UDP on a z↔x-only transition (a no-match else would
        // otherwise clobber a previously-matched output to x → silent-wrong). With ≥1
        // input, fold each per-input guard with OR; the always already only wakes on
        // an input change, so `any_folded_change` is the only gate needed.
        let any_folded_change = (0..n_in)
            .map(folded_changed)
            .reduce(|a, b| mk_bin(BinOp::LogOr, a, b))
            .unwrap_or(mk_lit("1'b1"));
        let guarded_cascade = Stmt::If {
            cond: any_folded_change,
            then_s: Box::new(full_cascade),
            else_s: None, // no folded change ⇒ hold the output (empty else)
            span,
        };
        // Re-snapshot shadows LAST (blocking), after all reads — UNCONDITIONAL so the
        // shadow always tracks the latest raw input (folded matching is z/x-agnostic).
        let mut body_stmts: Vec<Stmt> = vec![guarded_cascade];
        for (k, _inp) in ordered_inputs.iter().enumerate() {
            body_stmts.push(Stmt::Blocking {
                lhs: Lvalue::Ident(HierPath {
                    segments: vec![shadow_name(k)],
                    span,
                }),
                delay: None,
                event: None,
                rhs: mk_ident(&ordered_inputs[k]),
                span,
            });
        }
        // Sensitivity: ALL inputs as plain NoEdge terms (→ AnyEdge wake = finest).
        // Shadow regs and `out` are NOT in the list.
        let sens: Vec<EventExpr> = ordered_inputs
            .iter()
            .map(|inp| EventExpr {
                edge: Edge::NoEdge,
                expr: mk_ident(inp),
                iff: None,
                span,
            })
            .collect();
        let always = ProceduralBlock {
            kind: ProcKind::Always,
            sensitivity: Some(Sensitivity::List(sens)),
            body: Box::new(Stmt::Block {
                label: None,
                decls: Vec::new(),
                stmts: body_stmts,
                span,
            }),
            span,
        };
        // ── module body: the output port is a `reg` (single declaration, like the
        //    comb path); shadow regs are body regs; t0 power-on state is seeded by a
        //    synthetic `initial out = <init>;`. Shadow regs are NOT initialised — they
        //    power on as x (4-state reg default), so the first real input transition
        //    classifies as `x → value` through the table (exact t0-settling replay). ──
        let init_raw = match initial_val {
            Some('0') => "1'b0",
            Some('1') => "1'b1",
            _ => "1'bx",
        };
        let init_block = ProceduralBlock {
            kind: ProcKind::Initial,
            sensitivity: None,
            body: Box::new(Stmt::Blocking {
                lhs: out_lval.clone(),
                delay: None,
                event: None,
                rhs: mk_lit(init_raw),
                span,
            }),
            span,
        };
        // shadow regs (no init — power on x; seeded on the first input change).
        let mut body: Vec<ModuleItem> = Vec::new();
        for (k, _inp) in ordered_inputs.iter().enumerate() {
            body.push(ModuleItem::NetVar(NetVarDecl {
                kind: NetVarKind::Reg,
                signed: false,
                range: None,
                packed: Vec::new(),
                delay: None,
                names: vec![DeclName {
                    name: shadow_name(k),
                    unpacked: Vec::new(),
                    init: None,
                    span,
                }],
                lifetime: None,
                class_type: None,
                class_args: Vec::new(),
                const_param: false,
                span,
            }));
        }
        body.push(ModuleItem::Proc(init_block));
        body.push(ModuleItem::Proc(always));
        // ── ports: output (reg, procedurally driven) first, then input wires ──
        let mut ports: Vec<AnsiPort> = Vec::with_capacity(port_names.len());
        ports.push(AnsiPort {
            dir: PortDir::Output,
            net_or_var: Some(NetVarKind::Reg),
            signed: false,
            range: None,
            packed: Vec::new(),
            name: out_id.clone(),
            default: None,
            iface: None,
            span,
        });
        for inp in &ordered_inputs {
            ports.push(AnsiPort {
                dir: PortDir::Input,
                net_or_var: None, // default wire
                signed: false,
                range: None,
                packed: Vec::new(),
                name: inp.clone(),
                default: None,
                iface: None,
                span,
            });
        }
        Some(ModuleDecl {
            is_macromodule: false,
            name,
            params: Vec::new(),
            ports: PortList::Ansi(ports),
            body,
            span,
        })
    }
}
